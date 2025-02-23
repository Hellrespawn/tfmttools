use camino::Utf8PathBuf;
use clap::error::ErrorKind;
use color_eyre::Result;
use tfmttools_fs::{FsHandler, PathIteratorOptions};
use tfmttools_history_core::HistoryMode;
use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt, registry};

use crate::TERM;
use crate::args::{Args, Subcommand};
use crate::commands::{
    FixCommand, RenameContext, RenameMiscOptions, RenameTemplateOptions,
    UndoRedoCommand, clear_history, copy_tags, list_templates, rename,
    show_history,
};
use crate::config::paths::AppPaths;

const LOG_ENV_VAR: &str = "TFMT_LOG";

/// Main entrypoint for tfmttools
pub fn main() -> Result<()> {
    let _guard = init_tracing();

    info!("Initialized tracing.");

    debug!(
        "Running binary: {}",
        std::env::args().next().unwrap_or("Unknown".to_owned())
    );

    let args = Args::parse();

    let result = run(args);

    if let Err(err) = result {
        eprintln!(
            "{}",
            Args::command().error(ErrorKind::DisplayHelp, err.to_string())
        );
    }

    Ok(())
}

fn run(args: Args) -> Result<()> {
    if args.dry_run {
        println!("Doing dry run. No files will be modified.");
    }
    let app_paths = AppPaths::from_args(&args)?;
    let fs_handler = FsHandler::new(args.dry_run);

    install_restore_cursor_hooks();

    match args.command {
        Subcommand::ClearHistory => clear_history(&app_paths, args.dry_run)?,
        Subcommand::Templates(list_templates_args) => {
            let template_directory = list_templates_args
                .custom_template_directory
                .unwrap_or(app_paths.config_directory().to_owned());

            list_templates(&template_directory)?;
        },
        Subcommand::Rename(rename_args) => {
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

            let misc_options = RenameMiscOptions::new(
                rename_args.always_copy,
                rename_args.yes,
                args.dry_run,
            );

            let rename_context = RenameContext::new(
                &app_paths,
                &fs_handler,
                &path_iterator_options,
                &template_options,
                misc_options,
            );

            rename(&rename_context)?;
        },
        Subcommand::Undo(undo_redo) => {
            UndoRedoCommand::new(
                undo_redo.yes,
                undo_redo.amount.unwrap_or(1),
                HistoryMode::Undo,
            )
            .run(&app_paths, &fs_handler)?;
        },
        Subcommand::Redo(undo_redo) => {
            UndoRedoCommand::new(
                undo_redo.yes,
                undo_redo.amount.unwrap_or(1),
                HistoryMode::Redo,
            )
            .run(&app_paths, &fs_handler)?;
        },
        Subcommand::History(show_history_args) => {
            show_history(&app_paths, show_history_args.verbose)?;
        },
        Subcommand::Fix(fix) => {
            let input_dir = get_input_directory(fix.custom_input_directory)?;

            FixCommand::new(
                input_dir,
                fix.yes,
                args.dry_run,
                fix.recursion_depth,
            )
            .run(&app_paths)?;
        },

        Subcommand::CopyTags(copy_tags_args) => {
            copy_tags(
                &copy_tags_args.source,
                &copy_tags_args.target,
                copy_tags_args.yes,
                args.dry_run,
            )?;
        },
    }

    Ok(())
}

// Initialize logger. if `TFMT_LOG` is set, write the log to the current
// directory.
fn init_tracing() -> Option<WorkerGuard> {
    if std::env::var_os(LOG_ENV_VAR).is_some() {
        let file_appender = tracing_appender::rolling::never(
            std::env::current_dir().expect("Unable to get current directory."),
            format!("{}.log", crate::PKG_NAME),
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

/// Make the cursor visible again, ignoring the result.
pub fn show_cursor() {
    let _ = TERM.show_cursor();
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
