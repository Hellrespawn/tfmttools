#![allow(clippy::upper_case_acronyms)]
use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use lofty::{ItemKey, Tag, TaggedFileExt};
use once_cell::sync::Lazy;

use crate::tags::Tags;

pub(crate) static FORBIDDEN_CHARACTERS: Lazy<HashMap<char, Option<&str>>> =
    Lazy::new(|| {
        let mut map = HashMap::new();

        map.insert('<', None);
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

pub(crate) struct AudioFile {
    path: Utf8PathBuf,
    tag: Tag,
    extension: String,
}

impl std::fmt::Debug for AudioFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioFile")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl AudioFile {
    pub(crate) const SUPPORTED_EXTENSIONS: [&'static str; 2] = ["mp3", "ogg"];

    pub(crate) fn new(path: &Utf8Path) -> Result<AudioFile> {
        let path = path.to_owned();
        let tagged_file = lofty::read_from_path(&path)?;
        let tag = tagged_file
            .primary_tag()
            .ok_or_else(|| eyre!("Unable to read primary tag for '{}'", path))?
            .clone();

        let extension = path.extension().unwrap().to_string();

        Ok(AudioFile { path, tag, extension })
    }

    pub(crate) fn path_predicate(path: &Utf8Path) -> bool {
        path.extension().map_or(false, |extension| {
            for supported_extension in AudioFile::SUPPORTED_EXTENSIONS {
                if extension == supported_extension {
                    return true;
                }
            }

            false
        })
    }

    pub(crate) fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub(crate) fn extension(&self) -> &str {
        self.extension.as_ref()
    }

    fn get_tag_safe(&self, key: &ItemKey) -> Option<String> {
        self.tag.get_string(key).map(|string| {
            FORBIDDEN_CHARACTERS.iter().fold(
                string.to_owned(),
                |string, (char, replacement)| {
                    string.replace(*char, replacement.unwrap_or(""))
                },
            )
        })
    }
}

impl Tags for AudioFile {
    fn album(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::AlbumTitle)
    }

    fn album_artist(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::AlbumArtist)
    }

    fn albumsort(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::AlbumTitleSortOrder)
    }

    fn artist(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::TrackArtist)
    }

    fn genre(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::Genre)
    }

    fn title(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::TrackTitle)
    }

    fn raw_disc_number(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::DiscNumber)
    }

    fn raw_track_number(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::TrackNumber)
    }

    fn date(&self) -> Option<String> {
        self.get_tag_safe(&ItemKey::RecordingDate)
            .or_else(|| self.get_tag_safe(&ItemKey::Year))
            .or_else(|| self.get_tag_safe(&ItemKey::OriginalReleaseDate))
    }
}
