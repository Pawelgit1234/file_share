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
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage the local file-sharing daemon
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },

    /// Connect to and interact with a remote file server
    Client {
        #[command(subcommand)]
        command: ClientCommands,
    },
}

/// Commands for managing your local daemon
#[derive(Subcommand)]
pub enum DaemonCommands {
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
        /// Optional custom name for sharing
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

/// Commands for connecting to a remote server
#[derive(Subcommand)]
pub enum ClientCommands {
    /// Connect to a remote server
    Connect {
        /// Server address
        addr: String,
        /// Optional password
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Disconnect from the current server
    Disconnect,

    /// Request a list of available files
    List,

    /// Download a file
    Download {
        /// File name to download
        name: String,
        /// Save path
        #[arg(short, long)]
        output: Option<String>,
    },
}
