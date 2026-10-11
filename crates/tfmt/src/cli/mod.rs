use std::process::ExitCode;

mod args;
mod args_definition;
pub(crate) mod options;

use chrono::Local;
#[cfg(not(feature = "debug"))]
use clap::error::ErrorKind;
use color_eyre::Result;
#[cfg(not(feature = "debug"))]
use color_eyre::eyre::Report;
use tfmttools_fs::FsHandler;
use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt, registry};

use self::args::TFMTArgs;
pub(crate) use self::args::{RenameArgs, ValidateArgs, ValidateType};
pub use self::options::{
    ConfirmMode, DisplayMode, RenameOptions, TFMTOptions, TemplateOption,
    ValidateOptions,
};
use crate::commands::CommandOutcome;
use crate::ui::show_cursor;

const LOG_ENV_VAR: &str = "TFMT_LOG";

/// Main entrypoint for tfmt
pub fn run() -> Result<ExitCode> {
    let _guard = init_tracing();

    info!("Initialized tracing.");

    debug!(
        "Running binary: {}",
        std::env::args().next().unwrap_or("Unknown".to_owned())
    );

    let args = TFMTArgs::parse();

    let name = args.command.name();

    if args.dry_run {
        println!("Doing dry run. No files will be modified.");
    }

    let result = (|| {
        let app_options = TFMTOptions::try_from(&args)?;
        let fs_handler = FsHandler::new(app_options.fs_mode());
        install_restore_cursor_hooks();
        args.command.run(&app_options, &fs_handler)
    })();
    command_exit_code(result, name)
}

fn command_exit_code(
    result: Result<CommandOutcome>,
    name: &str,
) -> Result<ExitCode> {
    #[cfg(not(feature = "debug"))]
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(error) => {
            render_cli_error(&error, name);
            return Ok(ExitCode::FAILURE);
        },
    };
    #[cfg(feature = "debug")]
    let outcome = {
        let _ = name;
        result?
    };
    Ok(match outcome {
        CommandOutcome::Success => ExitCode::SUCCESS,
        CommandOutcome::ValidationFailed => ExitCode::FAILURE,
    })
}

#[must_use]
pub fn command() -> clap::Command {
    TFMTArgs::command()
}

#[cfg(not(feature = "debug"))]
fn render_cli_error(err: &Report, name: &str) {
    let mut command = TFMTArgs::command();

    let subcommand = command.find_subcommand_mut(name);

    if let Some(subcommand) = subcommand {
        eprintln!(
            "{}",
            subcommand.error(ErrorKind::DisplayHelp, err.to_string())
        );
    } else {
        eprintln!("{}", command.error(ErrorKind::DisplayHelp, err.to_string()));
    }
}

// Initialize logger. if `TFMT_LOG` is set, write the log to the current
// directory.
fn init_tracing() -> Option<WorkerGuard> {
    if std::env::var_os(LOG_ENV_VAR).is_some() {
        let now = Local::now();

        let formatted = now.format("%Y%m%d.%H%M%S%3f");

        let file_appender = tracing_appender::rolling::never(
            std::env::current_dir().expect("Unable to get current directory."),
            format!("{}-{}.log", crate::PKG_NAME, formatted),
        );

        let (non_blocking, guard) =
            tracing_appender::non_blocking(file_appender);

        registry()
            .with(EnvFilter::from_env(LOG_ENV_VAR))
            .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
            .init();

        Some(guard)
    } else {
        None
    }
}

/// Add a custom hook that restores the cursor on panic or SIGINT
fn install_restore_cursor_hooks() {
    let default_panic = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        show_cursor();
        default_panic(info);
    }));

    ctrlc::set_handler(|| {
        show_cursor();
        println!();
        std::process::exit(-1);
    })
    .expect("Unable to intercept Ctrl-c.");
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::process::ExitCode;

    use super::*;
    use crate::commands::CommandOutcome;

    struct DropFlag<'a>(&'a Cell<bool>);
    impl Drop for DropFlag<'_> {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    #[test]
    fn normal_outcomes_return_and_drop_owned_guards() {
        for (result, expected) in [
            (Ok(CommandOutcome::Success), ExitCode::SUCCESS),
            (Ok(CommandOutcome::ValidationFailed), ExitCode::FAILURE),
            (
                Err(color_eyre::eyre::eyre!("operational failure")),
                ExitCode::FAILURE,
            ),
        ] {
            #[cfg(feature = "debug")]
            let is_error = result.is_err();
            let dropped = Cell::new(false);
            let outcome = {
                let _guard = DropFlag(&dropped);
                command_exit_code(result, "validate")
            };
            assert!(dropped.get());
            #[cfg(not(feature = "debug"))]
            assert_eq!(outcome.unwrap(), expected);
            #[cfg(feature = "debug")]
            {
                assert_eq!(outcome.is_err(), is_error);
                if let Ok(code) = outcome {
                    assert_eq!(code, expected);
                }
            }
        }
    }
}
