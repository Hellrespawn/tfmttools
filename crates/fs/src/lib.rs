#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod action;
mod checksum;
mod file_or_name;
mod fs;
mod path_iterator;
mod template;

pub use action::ActionHandler;
pub use checksum::{get_file_checksum, get_path_checksum};
pub use file_or_name::FileOrName;
pub use fs::{FsHandler, RemoveDirResult, get_longest_common_prefix};
pub use path_iterator::{PathIterator, PathIteratorOptions};
pub use template::TemplateLoader;
