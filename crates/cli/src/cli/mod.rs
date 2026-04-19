mod args;
pub(crate) mod options;

use chrono::Local;
use clap::error::ErrorKind;
use color_eyre::Result;
use color_eyre::eyre::Report;
use tfmttools_fs::FsHandler;
use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt, registry};

pub(crate) use self::args::RenameArgs;
use self::args::TFMTArgs;
pub use self::options::{
    ConfirmMode, DisplayMode, RenameOptions, TFMTOptions, TemplateOption,
};
use crate::ui::show_cursor;

const LOG_ENV_VAR: &str = "TFMT_LOG";

/// Main entrypoint for tfmttools
pub fn run() -> Result<()> {
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

    let app_options = TFMTOptions::try_from(&args)?;

    let fs_handler = FsHandler::new(app_options.fs_mode());

    install_restore_cursor_hooks();

    let result = args.command.run(&app_options, &fs_handler);

    #[cfg(not(feature = "debug"))]
    if let Err(err) = result {
        render_cli_error(&err, name);
    }

    #[cfg(feature = "debug")]
    return result;

    Ok(())
}

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
