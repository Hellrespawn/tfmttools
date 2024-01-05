use lofty::ItemKey;
use minijinja::value::StructObject;
use minijinja::Value;
use tracing::trace;

use crate::audiofile::AudioFile;
use crate::fs::FORBIDDEN_CHARACTERS;

#[derive(Debug)]
#[allow(dead_code)]
pub enum InterpolationMode {
    Normal,
    Safe,
    Strict,
}

#[derive(Debug)]
pub struct AudioFileContext {
    audio_file: AudioFile,
    arguments: Vec<String>,
    mode: InterpolationMode,
}

impl AudioFileContext {
    pub fn safe(audio_file: AudioFile, arguments: Vec<String>) -> Self {
        Self { audio_file, arguments, mode: InterpolationMode::Safe }
    }

    fn process_mode(&self, value: String) -> String {
        match self.mode {
            InterpolationMode::Normal => value,
            InterpolationMode::Safe => {
                FORBIDDEN_CHARACTERS.iter().fold(
                    value,
                    |string, (char, replacement)| {
                        string.replace(*char, replacement.unwrap_or(""))
                    },
                )
            },
            InterpolationMode::Strict => unimplemented!(),
        }
    }

    fn read_value(&self, key: &ItemKey) -> Option<String> {
        let tag = self
            .audio_file
            .tag()
            .get_string(key)
            .map(std::borrow::ToOwned::to_owned);

        trace!(
            "[{}][{:?}] => '{}'",
            self.audio_file.path().file_name().unwrap_or("unknown"),
            key,
            if let Some(tag) = &tag { tag } else { "unknown" }
        );

        tag
    }

    fn get_string(&self, key: &ItemKey) -> Option<Value> {
        Some(self.process_mode(self.read_value(key)?).into())
    }

    fn get_number(&self, key: &ItemKey) -> Option<Value> {
        let tag = self.read_value(key)?;

        let number = if let Some((current, _)) = tag.split_once('/') {
            current.to_owned()
        } else {
            tag.clone()
        };

        trace!(
            "[{}][{:?}] '{}' => '{}' (as number)",
            self.audio_file.path().file_name().unwrap_or("unknown"),
            key,
            tag,
            number
        );

        let number = self.process_mode(number);

        let number = number.parse::<usize>().ok()?;

        Some(number.into())
    }

    fn get_date(&self) -> Option<Value> {
        self.get_string(&ItemKey::RecordingDate)
            .or_else(|| self.get_string(&ItemKey::Year))
            .or_else(|| self.get_string(&ItemKey::OriginalReleaseDate))
    }
}

impl StructObject for AudioFileContext {
    fn get_field(&self, field: &str) -> Option<Value> {
        // TODO Add more tags
        match field {
            "args" | "arguments" => Some(self.arguments.clone().into()),
            "album" => self.get_string(&ItemKey::AlbumTitle),
            "albumartist" | "album_artist" => {
                self.get_string(&ItemKey::AlbumArtist)
            },
            "albumsort" | "album_sort" => {
                self.get_number(&ItemKey::AlbumTitleSortOrder)
            },
            "artist" => self.get_string(&ItemKey::TrackArtist),
            "disc_number" | "discnumber" | "disk_number" | "disknumber" => {
                self.get_number(&ItemKey::DiscNumber)
            },
            "genre" => self.get_string(&ItemKey::Genre),
            "title" => self.get_string(&ItemKey::TrackTitle),
            "date" | "year" => self.get_date(),
            "track_number" | "tracknumber" => {
                self.get_number(&ItemKey::TrackNumber)
            },
            _ => None,
        }
    }
}
