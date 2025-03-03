use camino::Utf8PathBuf;
use chrono::Local;
use clap::error::ErrorKind;
use color_eyre::Result;
use tfmttools_fs::{FsHandler, PathIteratorOptions};
use tfmttools_history_core::HistoryMode;
use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt, registry};

use crate::args::{TFMTArgs, TFMTSubcommand};
use crate::commands::{
    RenameContext, RenameMiscOptions, RenameTemplateOptions, UndoRedoCommand,
    clear_history, list_templates, rename, show_history,
};
use crate::config::paths::AppPaths;
use crate::term::{show_cursor, terminal_height};
use crate::ui::PreviewListSize;

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

    Ok(())
}

fn run(args: TFMTArgs) -> Result<()> {
    if args.dry_run {
        println!("Doing dry run. No files will be modified.");
    }
    let app_paths = AppPaths::from_args(&args)?;
    let fs_handler = FsHandler::new(args.dry_run);

    let preview_list_size = args
        .preview_list_size
        .unwrap_or_else(|| PreviewListSize::new(terminal_height()));

    install_restore_cursor_hooks();

    match args.command {
        TFMTSubcommand::ClearHistory => {
            clear_history(&app_paths, args.dry_run)?;
        },
        TFMTSubcommand::Templates(list_templates_args) => {
            let template_directory = list_templates_args
                .custom_template_directory
                .unwrap_or(app_paths.config_directory().to_owned());

            list_templates(&template_directory)?;
        },
        TFMTSubcommand::Rename(rename_args) => {
            let input_directory =
                get_input_directory(rename_args.custom_input_directory)?;

            let template_directory = rename_args
                .custom_template_directory
                .unwrap_or(app_paths.config_directory().to_owned());

            let path_iterator_options = PathIteratorOptions::with_depth(
                &input_directory,
                rename_args.recursion_depth,
            );

            let template_options = RenameTemplateOptions::new(
                template_directory,
                rename_args.template,
                rename_args.arguments,
            );

            let mut misc_options = RenameMiscOptions::new(
                rename_args.always_copy,
                rename_args.yes,
                args.dry_run,
                preview_list_size,
            );

            if let Some(run_id) = args.custom_run_id {
                misc_options = misc_options.with_run_id(run_id);
            }

            let rename_context = RenameContext::new(
                &app_paths,
                &fs_handler,
                &path_iterator_options,
                &template_options,
                &misc_options,
            );

            rename(&rename_context)?;
        },
        TFMTSubcommand::Undo(undo_redo) => {
            UndoRedoCommand::new(
                undo_redo.yes,
                undo_redo.amount.unwrap_or(1),
                HistoryMode::Undo,
                preview_list_size,
            )
            .run(&app_paths, &fs_handler)?;
        },
        TFMTSubcommand::Redo(undo_redo) => {
            UndoRedoCommand::new(
                undo_redo.yes,
                undo_redo.amount.unwrap_or(1),
                HistoryMode::Redo,
                preview_list_size,
            )
            .run(&app_paths, &fs_handler)?;
        },
        TFMTSubcommand::History(show_history_args) => {
            show_history(&app_paths, show_history_args.verbose)?;
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

pub fn default_input_dir() -> Result<Utf8PathBuf> {
    let path = std::env::current_dir()?;

    Ok(path.clone().try_into()?)
}

fn get_input_directory(
    custom_input_directory: Option<Utf8PathBuf>,
) -> Result<Utf8PathBuf> {
    let input_directory = if let Some(input_directory) = custom_input_directory
    {
        Ok(input_directory)
    } else {
        default_input_dir()
    }?;

    Ok(input_directory)
}
