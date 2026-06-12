use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "knowledge-agent")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve {
        vault: PathBuf,
        #[arg(long, default_value_t = 3030)]
        port: u16,
        #[arg(long)]
        web_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve {
            vault,
            port,
            web_dir,
        } => {
            knowledge_agent_server::serve(vault, port, web_dir).await?;
        }
    }
    Ok(())
}
