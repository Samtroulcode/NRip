use clap::builder::styling::{AnsiColor, Styles};
use clap::{ArgAction, ColorChoice, CommandFactory, FromArgMatches, Parser};
use std::io::IsTerminal as _;
use std::path::PathBuf;

mod fs_safemove;
mod graveyard;
mod index;
mod paths;
mod safety;
mod ui;

// Palette de styles pour l'aide Clap (-h/--help)
fn help_styles() -> Styles {
    Styles::styled()
        .usage(AnsiColor::Yellow.on_default().bold())
        .header(AnsiColor::Yellow.on_default().bold())
        .literal(AnsiColor::Green.on_default()) // noms d'options --long/-s
        .placeholder(AnsiColor::Cyan.on_default()) // <PLACEHOLDERS>
}

#[derive(Parser)]
#[command(
    name = "nrip", 
    version, 
    about = "Safe rm with a graveyard", 
    color = clap::ColorChoice::Auto, 
    styles = help_styles()
)]
struct Cli {
    /// Files/dirs to remove (default action)
    #[arg(value_name = "PATHS", allow_hyphen_values = true)]
    paths: Vec<PathBuf>,

    /// Prune graveyard; optional TARGET value allows `-p TARGET`
    #[arg(
        short = 'p',
        long = "prune",
        value_name = "TARGET",
        num_args = 0..=1,              // valeur optionnelle
        action = ArgAction::Set,       // important !
        conflicts_with = "paths"
    )]
    prune: Option<Option<String>>, // <- Option<Option<...>>

    /// (optional) explicit target
    #[arg(long = "target", requires = "prune")]
    target: Option<String>,

    /// Resurrect (restore) from graveyard; optional TARGET allows `-r TARGET`
    #[arg(
        short = 'r',
        long = "resurrect",
        value_name = "TARGET",
        num_args = 0..=1,
        action = ArgAction::Set,
        conflicts_with_all = ["paths", "prune", "target", "list"]
    )]
    resurrect: Option<Option<String>>,

    /// Force
    #[arg(short = 'f', long = "force")]
    force: bool,

    /// List graveyard contents
    #[arg(short = 'l', long = "list")]
    list: bool,

    /// Dry run (ni changes)
    #[arg(long)]
    dry_run: bool,

    /// (optional) skip confirmation prompts
    #[arg(short = 'y', long)]
    yes: bool,

    #[arg(hide = true, long = "__complete", value_names = ["CONTEXT", "PREFIX"], num_args = 1..=2)]
    __complete: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    // Politique couleur:
    // - Si NO_COLOR est défini → jamais de couleur
    // - Sinon, Auto (TTY uniquement)
    let no_color_env = std::env::var_os("NO_COLOR").is_some();
    let is_tty = std::io::stdout().is_terminal();

    // Config Clap (help colorisée)
    let mut cmd = Cli::command();
    cmd = cmd.color(if no_color_env {
        ColorChoice::Never
    } else {
        ColorChoice::Auto
    });
    let mut matches = cmd.get_matches();
    let cli = Cli::from_arg_matches_mut(&mut matches)?;

    // Config `yansi` (pour nos propres sorties)
    if no_color_env || !is_tty {
        yansi::disable();
    } else {
        yansi::enable();
    }
    // Internal completion endpoint
    if !cli.__complete.is_empty() {
        let context = cli.__complete[0].as_str();
        let prefix = cli.__complete.get(1).map(|s| s.as_str());
        match context {
            "prune" | "resurrect" => {
                for s in graveyard::completion_candidates(prefix)? {
                    println!("{s}");
                }
            }
            _ => {}
        }
        return Ok(());
    }

    // RESURRECT
    if let Some(res_opt) = cli.resurrect {
        // res_opt est déjà un Option<String> : None => interactif ; Some(s) => match par s
        let target = res_opt;
        graveyard::resurrect_cmd(target, cli.dry_run, cli.yes)?;
        return Ok(());
    }

    // PRUNE
    if let Some(prune_opt) = cli.prune {
        let target = match (cli.target, prune_opt) {
            (Some(t), _) => Some(t),    // --target prioritaire
            (None, Some(t)) => Some(t), // -p TARGET
            (None, None) => None,       // -p (prune total)
        };
        graveyard::prune(target, cli.dry_run, cli.yes)?;
        return Ok(());
    }

    // LIST
    if cli.list {
        graveyard::list()?;
        return Ok(());
    }

    // Default action: bury paths
    if !cli.paths.is_empty() {
        graveyard::bury(&cli.paths, cli.force)?;
        return Ok(());
    }

    // Nothing specified → show help
    let mut cmd = Cli::command();
    cmd.print_help()?;
    println!();
    Ok(())
}
