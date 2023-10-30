use color_eyre::Result;

use crate::cli::args::Command;
use crate::cli::commands::{self, UndoMode};
use crate::cli::{ui, Config};

/// Main entrypoint for tfmttools
pub fn main(dry_run_override: bool) -> Result<()> {
    let args = crate::cli::args::parse_args();

    let config = Config::from_args(&args)?.aggregate_dry_run(dry_run_override);

    if let Err(err) = select_command(config, args.command) {
        ui::print_error(&err);
    }

    Ok(())
}

fn select_command(config: Config, command: Command) -> Result<()> {
    match command {
        Command::ClearHistory { .. } => commands::clear_history(&config),
        Command::ListTemplates => commands::list_templates(&config),
        Command::Undo { times, .. } => {
            commands::undo(&config, UndoMode::Undo, times)
        },
        Command::Redo { times, .. } => {
            commands::undo(&config, UndoMode::Redo, times)
        },
        Command::Rename { name, arguments, recurse, .. } => {
            let config = config.with_recursion_depth(recurse);
            commands::rename(&config, &name, arguments)
        },

        Command::Seed { force, .. } => commands::seed(&config, force),
    }
}
