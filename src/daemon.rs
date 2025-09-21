use std::fs::{File};
use std::fs;

use daemonize::Daemonize;

use crate::config::{DAEMON_ERR_PATH, DAEMON_OUT_PATH, DAEMON_PID_PATH};

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
                .block_on(callback());
        }
        Err(e) => eprintln!("Error: {}", e),
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