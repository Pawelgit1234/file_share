use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_rustls::server::TlsStream;

use crate::config::{CERT_PATH, KEY_PATH};
use crate::network::{create_or_load_tls, recv_message, Request, Response};
use crate::network::send_message;

pub struct Server {
    password: Option<String>,
    files: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl Server {
    pub fn new(password: Option<String>) -> Self {
        Server {
            password,
            files: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    pub async fn run(&self, port: u16) -> anyhow::Result<()> {
        let addr = format!("0.0.0.0:{port}");
        let listener = TcpListener::bind(addr).await?;
        let acceptor = Arc::from(create_or_load_tls(CERT_PATH, KEY_PATH)?);

        loop {
            let (socket, peer) = listener.accept().await?;
            println!("New connection: {peer}");

            let acceptor = Arc::clone(&acceptor);
            let password = self.password.clone();
            let files = Arc::clone(&self.files);

            tokio::spawn(async move {
                let tls_stream: TlsStream<_> = match acceptor.accept(socket).await {
                    Ok(stream) => stream,
                    Err(err) => {
                        eprintln!("TLS handshake failed for {peer}: {err}");
                        return;
                    }
                };

                if let Err(e) = Self::handle_client(tls_stream, password, files).await {
                    eprintln!("Error handling client {peer}: {e}");
                }
            });
        }
    }

    pub async fn add_file(&self, name: String, path: PathBuf) {
        let mut files = self.files.write().await;
        files.insert(name, path);
    }

    pub async fn remove_file(&self, name: &str) {
        let mut files = self.files.write().await;
        files.remove(name);
    }

    pub async fn list_files(&self) -> HashMap<String, String> {
        let files = self.files.read().await;
        files.iter()
            .map(|(k, v)| (k.clone(), v.to_string_lossy().to_string()))
            .collect()
    }

    async fn handle_client<S>(
        mut socket: S,
        password: Option<String>,
        files: Arc<RwLock<HashMap<String, PathBuf>>>
    ) -> anyhow::Result<()>
    where S: AsyncRead + AsyncWrite + Unpin
    {
        let req: Request = recv_message(&mut socket).await?;
        match req {
            Request::Auth(pass) if pass == password => {
                send_message(&mut socket, &Response::AuthOk).await?;
            }
            _ => {
                send_message(&mut socket, &Response::AuthErr).await?;
                return Ok(());
            }
        }

        loop {
            let req: Request = match recv_message(&mut socket).await {
                Ok(msg) => msg,
                Err(_) => {
                    println!("Client disconnected");
                    return Ok(());
                }
            };

            match req {
                Request::List => {
                    let files = files.read().await.keys().cloned().collect::<Vec<String>>();
                    send_message(&mut socket, &Response::List(files)).await?;
                }
                Request::Download(name) => {
                
                }
                Request::Quit => {
                    send_message(&mut socket, &Response::Bye).await?;
                    break;
                }
                _ => { break; }
            }
                
        };
        Ok(())
    }
}
