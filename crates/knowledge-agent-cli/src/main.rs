use anyhow::{Result, bail};
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
    Init {
        #[arg(default_value = ".")]
        vault: PathBuf,
    },
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
        Command::Init { vault } => {
            let report = knowledge_agent_core::settings::init_vault(&vault)?;
            println!("initialized vault at {}", vault.display());
            println!(
                "{} {}",
                if report.created_vault_settings {
                    "created"
                } else {
                    "exists"
                },
                report.vault_settings_path.display()
            );
            println!(
                "{} {}",
                if report.created_local_state_dir {
                    "created"
                } else {
                    "exists"
                },
                report.local_state_dir.display()
            );
            println!(
                "{} {}",
                if report.updated_gitignore {
                    "updated"
                } else {
                    "ok"
                },
                report.gitignore_path.display()
            );
        }
        Command::Serve {
            vault,
            port,
            web_dir,
        } => {
            if !vault.is_dir() {
                bail!(
                    "vault path must be an existing directory: {}",
                    vault.display()
                );
            }
            knowledge_agent_server::serve(vault, port, web_dir).await?;
        }
    }
    Ok(())
}
