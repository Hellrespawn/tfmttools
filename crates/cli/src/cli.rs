use chrono::Local;
use clap::error::ErrorKind;
use color_eyre::Result;
use tfmttools_core::util::{Utf8Directory, Utf8PathExt};
use tfmttools_fs::{FsHandler, PathIteratorOptions};
use tfmttools_history::HistoryMode;
use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt, registry};

use crate::args::{TFMTArgs, TFMTSubcommand};
use crate::commands::{
    RenameContext, UndoRedoCommand, clear_history, list_templates, rename,
    show_history,
};
use crate::options::{ConfirmMode, RenameOptions, TFMTOptions};
use crate::term::show_cursor;

const LOG_ENV_VAR: &str = "TFMT_LOG";

/// Main entrypoint for tfmttools
pub fn main() -> Result<()> {
    let _guard = init_tracing();

    info!("Initialized tracing.");

    debug!(
        "Running binary: {}",
        std::env::args().next().unwrap_or("Unknown".to_owned())
    );

    let args = TFMTArgs::parse();

    let name = args.command.name();

    let result = run(args);

    #[cfg(not(feature = "debug"))]
    if let Err(err) = result {
        let mut command = TFMTArgs::command();

        let subcommand = command.find_subcommand_mut(name);

        if let Some(subcommand) = subcommand {
            eprintln!(
                "{}",
                subcommand.error(ErrorKind::DisplayHelp, err.to_string())
            );
        } else {
            eprintln!(
                "{}",
                command.error(ErrorKind::DisplayHelp, err.to_string())
            );
        }
    }

    #[cfg(feature = "debug")]
    return result;

    Ok(())
}

fn run(args: TFMTArgs) -> Result<()> {
    if args.dry_run {
        println!("Doing dry run. No files will be modified.");
    }
    let app_options = TFMTOptions::try_from(&args)?;

    let fs_handler = FsHandler::new(app_options.fs_mode());

    install_restore_cursor_hooks();

    match args.command {
        TFMTSubcommand::ClearHistory => {
            clear_history(&app_options)?;
        },
        TFMTSubcommand::ListTemplates(list_templates_args) => {
            let template_directory = list_templates_args
                .custom_template_directory
                .map(Utf8Directory::new)
                .unwrap_or(Ok(app_options.config_directory().to_owned()))?;

            list_templates(&template_directory)?;
        },
        TFMTSubcommand::Rename(rename_args) => {
            let rename_options =
                RenameOptions::try_from((rename_args, &app_options))?;

            let path_iterator_options = PathIteratorOptions::with_depth(
                rename_options.input_directory().as_path(),
                rename_options.recursion_depth(),
            );

            let rename_context = RenameContext::new(
                &fs_handler,
                &path_iterator_options,
                &app_options,
                &rename_options,
            );

            rename(&rename_context)?;
        },
        TFMTSubcommand::Undo(undo_redo_args) => {
            UndoRedoCommand::new(
                matches!(app_options.confirm_mode(), ConfirmMode::NoConfirm),
                undo_redo_args.amount.unwrap_or(1),
                HistoryMode::Undo,
                app_options.preview_list_size(),
            )
            .run(&app_options, &fs_handler)?;
        },
        TFMTSubcommand::Redo(undo_redo_args) => {
            UndoRedoCommand::new(
                matches!(app_options.confirm_mode(), ConfirmMode::NoConfirm),
                undo_redo_args.amount.unwrap_or(1),
                HistoryMode::Redo,
                app_options.preview_list_size(),
            )
            .run(&app_options, &fs_handler)?;
        },
        TFMTSubcommand::ShowHistory => {
            show_history(&app_options)?;
        },
    }

    Ok(())
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
