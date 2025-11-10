mod network;
mod daemon;
mod settings;
mod utils;

use clap::Parser;

use settings::cli::{Cli, Command};
use daemon::{handle_client_command, handle_server_command};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Daemon { command } => handle_server_command(command).await,
        Command::Client { command } => handle_client_command(command).await,
    }
}
