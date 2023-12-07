mod args;
mod commands;
mod main;
mod preview;
mod ui;
mod util;

use console::Term;
use once_cell::sync::Lazy;

pub(crate) static TERM: Lazy<Term> = Lazy::new(Term::stdout);

pub(crate) use args::Args;
pub use main::main;
