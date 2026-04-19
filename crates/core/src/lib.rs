#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod action;
pub mod audiofile;
pub mod error;
pub mod history;
pub mod item_keys;
pub mod templates;
pub mod util;

pub const MAX_PATH_LENGTH: usize = 256;
