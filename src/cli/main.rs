use color_eyre::Result;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, registry, EnvFilter};

use super::Args;
use crate::cli::args::Command;
use crate::cli::{commands, ui};
use crate::config::Config;
use crate::TERM;

/// Main entrypoint for tfmttools
pub fn main(dry_run_override: bool) -> Result<()> {
    registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let mut config: Config = (&args).try_into()?;

    *config.dry_run_mut() = config.dry_run() || dry_run_override;

    hide_cursor();

    if let Err(err) = select_command(&config, args.command) {
        ui::print_error(&err);
    }

    let _ = TERM.show_cursor();

    Ok(())
}

fn hide_cursor() {
    let default_panic = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        let _ = TERM.show_cursor();
        default_panic(info);
    }));

    let _ = TERM.hide_cursor();
}

fn select_command(config: &Config, command: Command) -> Result<()> {
    match command {
        Command::ClearHistory { .. } => commands::clear_history(config),
        Command::ListTemplates => commands::list_templates(config),
        Command::Rename { name, arguments, .. } => {
            commands::rename(config, &name, arguments)
        },

        Command::Seed { force, .. } => commands::seed(config, force),
    }
}
