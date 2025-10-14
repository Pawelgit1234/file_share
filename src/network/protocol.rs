use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

// client -> server
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Auth(Option<String>),
    List,
    Download(String),
    Quit,
}

// server -> client
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    AuthOk,
    AuthErr,
    List(HashMap<String, PathBuf>),
    Data(Vec<u8>),
    Error(String),
    Bye,
}