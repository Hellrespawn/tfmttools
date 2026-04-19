use std::collections::HashMap;
use std::sync::LazyLock;

use convert_case::{Case, Casing};
use convert_case_extras::is_case;
use lofty::tag::ItemKey;

use crate::error::{TFMTError, TFMTResult};

pub struct ItemKeys;

impl ItemKeys {
    #[must_use]
    pub fn all() -> &'static [ItemKey] {
        &ITEM_KEYS
    }

    pub fn from_string(string: &str) -> TFMTResult<ItemKey> {
        STRING_TO_ITEM_KEY_MAP
            .get(string)
            .copied()
            .ok_or(TFMTError::UnknownTag(string.to_owned()))
    }
}

fn all_cases(pascal_case: &str) -> Vec<String> {
    CASES
        .into_iter()
        .map(|case| pascal_case.from_case(Case::Pascal).to_case(case))
        .collect()
}

static STRING_TO_ITEM_KEY_MAP: LazyLock<HashMap<String, ItemKey>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        for key in ITEM_KEYS {
            let pascal_case = format!("{key:?}");

            insert_case(&pascal_case, key, &mut map);
        }

        insert_case("Album", ItemKey::AlbumTitle, &mut map);
        insert_case("Artist", ItemKey::TrackArtist, &mut map);
        insert_case("AlbumSort", ItemKey::AlbumTitleSortOrder, &mut map);
        insert_case("DiskNumber", ItemKey::DiscNumber, &mut map);
        insert_case("Title", ItemKey::TrackTitle, &mut map);

        map
    });

fn insert_case(
    pascal_case: &str,
    key: ItemKey,
    map: &mut HashMap<String, ItemKey>,
) {
    let pascal_case = match pascal_case {
        "AppleId3v2ContentGroup" => "AppleId3V2ContentGroup".to_owned(),
        _ => pascal_case.to_owned(),
    };

    assert!(
        is_case(&pascal_case, Case::Pascal),
        "Key '{pascal_case}' is not pascal case!",
    );

    let all_cases = all_cases(&pascal_case);

    for (i, new_case) in all_cases.into_iter().enumerate() {
        if map.contains_key(new_case.as_str()) {
            if i == 0 {
                panic!(
                    "Key collision!\nAttempt: '{new_case}' => '{key:?}'\nExists: '{new_case}' => '{:?}'",
                    map.get(new_case.as_str()).unwrap()
                );
            } else {
                continue;
            }
        }

        map.insert(new_case, key);
    }
}

const CASES: [Case; 9] = [
    Case::Camel,
    Case::Cobol,
    Case::Flat,
    Case::Kebab,
    Case::Pascal,
    Case::UpperSnake,
    Case::Snake,
    Case::Train,
    Case::UpperFlat,
];

const ITEM_KEYS: [ItemKey; 100] = [
    // Titles
    ItemKey::AlbumTitle,
    ItemKey::SetSubtitle,
    ItemKey::ShowName,
    ItemKey::ContentGroup,
    ItemKey::TrackTitle,
    ItemKey::TrackSubtitle,
    // Original names
    ItemKey::OriginalAlbumTitle,
    ItemKey::OriginalArtist,
    ItemKey::OriginalLyricist,
    // Sorting
    ItemKey::AlbumTitleSortOrder,
    ItemKey::AlbumArtistSortOrder,
    ItemKey::TrackTitleSortOrder,
    ItemKey::TrackArtistSortOrder,
    ItemKey::ShowNameSortOrder,
    ItemKey::ComposerSortOrder,
    // People & Organizations
    ItemKey::AlbumArtist,
    ItemKey::TrackArtist,
    ItemKey::Arranger,
    ItemKey::Writer,
    ItemKey::Composer,
    ItemKey::Conductor,
    ItemKey::Director,
    ItemKey::Engineer,
    ItemKey::Lyricist,
    ItemKey::MixDj,
    ItemKey::MixEngineer,
    ItemKey::Performer,
    ItemKey::Producer,
    ItemKey::Publisher,
    ItemKey::Label,
    ItemKey::InternetRadioStationName,
    ItemKey::InternetRadioStationOwner,
    ItemKey::Remixer,
    // Counts & Indexes
    ItemKey::DiscNumber,
    ItemKey::DiscTotal,
    ItemKey::TrackNumber,
    ItemKey::TrackTotal,
    ItemKey::Popularimeter,
    ItemKey::ParentalAdvisory,
    // Dates
    // Recording date
    //
    // <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#date-10>
    ItemKey::RecordingDate,
    // Year
    ItemKey::Year,
    // Release date
    //
    // The release date of a podcast episode or any other kind of release.
    //
    // <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#release-date-10>
    ItemKey::ReleaseDate,
    // Original release date/year
    //
    // <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#original-release-date-1>
    // <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#original-release-year-1>
    ItemKey::OriginalReleaseDate,
    // Identifiers
    ItemKey::Isrc,
    ItemKey::Barcode,
    ItemKey::CatalogNumber,
    ItemKey::Work,
    ItemKey::Movement,
    ItemKey::MovementNumber,
    ItemKey::MovementTotal,
    //////////////////////////////////////////
    // MusicBrainz Identifiers
    // MusicBrainz Recording ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id21>
    ItemKey::MusicBrainzRecordingId,
    // MusicBrainz Track ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id24>
    ItemKey::MusicBrainzTrackId,
    // MusicBrainz Release ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id23>
    ItemKey::MusicBrainzReleaseId,
    // MusicBrainz Release Group ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#musicbrainz-release-group-id>
    ItemKey::MusicBrainzReleaseGroupId,
    // MusicBrainz Artist ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id17>
    ItemKey::MusicBrainzArtistId,
    // MusicBrainz Release Artist ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id22>
    ItemKey::MusicBrainzReleaseArtistId,
    // MusicBrainz Work ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#musicbrainz-work-id>
    ItemKey::MusicBrainzWorkId,
    //////////////////////////////////////////

    // Flags
    ItemKey::FlagCompilation,
    ItemKey::FlagPodcast,
    // File Information
    ItemKey::FileOwner,
    ItemKey::TaggingTime,
    ItemKey::Length,
    ItemKey::OriginalFileName,
    ItemKey::OriginalMediaType,
    // Encoder information
    ItemKey::EncodedBy,
    ItemKey::EncoderSoftware,
    ItemKey::EncoderSettings,
    ItemKey::EncodingTime,
    ItemKey::ReplayGainAlbumGain,
    ItemKey::ReplayGainAlbumPeak,
    ItemKey::ReplayGainTrackGain,
    ItemKey::ReplayGainTrackPeak,
    // URLs
    ItemKey::AudioFileUrl,
    ItemKey::AudioSourceUrl,
    ItemKey::CommercialInformationUrl,
    ItemKey::CopyrightUrl,
    ItemKey::TrackArtistUrl,
    ItemKey::RadioStationUrl,
    ItemKey::PaymentUrl,
    ItemKey::PublisherUrl,
    // Style
    ItemKey::Genre,
    ItemKey::InitialKey,
    ItemKey::Color,
    ItemKey::Mood,
    // Decimal BPM value with arbitrary precision
    //
    // Only read and written if the tag format supports a field for decimal BPM values
    // that are not restricted to integer values.
    //
    // Not supported by ID3v2 that restricts BPM values to integers in `TBPM`.
    ItemKey::Bpm,
    // Non-fractional BPM value with integer precision
    //
    // Only read and written if the tag format has a field for integer BPM values,
    // e.g. ID3v2 ([`TBPM` frame](https://github.com/id3/ID3v2.4/blob/516075e38ff648a6390e48aff490abed987d3199/id3v2.4.0-frames.txt#L376))
    // and MP4 (`tmpo` integer atom).
    ItemKey::IntegerBpm,
    // Legal
    ItemKey::CopyrightMessage,
    ItemKey::License,
    // Podcast
    ItemKey::PodcastDescription,
    ItemKey::PodcastSeriesCategory,
    ItemKey::PodcastUrl,
    ItemKey::PodcastGlobalUniqueId,
    ItemKey::PodcastKeywords,
    // Miscellaneous
    ItemKey::Comment,
    ItemKey::Description,
    ItemKey::Language,
    ItemKey::Script,
    ItemKey::Lyrics,
    // Vendor-specific
    ItemKey::AppleXid,
    ItemKey::AppleId3v2ContentGroup, // GRP1
];

// fn autocomplete(key: &ItemKey) {
//     match key {}
// }
