use serde::{Deserialize, Serialize};

// client -> server
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Auth(Option<String>),
    Quit,

    List,

    Download { name: String, offset: u64 },
    Ack { index: u64 }
}

// server -> client
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    AuthOk,
    AuthErr,
    Bye,

    List(Vec<String>),
    Error(String),

    FileInfo {
        name: String,
        size: u64,
        hash: String,
        chunk_size: u64,
    },
    Chunck { index: u64, data: Vec<u8> },
    Done,
}