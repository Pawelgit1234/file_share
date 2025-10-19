mod network;
mod core;

mod cli;
mod daemon;
mod config;

use clap::Parser;

use cli::{Cli, Commands};
use daemon::{start_daemon, stop_daemon};

use daemon::{ handle_response, send_command, Command };
use network::Server;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { port, password } => {
            start_daemon(move || async move {
                let server = Server::new(password);
                if let Err(err) = server.run(port).await {
                    eprintln!("Error by start of server: {err}");
                }
            });
        }
        Commands::Stop => {
            stop_daemon();
        }
        Commands::Add { path, name } => {
            handle_response(send_command(Command::Add { path, name }).await);
        }
        Commands::Delete { name } => {
            handle_response(send_command(Command::Delete { name }).await);
        }
        Commands::List => {
        }
    }
}
