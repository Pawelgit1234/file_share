use clap::{Parser, Subcommand};

use crate::config::{AUTHOR, VERSION, ABOUT, LONG_ABOUT, NAME};

/// P2P File Share CLI
#[derive(Parser)]
#[command(
    name = NAME,
    author = AUTHOR,
    version = VERSION,
    about = ABOUT,
    long_about = LONG_ABOUT,
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the file sharing daemon
    Start {
        /// Port to listen on
        port: u16,
        /// Optional password for the daemon
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Stop the file sharing daemon
    Stop,

    /// Add a file to share
    Add {
        /// Path to the file
        path: String,
        /// Optional name to use in sharing (default: random id)
        name: Option<String>,
    },

    /// Delete a shared file
    Delete {
        /// Name of the file to delete
        name: String,
    },

    /// List all shared files
    List,
}