use crate::settings::ClientCliCommand;

pub async fn handle_client_command(command: ClientCliCommand) {
    match command {
        ClientCliCommand::Connect { addr, password } => {
        }
        ClientCliCommand::Disconnect => {
        }
        ClientCliCommand::Download { name, output } => {
        }
        ClientCliCommand::List => {
        }
        // dowload list
    }
}