// src/main.rs
use clap::{ArgAction, CommandFactory, Parser};
use std::path::PathBuf;

mod graveyard;
mod index;

#[derive(Parser)]
#[command(name = "rip", version, about = "Safe rm with a graveyard")]
struct Cli {
    /// Files/dirs to remove (default action)
    #[arg(value_name = "PATHS")]
    paths: Vec<PathBuf>,

    /// Prune graveyard; optional TARGET value allows `-p TARGET`
    /// - `-p`            -> prune all (with confirmation)
    /// - `-p TARGET`     -> prune matches for TARGET (basename or id prefix)
    #[arg(
        short = 'p',
        long = "prune",
        value_name = "TARGET",
        num_args = 0..=1,              // valeur optionnelle
        action = ArgAction::Set,       // important !
        conflicts_with = "paths"
    )]
    prune: Option<Option<String>>, // <- Option<Option<...>>

    /// (optional) explicit target ;
    #[arg(long = "target", requires = "prune")]
    target: Option<String>,

    #[arg(short = 'l', long = "list")]
    list: bool,

    #[arg(long)]
    dry_run: bool,

    #[arg(short = 'y', long)]
    yes: bool,

    #[arg(hide = true, long = "__complete", value_names = ["CONTEXT", "PREFIX"], num_args = 1..=2)]
    __complete: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Internal completion endpoint
    if !cli.__complete.is_empty() {
        let context = cli.__complete[0].as_str();
        let prefix = cli.__complete.get(1).map(|s| s.as_str());
        if context == "prune" {
            for s in graveyard::completion_candidates(prefix)? {
                println!("{s}");
            }
        }
        return Ok(());
    }

    // Operations
    if let Some(prune_opt) = cli.prune {
        let target = match (cli.target, prune_opt) {
            (Some(t), _) => Some(t),    // --target prioritaire
            (None, Some(t)) => Some(t), // -p TARGET
            (None, None) => None,       // -p (prune total)
        };
        graveyard::prune(target, cli.dry_run, cli.yes)?;
        return Ok(());
    }

    if cli.list {
        graveyard::list()?;
        return Ok(());
    }

    // Default action: bury paths
    if !cli.paths.is_empty() {
        graveyard::bury(cli.paths)?;
        return Ok(());
    }

    // Nothing specified → show help
    let mut cmd = Cli::command();
    cmd.print_help()?;
    println!();
    Ok(())
}

