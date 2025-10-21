use std::collections::HashMap;
use std::fs::File;
use std::fs;
use std::path::{Path, PathBuf};

use daemonize::Daemonize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::config::{DAEMON_ERR_PATH, DAEMON_OUT_PATH, DAEMON_PID_PATH, DAEMON_SOCKET_PATH};
use crate::network::Server;

pub fn start_daemon<F, Fut>(callback: F)
where
    F: FnOnce(mpsc::Receiver<DaemonMessage>) -> Fut + 'static,
    Fut: std::future::Future<Output = ()> + 'static,
{
    let stdout = File::create(DAEMON_OUT_PATH).unwrap();
    let stderr = File::create(DAEMON_ERR_PATH).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(DAEMON_PID_PATH)
        .chown_pid_file(true)
        .working_directory("/")
        .stdout(stdout)
        .stderr(stderr);

    match daemonize.start() {
        Ok(_) => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let (tx, rx) = mpsc::channel::<DaemonMessage>(32);
                    tokio::spawn(start_listener(tx.clone()));
                    callback(rx).await;
                });
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

// user sends it to daemon
#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Add { path: String, name: Option<String> },
    Delete { name: String },
    List,
}

// response from daemon
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok(String),
    Err(String),
    List(HashMap<String, String>), // TODO: data
}

// oneshot message from daemon to server
pub struct DaemonMessage {
    pub cmd: Command,
    pub resp_tx: oneshot::Sender<Response>,
}

pub async fn send_command(cmd: Command) -> anyhow::Result<Response> {
    let mut stream = UnixStream::connect(DAEMON_SOCKET_PATH).await?;

    // sends command
    let encoded: Vec<u8> = bincode::serialize(&cmd)?;
    stream.write(&encoded).await?;

    // response
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await?;
    let resp: Response = bincode::deserialize(&buf[..n])?;

    Ok(resp)
}

async fn start_listener(tx: mpsc::Sender<DaemonMessage>) {
    if Path::new(DAEMON_SOCKET_PATH).exists() {
        let _ = fs::remove_file(DAEMON_SOCKET_PATH);
    }

    let listener = match UnixListener::bind(DAEMON_SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind Unix socket: {e}");
            return;
        }
    };

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to accept connection: {e}");
                continue;
            }
        };
        let tx = tx.clone();

        tokio::spawn(async move {
            let mut buf = Vec::new();
            match socket.read_to_end(&mut buf).await {
                Ok(n) if n > 0 => {
                    match bincode::deserialize::<Command>(&buf) {
                        Ok(cmd) => {
                            let (resp_tx, resp_rx) = oneshot::channel();
                            let msg = DaemonMessage { cmd, resp_tx };

                            if tx.send(msg).await.is_err() {
                                let _ = socket.write_all(
                                    &bincode::serialize(&Response::Err("Daemon not running".into())).unwrap()
                                ).await;
                                return;
                            }

                            match resp_rx.await {
                                Ok(resp) => {
                                    if let Err(e) = socket.write_all(&bincode::serialize(&resp).unwrap()).await {
                                        eprintln!("Failed to write response: {e}");
                                    }
                                }
                                Err(_) => {
                                    let _ = socket.write_all(
                                        &bincode::serialize(&Response::Err("Daemon failed to respond".into())).unwrap()
                                    ).await;
                                }
                            }
                        }
                        Err(e) => {
                            let resp = Response::Err(format!("Invalid command: {e}"));
                            if let Err(e) = socket.write_all(&bincode::serialize(&resp).unwrap()).await {
                                eprintln!("Failed to write error response: {e}");
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => eprintln!("Failed to read from socket: {e}"),
            }
        });
    }
}

pub async fn handle_daemon_message(mut rx: mpsc::Receiver<DaemonMessage>, server: Server) {
    while let Some(msg) = rx.recv().await {
        let DaemonMessage { cmd, resp_tx } = msg;
        let resp = match cmd {
            Command::Add { path, name } => {
                // if name is None than take it from path: .../.../test.txt -> test.txt
                let name = name.unwrap_or_else(|| {
                    Path::new(&path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into()
                });
                server.add_file(name, PathBuf::from(path)).await;
                Response::Ok("File added".into())
            }
            Command::Delete { name } => {
                server.remove_file(&name).await;
                Response::Ok(format!("File '{name}' deleted"))
            }
            Command::List => {
                let list = server.list_files().await;
                Response::List(list)
            }
        };

        let _ = resp_tx.send(resp);
    }

}

pub fn stop_daemon() {
    if let Ok(pid_str) = fs::read_to_string(DAEMON_PID_PATH) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            let res = unsafe { libc::kill(pid, libc::SIGTERM) }; // kills the process
            if res == 0 {
                println!("Stopped daemon with PID {pid}");
            } else {
                eprintln!("Failed to kill PID {pid}");
            }
            let _ = fs::remove_file(DAEMON_PID_PATH);
        }
    } else {
        eprintln!("No running daemon found");
    }
}

pub fn handle_response(result: anyhow::Result<Response>) {
    match result {
        Ok(Response::Ok(msg)) => println!("{msg}"),
        Ok(Response::Err(msg)) => eprintln!("{msg}"),
        Ok(Response::List(files)) => {
            for (k, v) in files {
                println!("{k} ({v})");
            }
        }
        Err(e) => eprint!("Error sending command: {e}"),
    }
}