// #![warn(missing_docs)]
#![warn(clippy::pedantic)]
//#![warn(clippy::cargo)]
//#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

mod args;
mod commands;
mod config;
mod ui;
mod util;

pub mod cli;

use console::Term;
use once_cell::sync::Lazy;

pub static TERM: Lazy<Term> = Lazy::new(Term::stdout);

#[cfg(feature = "debug")]
mod debug;

// TODO Check if leftovers are images and offer to delete.

// TODO? Update tag with leading/trailing whitespace?
// TODO? Separate Move ActionType into CopyFile and RemoveFile?
// TODO? Add more obscure tags?
