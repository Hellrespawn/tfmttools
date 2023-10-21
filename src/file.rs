#![allow(clippy::upper_case_acronyms)]
use crate::tags::Tags;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use lofty::{ItemKey, Tag, TaggedFileExt};
use std::path::{Path, PathBuf};

pub(crate) struct AudioFile {
    path: PathBuf,
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

    pub(crate) fn new(path: &Path) -> Result<AudioFile> {
        let path = path.to_owned();
        let tagged_file = lofty::read_from_path(&path)?;
        let tag = tagged_file
            .primary_tag()
            .ok_or_else(|| {
                eyre!("Unable to read primary tag for '{}'", path.display())
            })?
            .clone();

        let extension = path.extension().unwrap().to_string_lossy().to_string();

        Ok(AudioFile {
            path,
            tag,
            extension,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn extension(&self) -> &str {
        self.extension.as_ref()
    }
}

impl Tags for AudioFile {
    fn album(&self) -> Option<&str> {
        self.tag.get_string(&ItemKey::AlbumTitle)
    }

    fn album_artist(&self) -> Option<&str> {
        self.tag.get_string(&ItemKey::AlbumArtist)
    }

    fn albumsort(&self) -> Option<&str> {
        self.tag.get_string(&ItemKey::AlbumTitleSortOrder)
    }

    fn artist(&self) -> Option<&str> {
        self.tag.get_string(&ItemKey::TrackArtist)
    }

    fn genre(&self) -> Option<&str> {
        self.tag.get_string(&ItemKey::Genre)
    }

    fn title(&self) -> Option<&str> {
        self.tag.get_string(&ItemKey::TrackTitle)
    }

    fn raw_disc_number(&self) -> Option<&str> {
        self.tag.get_string(&ItemKey::DiscNumber)
    }

    fn raw_track_number(&self) -> Option<&str> {
        self.tag.get_string(&ItemKey::TrackNumber)
    }

    fn year(&self) -> Option<&str> {
        self.tag
            .get_string(&ItemKey::RecordingDate)
            .or_else(|| self.tag.get_string(&ItemKey::Year))
            .or_else(|| self.tag.get_string(&ItemKey::OriginalReleaseDate))
    }
}
