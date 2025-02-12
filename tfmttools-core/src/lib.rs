pub mod action;
pub mod audiofile;
pub mod error;
pub mod history;
pub mod item_keys;
pub mod templates;
pub mod util;

use std::collections::HashMap;

use once_cell::sync::Lazy;

pub static FORBIDDEN_CHARACTERS: Lazy<HashMap<char, Option<&str>>> =
    Lazy::new(|| {
        let mut map = HashMap::new();

        map.insert('<', None);
        map.insert('"', None);
        map.insert('>', None);
        map.insert(':', None);
        map.insert('|', None);
        map.insert('?', None);
        map.insert('*', None);
        map.insert('~', Some("-"));
        map.insert('/', Some("-"));
        map.insert('\\', Some("-"));

        map
    });
