// src/main.rs
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod graveyard;
mod index;

#[derive(Parser)]
#[command(name = "riptide")]
#[command(about = "A safe replacement for rm with a graveyard", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Remove (move) files into the graveyard
    Rm {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// List files in the graveyard
    Ls,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Rm { paths } => {
            graveyard::bury(paths)?;
        }
        Commands::Ls => {
            graveyard::list()?;
        }
    }

    Ok(())
}
