mod args;
mod commands;
mod history;
mod options;
mod term;
mod ui;

pub mod cli;

pub const PKG_NAME: &str = "tfmttools";

#[cfg(feature = "debug")]
mod debug;
