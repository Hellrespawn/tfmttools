pub mod cli;
mod commands;
mod history;
mod ui;

pub const PKG_NAME: &str = "tfmt";

#[cfg(feature = "debug")]
mod debug;
