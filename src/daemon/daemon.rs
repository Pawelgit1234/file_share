use std::fs::File;
use std::fs;
use std::path::{Path, PathBuf};

use daemonize::Daemonize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use crate::daemon::{DaemonCommand, DaemonMessage, DaemonResponse};
use crate::network::Server;

pub fn start_daemon<F, Fut, L, Lfut>(
    callback: F,
    listener: L,
    out_path: &str,
    err_path: &str,
    pid_path: &str,
)
where
    F: FnOnce(mpsc::Receiver<DaemonMessage>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
    L: FnOnce(mpsc::Sender<DaemonMessage>) -> Lfut + Send + 'static,
    Lfut: std::future::Future<Output = ()> + Send + 'static,
{
    let stdout = File::create(out_path).unwrap();
    let stderr = File::create(err_path).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(pid_path)
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
                    tokio::spawn(listener(tx.clone()));
                    callback(rx).await;
                });
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

pub fn stop_daemon(pid_path: &str) {
    if let Ok(pid_str) = fs::read_to_string(pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            let res = unsafe { libc::kill(pid, libc::SIGTERM) }; // kills the process
            if res == 0 {
                println!("Stopped daemon with PID {pid}");
            } else {
                eprintln!("Failed to kill PID {pid}");
            }
            let _ = fs::remove_file(pid_path);
        }
    } else {
        eprintln!("No running daemon found");
    }
}

pub async fn send_command(cmd: DaemonCommand, socket_path: &str) -> anyhow::Result<DaemonResponse> {
    let mut stream = UnixStream::connect(socket_path).await?;

    // sends command
    let encoded: Vec<u8> = bincode::serialize(&cmd)?;
    stream.write(&encoded).await?;

    // response
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await?;
    let resp: DaemonResponse = bincode::deserialize(&buf[..n])?;

    Ok(resp)
}

pub async fn handle_daemon_message(mut rx: mpsc::Receiver<DaemonMessage>, server: Server) {
    while let Some(msg) = rx.recv().await {
        let DaemonMessage { cmd, resp_tx } = msg;
        let resp = match cmd {
            DaemonCommand::Add { path, name } => {
                // if name is None than take it from path: .../.../test.txt -> test.txt
                let name = name.unwrap_or_else(|| {
                    Path::new(&path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into()
                });
                server.add_file(name, PathBuf::from(path)).await;
                DaemonResponse::Ok("File added".into())
            }
            DaemonCommand::Delete { name } => {
                server.remove_file(&name).await;
                DaemonResponse::Ok(format!("File '{name}' deleted"))
            }
            DaemonCommand::List => {
                let list = server.list_files().await;
                DaemonResponse::List(list)
            }
        };

        let _ = resp_tx.send(resp);
    }
}

pub fn handle_response(result: anyhow::Result<DaemonResponse>) {
    match result {
        Ok(DaemonResponse::Ok(msg)) => println!("{msg}"),
        Ok(DaemonResponse::Err(msg)) => eprintln!("{msg}"),
        Ok(DaemonResponse::List(files)) => {
            for (k, v) in files {
                println!("{k} ({v})");
            }
        }
        Err(e) => eprint!("Error sending command: {e}"),
    }
}
