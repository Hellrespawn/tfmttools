#![warn(clippy::pedantic)]
//#![warn(clippy::cargo)]
// #![warn(missing_docs)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

mod args;
mod commands;
mod config;
mod history;
mod term;
mod ui;

pub mod cli;

pub const PKG_NAME: &str = "tfmttools";

#[cfg(feature = "debug")]
mod debug;
