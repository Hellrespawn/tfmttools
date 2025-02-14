#![warn(clippy::pedantic)]
//#![warn(clippy::cargo)]
// #![warn(missing_docs)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

mod args;
mod commands;
mod config;
mod history;
mod ui;

pub mod cli;

use console::Term;
use once_cell::sync::Lazy;

pub static TERM: Lazy<Term> = Lazy::new(Term::stdout);
pub const PKG_NAME: &str = "tfmttools";

#[cfg(feature = "debug")]
mod debug;
