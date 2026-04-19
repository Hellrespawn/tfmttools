#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod cli;
mod commands;
mod history;
mod ui;

pub const PKG_NAME: &str = "tfmttools";

#[cfg(feature = "debug")]
mod debug;
