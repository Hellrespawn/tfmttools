use lofty::tag::ItemKey;
use tfmttools_picotmpl::Scalar;

use crate::audiofile::AudioFile;
use crate::error::{TFMTError, TFMTResult};
use crate::item_keys::{TagSource, canonical_tag_name, resolve_tag_name};
use crate::warning::Warning;

pub(super) struct AudioContext<'a> {
    audio: &'a AudioFile,
    warnings: Vec<Warning>,
}

impl<'a> AudioContext<'a> {
    pub fn new(audio: &'a AudioFile) -> Self {
        Self { audio, warnings: Vec::new() }
    }

    pub fn take_warnings(self) -> Vec<Warning> {
        self.warnings
    }

    fn raw(&self, key: ItemKey) -> Option<&str> {
        self.audio.tag().get_string(key)
    }

    fn text(&mut self, key: ItemKey) -> Option<Scalar> {
        let raw = self.raw(key)?.to_owned();
        if raw != raw.trim() {
            let warning = Warning::WhitespaceInTag {
                file: self.audio.file().file_name().to_owned(),
                tag_name: canonical_tag_name(key)
                    .expect("resolved tag has a canonical name")
                    .to_owned(),
            };
            if !self.warnings.contains(&warning) {
                self.warnings.push(warning);
            }
        }
        Some(Self::scalar(raw))
    }

    fn scalar(text: String) -> Scalar {
        text.parse::<usize>()
            .ok()
            .and_then(|number| i64::try_from(number).ok())
            .map_or_else(|| Scalar::Text(text), Scalar::Integer)
    }

    fn numeric_scalar(number: usize) -> Scalar {
        i64::try_from(number)
            .map_or_else(|_| Scalar::Text(number.to_string()), Scalar::Integer)
    }

    fn number(&self, key: ItemKey, total: bool) -> Option<Scalar> {
        let raw = self.raw(key)?;
        let (current, count) = if let Some((current, count)) =
            raw.split_once('/')
        {
            (current.parse::<usize>().ok()?, Some(count.parse::<usize>().ok()?))
        } else {
            (raw.parse::<usize>().ok()?, None)
        };
        let number = if total { count? } else { current };
        Some(Self::numeric_scalar(number))
    }

    fn total(&self, key: ItemKey, current_key: ItemKey) -> Option<Scalar> {
        match self.raw(key) {
            Some(raw) => self.number(key, raw.contains('/')),
            None => self.number(current_key, true),
        }
    }

    fn date_value(&mut self, key: ItemKey) -> Option<Scalar> {
        self.text(key).filter(|value| {
            match value {
                Scalar::Text(text) => !text.is_empty(),
                Scalar::Integer(_) => true,
            }
        })
    }

    pub fn resolve(&mut self, name: &str) -> TFMTResult<Option<Scalar>> {
        let Some(source) = resolve_tag_name(name) else { return Ok(None) };
        let keys: &[ItemKey] = match source {
            TagSource::DateFallback => {
                &[
                    ItemKey::RecordingDate,
                    ItemKey::Year,
                    ItemKey::OriginalReleaseDate,
                ]
            },
            TagSource::Item(key) => {
                match key {
                    ItemKey::TrackTotal => &[key, ItemKey::TrackNumber],
                    ItemKey::DiscTotal => &[key, ItemKey::DiscNumber],
                    ItemKey::MovementTotal => &[key, ItemKey::MovementNumber],
                    _ => &[key],
                }
            },
        };
        for key in keys {
            if let Some(raw) = self.raw(*key)
                && let Some(character) =
                    tfmttools_picotmpl::prohibited_character(raw)
            {
                return Err(TFMTError::Template(format!(
                    "Tag {name:?} contains prohibited character {character:?}"
                )));
            }
        }
        Ok(self.resolve_value(source))
    }

    fn resolve_value(&mut self, source: TagSource) -> Option<Scalar> {
        let key = match source {
            TagSource::DateFallback => {
                return self
                    .date_value(ItemKey::RecordingDate)
                    .or_else(|| self.date_value(ItemKey::Year))
                    .or_else(|| self.date_value(ItemKey::OriginalReleaseDate));
            },
            TagSource::Item(key) => key,
        };
        match key {
            ItemKey::DiscTotal => self.total(key, ItemKey::DiscNumber),
            ItemKey::TrackTotal => self.total(key, ItemKey::TrackNumber),
            ItemKey::MovementTotal => self.total(key, ItemKey::MovementNumber),
            ItemKey::DiscNumber
            | ItemKey::TrackNumber
            | ItemKey::MovementNumber => self.number(key, false),
            _ => self.text(key),
        }
    }
}
