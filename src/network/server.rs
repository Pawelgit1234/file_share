use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;

use tokio::net::{ TcpListener, TcpStream };
use tokio::sync::RwLock;

use crate::network::{recv_message, Request, Response};
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

        loop {
            let (socket, peer) = listener.accept().await?;
            println!("New connection: {peer}");

            let password = self.password.clone();
            let files = Arc::clone(&self.files);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(socket, password, files).await {
                    eprint!("Error handling client {peer}: {e}");
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

    pub async fn list_files(&self) -> Vec<String> {
        let files = self.files.read().await;
        files.keys().cloned().collect()
    }

    async fn handle_client(
        mut socket: TcpStream,
        password: Option<String>,
        files: Arc<RwLock<HashMap<String, PathBuf>>>
    ) -> anyhow::Result<()> {
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
                    let files = files.read().await;
                    send_message(&mut socket, &*files);
                }
                Request::Download(name) => {
                
                }
                Request::Quit => {
                    send_message(&mut socket, &Response::Bye);
                    break;
                }
                _ => { break; }
            }
                
        };
        Ok(())
    }
}
