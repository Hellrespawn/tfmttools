#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod context;
mod data;
mod outcome;
mod runner;

pub use runner::test_runner;
