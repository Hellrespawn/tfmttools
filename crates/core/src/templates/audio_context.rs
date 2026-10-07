use convert_case::{Case, Casing};
use lofty::tag::ItemKey;
use path_template::Scalar;

use crate::action::FORBIDDEN_CHARACTERS;
use crate::audiofile::AudioFile;
use crate::item_keys::ItemKeys;
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

    fn safe(&mut self, key: ItemKey) -> Option<Scalar> {
        let raw = self.raw(key)?.to_owned();
        if raw != raw.trim() {
            let warning = Warning::WhitespaceInTag {
                file: self.audio.file().file_name().to_owned(),
                tag_name: format!("{key:?}")
                    .from_case(Case::Pascal)
                    .to_case(Case::Snake),
            };
            if !self.warnings.contains(&warning) {
                self.warnings.push(warning);
            }
        }
        let text = FORBIDDEN_CHARACTERS.iter().fold(
            raw.trim().to_owned(),
            |text, forbidden| {
                text.replace(
                    forbidden.char(),
                    forbidden.replacement().unwrap_or(""),
                )
            },
        );
        Some(Self::scalar(text.trim_end_matches('.').to_owned()))
    }

    fn scalar(text: String) -> Scalar {
        text.parse::<usize>()
            .ok()
            .and_then(|number| i64::try_from(number).ok())
            .map_or_else(|| Scalar::Text(text), Scalar::Integer)
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
        Some(Self::scalar(number.to_string()))
    }

    pub fn resolve(&mut self, name: &str) -> Option<Scalar> {
        if name == "date" {
            return self
                .safe(ItemKey::RecordingDate)
                .or_else(|| self.safe(ItemKey::Year))
                .or_else(|| self.safe(ItemKey::OriginalReleaseDate));
        }
        let key = ItemKeys::from_string(name).ok()?;
        match key {
            ItemKey::DiscTotal
            | ItemKey::TrackTotal
            | ItemKey::MovementTotal => self.number(key, true),
            ItemKey::DiscNumber
            | ItemKey::TrackNumber
            | ItemKey::MovementNumber => self.number(key, false),
            _ => self.safe(key),
        }
    }
}
