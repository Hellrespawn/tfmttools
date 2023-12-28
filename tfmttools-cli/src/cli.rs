use color_eyre::Result;
use tracing::{debug, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, registry, EnvFilter};

use super::config::Config;
use crate::args::Args;
use crate::TERM;

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

    let config = Config::from_args(args)?;

    debug!("Configuration:\n{:#?}", config);

    hide_cursor();

    let result = config.command().run(&config);

    show_cursor();

    result
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

/// Add a custom hook that restores the cursor on panic, then hide the cursor.
/// Ignore the result.
fn hide_cursor() {
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

    let _ = TERM.hide_cursor();
}

/// Make the cursor visible again, ignoring the result.
fn show_cursor() {
    let _ = TERM.show_cursor();
}
