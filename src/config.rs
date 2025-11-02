pub const NAME: &str = "File Share";
pub const AUTHOR: &str = "Pawelgit1234";
pub const VERSION: &str = "0.1.0";
pub const ABOUT: &str = "";
pub const LONG_ABOUT: &str = "";

pub const DAEMON_OUT_PATH: &str = "/tmp/file_share.out";
pub const DAEMON_ERR_PATH: &str = "/tmp/file_share.err";
pub const DAEMON_PID_PATH: &str = "/tmp/file_share.pid";
pub const DAEMON_SOCKET_PATH: &str = "/tmp/file_share.sock";

pub const CERT_PATH: &str = "~/.file_share/certs/cert.pem";
pub const KEY_PATH: &str = "~/.file_share/certs/key.pem";

pub const CHUNK_SIZE: usize = 64 * 1024; // 64 KB