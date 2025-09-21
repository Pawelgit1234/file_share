mod network;

mod cli;
mod daemon;
mod config;

use clap::Parser;

use cli::{Cli, Commands};
use daemon::{start_daemon, stop_daemon};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { port, password } => {
            start_daemon(|| async {
                println!("Hello World");
            });
        }
        Commands::Stop => {
            stop_daemon();
        }
        Commands::Add { path, name } => {
        }
        Commands::Delete { name } => {
        }
        Commands::List => {
        }
    }
}
