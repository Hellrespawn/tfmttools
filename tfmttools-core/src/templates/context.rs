use std::collections::HashMap;

use convert_case::{Case, Casing};
use lofty::ItemKey;
use minijinja::value::StructObject;
use minijinja::Value;
use once_cell::sync::Lazy;
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

    fn get_value(&self, key: &ItemKey) -> Option<Value> {
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
        self.get_value(&ItemKey::RecordingDate)
            .or_else(|| self.get_value(&ItemKey::Year))
            .or_else(|| self.get_value(&ItemKey::OriginalReleaseDate))
    }
}

impl StructObject for AudioFileContext {
    fn get_field(&self, field: &str) -> Option<Value> {
        match field {
            "args" | "Args" | "ARGS" | "arguments" | "Arguments"
            | "ARGUMENTS" => Some(self.arguments.clone().into()),
            "date" | "Date" | "DATE" => self.get_date(),
            _ => {
                let key = &STRING_TO_ITEM_KEY_MAP.get(field)?;

                match key {
                    ItemKey::DiscNumber => self.get_current(key),
                    ItemKey::DiscTotal => self.get_total(key),
                    ItemKey::TrackNumber => self.get_current(key),
                    ItemKey::TrackTotal => self.get_total(key),
                    ItemKey::MovementNumber => self.get_current(key),
                    ItemKey::MovementTotal => self.get_total(key),
                    _ => self.get_value(key),
                }
            },
        }
    }
}

fn all_cases(pascal_case: &str) -> Vec<String> {
    CASES
        .into_iter()
        .map(|case| pascal_case.from_case(Case::Pascal).to_case(case))
        .collect()
}

static STRING_TO_ITEM_KEY_MAP: Lazy<HashMap<String, ItemKey>> =
    Lazy::new(|| {
        let mut map = HashMap::new();

        for key in ITEM_KEYS {
            let pascal_case = format!("{key:?}");

            insert_case(&pascal_case, &key, &mut map);
        }

        insert_case("Album", &ItemKey::AlbumTitle, &mut map);
        insert_case("Artist", &ItemKey::TrackArtist, &mut map);
        insert_case("AlbumSort", &ItemKey::AlbumTitleSortOrder, &mut map);
        insert_case("DiskNumber", &ItemKey::DiscNumber, &mut map);
        insert_case("Title", &ItemKey::TrackTitle, &mut map);

        trace!("STRING_TO_ITEM_KEY_MAP:\n{:#?}", map);

        map
    });

fn insert_case(
    pascal_case: &str,
    key: &ItemKey,
    map: &mut HashMap<String, ItemKey>,
) {
    let pascal_case = match pascal_case {
        "PodcastURL" => "PodcastUrl".to_owned(),
        "PodcastGlobalUniqueID" => "PodcastGlobalUniqueId".to_owned(),
        "AppleId3v2ContentGroup" => "AppleId3V2ContentGroup".to_owned(),
        _ => pascal_case.to_owned(),
    };

    if !pascal_case.is_case(Case::Pascal) {
        panic!("Key '{}' is not pascal case!", pascal_case)
    }

    let all_cases = all_cases(&pascal_case);

    for (i, new_case) in all_cases.into_iter().enumerate() {
        if map.contains_key(new_case.as_str()) {
            if i == 0 {
                panic!("Key collision!\nAttempt: '{new_case}' => '{key:?}'\nExists: '{new_case}' => '{:?}'", map.get(new_case.as_str()).unwrap() );
            } else {
                continue;
            }
        }

        map.insert(new_case, key.to_owned());
    }
}

const CASES: [Case; 9] = [
    Case::Camel,
    Case::Cobol,
    Case::Flat,
    Case::Kebab,
    Case::Pascal,
    Case::ScreamingSnake,
    Case::Snake,
    Case::Train,
    Case::UpperFlat,
];

const ITEM_KEYS: [ItemKey; 102] = [
    ItemKey::AlbumTitle,
    ItemKey::SetSubtitle,
    ItemKey::ShowName,
    ItemKey::ContentGroup,
    ItemKey::TrackTitle,
    ItemKey::TrackSubtitle,
    ItemKey::OriginalAlbumTitle,
    ItemKey::OriginalArtist,
    ItemKey::OriginalLyricist,
    ItemKey::AlbumTitleSortOrder,
    ItemKey::AlbumArtistSortOrder,
    ItemKey::TrackTitleSortOrder,
    ItemKey::TrackArtistSortOrder,
    ItemKey::ShowNameSortOrder,
    ItemKey::ComposerSortOrder,
    ItemKey::AlbumArtist,
    ItemKey::TrackArtist,
    ItemKey::Arranger,
    ItemKey::Writer,
    ItemKey::Composer,
    ItemKey::Conductor,
    ItemKey::Director,
    ItemKey::Engineer,
    ItemKey::InvolvedPeople,
    ItemKey::Lyricist,
    ItemKey::MixDj,
    ItemKey::MixEngineer,
    ItemKey::MusicianCredits,
    ItemKey::Performer,
    ItemKey::Producer,
    ItemKey::Publisher,
    ItemKey::Label,
    ItemKey::InternetRadioStationName,
    ItemKey::InternetRadioStationOwner,
    ItemKey::Remixer,
    ItemKey::DiscNumber,
    ItemKey::DiscTotal,
    ItemKey::TrackNumber,
    ItemKey::TrackTotal,
    ItemKey::Popularimeter,
    ItemKey::ParentalAdvisory,
    ItemKey::RecordingDate,
    ItemKey::Year,
    ItemKey::OriginalReleaseDate,
    ItemKey::Isrc,
    ItemKey::Barcode,
    ItemKey::CatalogNumber,
    ItemKey::Work,
    ItemKey::Movement,
    ItemKey::MovementNumber,
    ItemKey::MovementTotal,
    ItemKey::MusicBrainzRecordingId,
    ItemKey::MusicBrainzTrackId,
    ItemKey::MusicBrainzReleaseId,
    ItemKey::MusicBrainzReleaseGroupId,
    ItemKey::MusicBrainzArtistId,
    ItemKey::MusicBrainzReleaseArtistId,
    ItemKey::MusicBrainzWorkId,
    ItemKey::FlagCompilation,
    ItemKey::FlagPodcast,
    ItemKey::FileType,
    ItemKey::FileOwner,
    ItemKey::TaggingTime,
    ItemKey::Length,
    ItemKey::OriginalFileName,
    ItemKey::OriginalMediaType,
    ItemKey::EncodedBy,
    ItemKey::EncoderSoftware,
    ItemKey::EncoderSettings,
    ItemKey::EncodingTime,
    ItemKey::ReplayGainAlbumGain,
    ItemKey::ReplayGainAlbumPeak,
    ItemKey::ReplayGainTrackGain,
    ItemKey::ReplayGainTrackPeak,
    ItemKey::AudioFileUrl,
    ItemKey::AudioSourceUrl,
    ItemKey::CommercialInformationUrl,
    ItemKey::CopyrightUrl,
    ItemKey::TrackArtistUrl,
    ItemKey::RadioStationUrl,
    ItemKey::PaymentUrl,
    ItemKey::PublisherUrl,
    ItemKey::Genre,
    ItemKey::InitialKey,
    ItemKey::Color,
    ItemKey::Mood,
    ItemKey::Bpm,
    ItemKey::CopyrightMessage,
    ItemKey::License,
    ItemKey::PodcastDescription,
    ItemKey::PodcastSeriesCategory,
    ItemKey::PodcastURL,
    ItemKey::PodcastReleaseDate,
    ItemKey::PodcastGlobalUniqueID,
    ItemKey::PodcastKeywords,
    ItemKey::Comment,
    ItemKey::Description,
    ItemKey::Language,
    ItemKey::Script,
    ItemKey::Lyrics,
    ItemKey::AppleXid,
    ItemKey::AppleId3v2ContentGroup,
];

// fn autocomplete(key: &ItemKey) {
//     match key {}
// }
