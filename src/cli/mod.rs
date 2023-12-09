mod commands;
mod config;
mod main;
mod preview;
mod ui;
mod util;

use console::Term;
use once_cell::sync::Lazy;

pub static TERM: Lazy<Term> = Lazy::new(Term::stdout);

pub use main::main;
