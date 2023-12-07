use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::util::PathOrString;

#[derive(Parser, Debug, PartialEq)]
#[command(version, about, long_about = None)]
/// Holds application-wide command line arguments.
pub(crate) struct Args {
    /// Sets a custom config and template_directory
    #[arg(short, long, alias = "config")]
    pub(crate) config_and_template_directory: Option<Utf8PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

// TODO Migrate Command enum to separate structs. Move config_andtemplate_directory to appropriate commands.

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
        /// Sets a custom input_directory
        #[arg(short, long, alias = "input", default_value_t = std::env::current_dir().expect("Unable to read CWD.").try_into().expect("CWD is not valid UTF8"))]
        input_directory: Utf8PathBuf,

        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        #[arg(short, long)]
        /// Skip interactive preview
        force: bool,

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

        #[arg(short, long)]
        /// Skip interactive preview
        force: bool,

        amount: Option<usize>,
    },
    Redo {
        #[arg(short, long)]
        /// Don't run command, only show what would happen.
        dry_run: bool,

        #[arg(short, long)]
        /// Skip interactive preview
        force: bool,

        amount: Option<usize>,
    },
}

impl Args {
    pub(crate) fn parse() -> Args {
        <Self as Parser>::parse()
    }

    #[must_use]
    pub(crate) fn dry_run(&self) -> bool {
        match self.command {
            Command::ClearHistory { dry_run }
            | Command::Rename { dry_run, .. }
            | Command::Seed { dry_run, .. }
            | Command::Undo { dry_run, .. }
            | Command::Redo { dry_run, .. } => dry_run,
            Command::ListTemplates => false,
        }
    }

    #[must_use]
    pub(crate) fn force(&self) -> bool {
        match self.command {
            Command::Rename { force, .. }
            | Command::Seed { force, .. }
            | Command::Undo { force, .. }
            | Command::Redo { force, .. } => force,
            Command::ListTemplates | Command::ClearHistory { .. } => false,
        }
    }
}

impl TryFrom<&Args> for Config {
    type Error = color_eyre::Report;

    fn try_from(args: &Args) -> Result<Self, Self::Error> {
        let template_directory =
            if let Some(path) = &args.config_and_template_directory {
                path.clone()
            } else {
                Self::default_path()?
            };

        let mut config =
            Self::new(args.dry_run(), args.force(), &template_directory)?;

        if let Command::Rename { recurse, .. } = &args.command {
            config.set_recursion_depth(*recurse);
        }

        Ok(config)
    }
}
