use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::util::PathOrString;

#[derive(Parser, Debug, PartialEq)]
#[command(version, about, long_about = None)]
/// Holds application-wide command line arguments.
pub(crate) struct Args {
    /// Sets a custom template_directory
    #[arg(short, long, alias = "config")]
    pub(crate) template_directory: Option<Utf8PathBuf>,

    #[arg(short, long)]
    /// Don't run command, only show what would happen.
    dry_run: bool,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand, Debug, PartialEq)]
/// Holds per-subcommand command line arguments.
pub(crate) enum Command {
    /// Clear undo/redo history.
    #[command(name = "clear")]
    ClearHistory {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,
    },

    /// Lists all available templates.
    #[command(name = "list")]
    ListTemplates,
    /// Rename files according to their tags.
    Rename {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        #[arg(short, long)]
        /// Maximum recursion depth when gathering files.
        recurse: Option<usize>,

        /// Name or path of desired template.
        name: PathOrString,

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
    Undo {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        amount: Option<usize>,
    },
    Redo {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        amount: Option<usize>,
    },
}

impl Args {
    pub(crate) fn parse() -> Args {
        <Self as Parser>::parse()
    }

    #[must_use]
    pub(crate) fn dry_run(&self) -> bool {
        self.dry_run
            || match self.command {
                Command::ClearHistory { dry_run }
                | Command::Rename { dry_run, .. }
                | Command::Seed { dry_run, .. }
                | Command::Undo { dry_run, .. }
                | Command::Redo { dry_run, .. } => dry_run,
                Command::ListTemplates => false,
            }
    }
}

impl TryFrom<&Args> for Config {
    type Error = color_eyre::Report;

    fn try_from(args: &Args) -> Result<Self, Self::Error> {
        let path = if let Some(path) = &args.template_directory {
            path.clone()
        } else {
            Self::default_path()?
        };

        let dry_run = args.dry_run();

        let mut config = Self::new(dry_run, &path)?;

        if let Command::Rename { recurse, .. } = &args.command {
            config.set_recursion_depth(*recurse);
        }

        Ok(config)
    }
}
