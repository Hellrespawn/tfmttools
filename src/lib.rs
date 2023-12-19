// #![warn(missing_docs)]
#![warn(clippy::pedantic)]
//#![warn(clippy::cargo)]
//#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

//! Tools to manage your music library using the `minijinja`
//! templating language.
//!
//! The `TagFormat` utility lets you use templates to dynamically
//! rename your music files based on their tags.

mod action;
mod audiofile;
mod fs;
mod template;
mod util;

/// Controls the command line interface
pub mod cli;

#[cfg(feature = "debug")]
mod debug;

// TODO Check if leftovers are images and offer to delete.

// TODO? Update tag with leading/trailing whitespace?
// TODO? Separate Move ActionType into CopyFile and RemoveFile?
// TODO? Add more obscure tags?
