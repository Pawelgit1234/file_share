use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;

use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_rustls::server::TlsStream;
use async_compression::tokio::write::ZstdEncoder;

use crate::settings::{CERT_PATH, CHUNK_SIZE, KEY_PATH};
use crate::network::{create_or_load_tls, recv_message, send_message, Request, Response};
use crate::utils::{get_file_length, hash_file};

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
                Request::Download { name, offset } => {
                    // find file
                    let read_files = files.read().await;
                    let Some(path) = read_files.get(&name) else {
                        send_message(&mut socket, &Response::Error("File not found".into())).await?;
                        continue;
                    };

                    // try to open file
                    let Ok(mut file) = File::open(path).await else {
                        let mut files = files.write().await;
                        files.remove(&name);
                        send_message(&mut socket, &Response::Error("Error opening file".into())).await?;
                        continue;
                    };

                    send_message(
                        &mut socket,
                        &Response::FileInfo {
                            name: name.clone(),
                            size: get_file_length(&file).await?,
                            hash: hash_file(&path).await?,
                            chunk_size: CHUNK_SIZE as u64,
                        }
                    ).await?;

                    file.seek(std::io::SeekFrom::Start(offset)).await?;
                    let mut encoder = ZstdEncoder::new(&mut socket);
                    
                    let mut buf = vec![0u8; CHUNK_SIZE];
                    let mut index: u64 = offset / CHUNK_SIZE as u64;

                    loop {
                        let n = file.read(&mut buf).await?;
                        if n == 0 { break; }

                        let chunk = Response::Chunck { index, data: buf[..n].to_vec() };
                        send_message(&mut encoder, &chunk).await?;

                        index += 1;

                        match recv_message::<Request, _>(&mut encoder).await {
                            Ok(Request::Ack { index: ack_idx }) if ack_idx == index - 1 => {}
                            Ok(Request::Ack { .. }) => {
                                eprintln!("Client ack mismatch, stopping transfer");
                                break;
                            }
                            _ => {
                                eprintln!("Client did not ack properly");
                                break;
                            }
                        }
                    }

                    encoder.shutdown().await?;
                    send_message(&mut socket, &Response::Done).await?;
                    println!("File '{name}' sent successfully to client");
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
