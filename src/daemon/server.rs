use std::fs;
use std::path::Path;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};
use tokio::net::UnixListener;

use crate::daemon::{DaemonCommand, DaemonMessage, DaemonResponse};
use super::{handle_daemon_message, start_daemon, send_command, stop_daemon, handle_response};
use crate::network::Server;
use crate::settings::{
    SERVER_DAEMON_SOCKET_PATH, SERVER_DAEMON_ERR_PATH,
    SERVER_DAEMON_OUT_PATH, SERVER_DAEMON_PID_PATH, ServerCliCommand
};

pub async fn handle_server_command(command: ServerCliCommand) {
    match command {
        ServerCliCommand::Start { port, password } => {
            start_daemon(move |mut rx| async move {
                let server = Server::new(password);
                if let Err(err) = server.run(port).await {
                    eprintln!("Error while starting server: {err}");
                }

                handle_daemon_message(rx, server).await;
            }, start_server_listener, SERVER_DAEMON_OUT_PATH, SERVER_DAEMON_ERR_PATH, SERVER_DAEMON_PID_PATH);
        }
        ServerCliCommand::Stop => {
            stop_daemon(SERVER_DAEMON_PID_PATH);
        }
        ServerCliCommand::Add { path, name } => {
            handle_response(send_command(DaemonCommand::Add { path, name }, SERVER_DAEMON_SOCKET_PATH).await);
        }
        ServerCliCommand::Delete { name } => {
            handle_response(send_command(DaemonCommand::Delete { name }, SERVER_DAEMON_SOCKET_PATH).await);
        }
        ServerCliCommand::List => {
            handle_response(send_command(DaemonCommand::List, SERVER_DAEMON_SOCKET_PATH).await);
        }
    }
}

async fn start_server_listener(tx: mpsc::Sender<DaemonMessage>) {
    if Path::new(SERVER_DAEMON_SOCKET_PATH).exists() {
        let _ = fs::remove_file(SERVER_DAEMON_SOCKET_PATH);
    }

    let listener = match UnixListener::bind(SERVER_DAEMON_SOCKET_PATH) {
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
                    match bincode::deserialize::<DaemonCommand>(&buf) {
                        Ok(cmd) => {
                            let (resp_tx, resp_rx) = oneshot::channel();
                            let msg = DaemonMessage { cmd, resp_tx };

                            if tx.send(msg).await.is_err() {
                                let _ = socket.write_all(
                                    &bincode::serialize(&DaemonResponse::Err("Daemon not running".into())).unwrap()
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
                                        &bincode::serialize(&DaemonResponse::Err("Daemon failed to respond".into())).unwrap()
                                    ).await;
                                }
                            }
                        }
                        Err(e) => {
                            let resp = DaemonResponse::Err(format!("Invalid command: {e}"));
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

