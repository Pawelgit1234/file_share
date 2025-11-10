use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

// user sends it to daemon
#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonCommand {
    Add { path: String, name: Option<String> },
    Delete { name: String },
    List,
}

// response from daemon
#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonResponse {
    Ok(String),
    Err(String),
    List(HashMap<String, String>),
}

// oneshot message from daemon to server
pub struct DaemonMessage {
    pub cmd: DaemonCommand,
    pub resp_tx: oneshot::Sender<DaemonResponse>,
}
