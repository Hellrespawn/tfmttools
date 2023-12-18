mod commands;
mod config;
mod main;
mod ui;
mod util;

pub mod preview;

use console::Term;
use once_cell::sync::Lazy;

pub static TERM: Lazy<Term> = Lazy::new(Term::stdout);

pub use main::main;
