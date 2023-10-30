use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug, PartialEq)]
#[command(version, about, long_about = None)]
/// Holds application-wide command line arguments.
pub(crate) struct Args {
    /// Sets a custom config folder
    #[arg(short, long)]
    pub(crate) config: Option<PathBuf>,

    #[arg(short, long)]
    /// Don't run command, only show what would happen.
    dry_run: bool,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand, Debug, PartialEq)]
/// Holds per-subcommand command line arguments.
pub(crate) enum Command {
    /// Clears the history
    #[command(name = "clear")]
    ClearHistory {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,
    },
    /// Lists all available templates.
    #[command(name = "list")]
    ListTemplates,
    /// Undo {times} times.
    Undo {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        /// Times to undo.
        #[arg(default_value_t = 1)]
        times: usize,
    },
    /// Redo {times} times.
    Redo {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        /// Times to redo
        #[arg(default_value_t = 1)]
        times: usize,
    },
    /// Rename files according to their tags.
    Rename {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        #[arg(short, long)]
        /// Maximum recursion depth when gathering files.
        recurse: Option<usize>,

        /// Name or path of desired template.
        name: String,

        /// Arguments array to pass to template.
        arguments: Vec<String>,
    },
    /// Adds examples to the filesystem.
    Seed {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        #[arg(short, long)]
        /// Overwrite existing files.
        force: bool,
    },
}

impl Args {
    #[must_use]
    pub(crate) fn dry_run(&self) -> bool {
        self.dry_run
            || match self.command {
                Command::ClearHistory { dry_run, .. }
                | Command::Undo { dry_run, .. }
                | Command::Redo { dry_run, .. }
                | Command::Rename { dry_run, .. }
                | Command::Seed { dry_run, .. } => dry_run,
                Command::ListTemplates => false,
            }
    }
}

/// Parses arguments
pub(crate) fn parse_args() -> Args {
    Args::parse()
}
