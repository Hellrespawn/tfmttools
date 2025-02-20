use std::sync::Arc;

use lofty::tag::ItemKey;
use minijinja::Value;
use minijinja::value::Object;
use tracing::trace;

use crate::action::FORBIDDEN_CHARACTERS;
use crate::audiofile::AudioFile;
use crate::item_keys::ItemKeys;

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
            InterpolationMode::Safe => Self::remove_forbidden_characters(value),

            InterpolationMode::Strict => unimplemented!(),
        }
    }

    fn remove_forbidden_characters(value: String) -> String {
        let value = FORBIDDEN_CHARACTERS.iter().fold(
            value,
            |string, forbidden_character| {
                string.replace(
                    forbidden_character.char(),
                    forbidden_character.replacement().unwrap_or(""),
                )
            },
        );

        let value = value.trim_end_matches('.');

        value.to_owned()
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

    fn get_value_for_item_key(&self, key: &ItemKey) -> Option<Value> {
        let string = self.process_mode(self.read_value(key)?);

        if let Ok(number) = string.parse::<usize>() {
            Some(number.into())
        } else {
            Some(string.into())
        }
    }

    fn parse_number_with_optional_total(
        string: &str,
    ) -> Option<(usize, Option<usize>)> {
        if let Some((current, total)) = string.split_once('/') {
            let current = current.parse::<usize>().ok()?;
            let total = total.parse::<usize>().ok()?;

            Some((current, Some(total)))
        } else {
            let string = string.parse::<usize>().ok()?;

            Some((string, None))
        }
    }

    fn get_current(&self, key: &ItemKey) -> Option<Value> {
        let tag = self.read_value(key)?;

        let (current, _) = Self::parse_number_with_optional_total(&tag)?;

        Some(current.into())
    }

    fn get_total(&self, key: &ItemKey) -> Option<Value> {
        let tag = self.read_value(key)?;

        let total = Self::parse_number_with_optional_total(&tag)?.1?;

        Some(total.into())
    }

    fn get_date(&self) -> Option<Value> {
        self.get_value_for_item_key(&ItemKey::RecordingDate)
            .or_else(|| self.get_value_for_item_key(&ItemKey::Year))
            .or_else(|| {
                self.get_value_for_item_key(&ItemKey::OriginalReleaseDate)
            })
    }
}

impl Object for AudioFileContext {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let field = key.as_str()?;

        match field {
            "args" | "Args" | "ARGS" | "arguments" | "Arguments"
            | "ARGUMENTS" => Some(self.arguments.clone().into()),
            "date" | "Date" | "DATE" => self.get_date(),
            _ => {
                let key = ItemKeys::from_string(field).ok()?;

                match key {
                    ItemKey::DiscNumber => self.get_current(key),
                    ItemKey::DiscTotal => self.get_total(key),
                    ItemKey::TrackNumber => self.get_current(key),
                    ItemKey::TrackTotal => self.get_total(key),
                    ItemKey::MovementNumber => self.get_current(key),
                    ItemKey::MovementTotal => self.get_total(key),
                    _ => self.get_value_for_item_key(key),
                }
            },
        }
    }
}
