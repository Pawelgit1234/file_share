use std::fs::File;
use std::fs;
use std::path::Path;

use daemonize::Daemonize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use serde::{Deserialize, Serialize};

use crate::config::{DAEMON_ERR_PATH, DAEMON_OUT_PATH, DAEMON_PID_PATH, DAEMON_SOCKET_PATH};

pub fn start_daemon<F, Fut>(callback: F)
where
    F: FnOnce() -> Fut + 'static,
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
                    tokio::spawn(start_listener());
                    callback().await;
                });
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Add { path: String, name: Option<String> },
    Delete { name: String },
    List,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok(String),
    Err(String),
    List(), // TODO: data
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

async fn start_listener() {
    if Path::new(DAEMON_SOCKET_PATH).exists() {
        let _ = std::fs::remove_file(DAEMON_SOCKET_PATH);
    }

    let listener = UnixListener::bind(DAEMON_SOCKET_PATH).unwrap();

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            match socket.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    let cmd: Result<Command, _> = bincode::deserialize(&buf[..n]);
                    let response = match cmd {
                        Ok(Command::Add { path, name }) => {
                            Response::Ok("File added".into())
                        }
                        Ok(Command::Delete { name }) => {
                            Response::Ok("File deleted".into())
                        }
                        Ok(Command::List) => {
                            Response::Ok("LIST".into())
                        }
                        Err(e) => Response::Err(format!("Invalid command: {e}"))
                    };

                    let encoded = bincode::serialize(&response).unwrap();
                    let _ = socket.write_all(&encoded).await;
                }
                _ => {}
            }
        });
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
        Ok(_) => {}
        Err(e) => eprint!("Error sending command: {e}"),
    }
}