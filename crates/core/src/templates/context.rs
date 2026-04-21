use std::sync::Arc;

use lofty::tag::ItemKey;
use minijinja::Value;
use minijinja::value::Object;
use tracing::trace;

use crate::action::FORBIDDEN_CHARACTERS;
use crate::audiofile::AudioFile;
use crate::item_keys::ItemKeys;

#[derive(Debug)]
pub struct AudioFileContext {
    audio_file: AudioFile,
    arguments: Vec<String>,
}

impl AudioFileContext {
    pub fn safe(audio_file: AudioFile, arguments: Vec<String>) -> Self {
        Self { audio_file, arguments }
    }

    fn safe_interpolation_value(value: String) -> String {
        Self::remove_forbidden_characters(value)
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

    fn read_raw_tag_value(&self, key: ItemKey) -> Option<String> {
        let tag = self
            .audio_file
            .tag()
            .get_string(key)
            .map(std::borrow::ToOwned::to_owned);

        trace!(
            "[{}][{:?}] => '{}'",
            self.audio_file.file().file_name(),
            key,
            if let Some(tag) = &tag { tag } else { "unknown" }
        );

        tag
    }

    fn read_safe_tag_value(&self, key: ItemKey) -> Option<String> {
        self.read_raw_tag_value(key).map(Self::safe_interpolation_value)
    }

    fn coerce_output_value(string: String) -> Value {
        if let Ok(number) = string.parse::<usize>() {
            number.into()
        } else {
            string.into()
        }
    }

    fn get_value_for_item_key(&self, key: ItemKey) -> Option<Value> {
        Some(Self::coerce_output_value(self.read_safe_tag_value(key)?))
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

    fn get_current(&self, key: ItemKey) -> Option<Value> {
        let tag = self.read_raw_tag_value(key)?;

        let (current, _) = Self::parse_number_with_optional_total(&tag)?;

        Some(current.into())
    }

    fn get_total(&self, key: ItemKey) -> Option<Value> {
        let tag = self.read_raw_tag_value(key)?;

        let total = Self::parse_number_with_optional_total(&tag)?.1?;

        Some(total.into())
    }

    fn get_date(&self) -> Option<Value> {
        self.get_value_for_item_key(ItemKey::RecordingDate)
            .or_else(|| self.get_value_for_item_key(ItemKey::Year))
            .or_else(|| {
                self.get_value_for_item_key(ItemKey::OriginalReleaseDate)
            })
    }
}

impl Object for AudioFileContext {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let field = key.as_str()?;
        let normalized_field = field.to_lowercase();

        match normalized_field.as_str() {
            "args" | "arguments" => Some(self.arguments.clone().into()),
            "date" => self.get_date(),
            _ => {
                let key = ItemKeys::from_string(&normalized_field).ok()?;

                match key {
                    ItemKey::DiscTotal
                    | ItemKey::TrackTotal
                    | ItemKey::MovementTotal => self.get_total(key),
                    ItemKey::TrackNumber
                    | ItemKey::DiscNumber
                    | ItemKey::MovementNumber => self.get_current(key),
                    _ => self.get_value_for_item_key(key),
                }
            },
        }
    }
}
