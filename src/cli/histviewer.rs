use crate::cli::Config;
use clap::Parser;
use color_eyre::Result;
use file_history::History;
use std::path::PathBuf;

#[derive(Parser, Debug, PartialEq)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    #[clap(parse(from_os_str))]
    config_dir: Option<PathBuf>,
}

/// Entry point for histviewer
pub fn histviewer() -> Result<()> {
    let args = Args::parse();

    let config_dir = args.config_dir.unwrap_or_else(|| {
        Config::default_path().expect("Unable to read home folder!")
    });

    let history = History::load(&config_dir, Config::HISTORY_NAME)?;

    println!("{history}");

    Ok(())
}
