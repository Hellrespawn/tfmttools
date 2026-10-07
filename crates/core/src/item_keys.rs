use lofty::tag::ItemKey;

use crate::error::{TFMTError, TFMTResult};

/// An audio tag reference, including the computed date fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagSource {
    Item(ItemKey),
    DateFallback,
}

/// Resolve an exact canonical name or an explicitly supported alias.
#[must_use]
pub fn resolve_tag_name(name: &str) -> Option<TagSource> {
    if name == "date" {
        return Some(TagSource::DateFallback);
    }
    TAG_NAMES.iter().chain(ALIASES).find_map(|&(candidate, key)| {
        (candidate == name).then_some(TagSource::Item(key))
    })
}

/// Parse a stored metadata key. Computed references are not writable tags.
pub fn parse_item_key(name: &str) -> TFMTResult<ItemKey> {
    match resolve_tag_name(name) {
        Some(TagSource::Item(key)) => Ok(key),
        _ => Err(TFMTError::UnknownTag(name.to_owned())),
    }
}

/// Return the canonical name used in diagnostics and persisted tag changes.
#[must_use]
pub fn canonical_tag_name(key: ItemKey) -> Option<&'static str> {
    TAG_NAMES
        .iter()
        .find_map(|&(name, candidate)| (candidate == key).then_some(name))
}

const ALIASES: &[(&str, ItemKey)] = &[
    ("album", ItemKey::AlbumTitle),
    ("artist", ItemKey::TrackArtist),
    ("album_sort", ItemKey::AlbumTitleSortOrder),
    ("disk_number", ItemKey::DiscNumber),
    ("title", ItemKey::TrackTitle),
];

const TAG_NAMES: &[(&str, ItemKey)] = &[
    // Titles
    ("album_title", ItemKey::AlbumTitle),
    ("set_subtitle", ItemKey::SetSubtitle),
    ("show_name", ItemKey::ShowName),
    ("content_group", ItemKey::ContentGroup),
    ("track_title", ItemKey::TrackTitle),
    ("track_subtitle", ItemKey::TrackSubtitle),
    // Original names
    ("original_album_title", ItemKey::OriginalAlbumTitle),
    ("original_artist", ItemKey::OriginalArtist),
    ("original_lyricist", ItemKey::OriginalLyricist),
    // Sorting
    ("album_title_sort_order", ItemKey::AlbumTitleSortOrder),
    ("album_artist_sort_order", ItemKey::AlbumArtistSortOrder),
    ("track_title_sort_order", ItemKey::TrackTitleSortOrder),
    ("track_artist_sort_order", ItemKey::TrackArtistSortOrder),
    ("show_name_sort_order", ItemKey::ShowNameSortOrder),
    ("composer_sort_order", ItemKey::ComposerSortOrder),
    // People & Organizations
    ("album_artist", ItemKey::AlbumArtist),
    ("track_artist", ItemKey::TrackArtist),
    ("arranger", ItemKey::Arranger),
    ("writer", ItemKey::Writer),
    ("composer", ItemKey::Composer),
    ("conductor", ItemKey::Conductor),
    ("director", ItemKey::Director),
    ("engineer", ItemKey::Engineer),
    ("lyricist", ItemKey::Lyricist),
    ("mix_dj", ItemKey::MixDj),
    ("mix_engineer", ItemKey::MixEngineer),
    ("performer", ItemKey::Performer),
    ("producer", ItemKey::Producer),
    ("publisher", ItemKey::Publisher),
    ("label", ItemKey::Label),
    ("internet_radio_station_name", ItemKey::InternetRadioStationName),
    ("internet_radio_station_owner", ItemKey::InternetRadioStationOwner),
    ("remixer", ItemKey::Remixer),
    // Counts & Indexes
    ("disc_number", ItemKey::DiscNumber),
    ("disc_total", ItemKey::DiscTotal),
    ("track_number", ItemKey::TrackNumber),
    ("track_total", ItemKey::TrackTotal),
    ("popularimeter", ItemKey::Popularimeter),
    ("parental_advisory", ItemKey::ParentalAdvisory),
    // Dates
    // Recording date
    //
    // <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#date-10>
    ("recording_date", ItemKey::RecordingDate),
    // Year
    ("year", ItemKey::Year),
    // Release date
    //
    // The release date of a podcast episode or any other kind of release.
    //
    // <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#release-date-10>
    ("release_date", ItemKey::ReleaseDate),
    // Original release date/year
    //
    // <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#original-release-date-1>
    // <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#original-release-year-1>
    ("original_release_date", ItemKey::OriginalReleaseDate),
    // Identifiers
    ("isrc", ItemKey::Isrc),
    ("barcode", ItemKey::Barcode),
    ("catalog_number", ItemKey::CatalogNumber),
    ("work", ItemKey::Work),
    ("movement", ItemKey::Movement),
    ("movement_number", ItemKey::MovementNumber),
    ("movement_total", ItemKey::MovementTotal),
    //////////////////////////////////////////
    // MusicBrainz Identifiers
    // MusicBrainz Recording ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id21>
    ("music_brainz_recording_id", ItemKey::MusicBrainzRecordingId),
    // MusicBrainz Track ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id24>
    ("music_brainz_track_id", ItemKey::MusicBrainzTrackId),
    // MusicBrainz Release ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id23>
    ("music_brainz_release_id", ItemKey::MusicBrainzReleaseId),
    // MusicBrainz Release Group ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#musicbrainz-release-group-id>
    ("music_brainz_release_group_id", ItemKey::MusicBrainzReleaseGroupId),
    // MusicBrainz Artist ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id17>
    ("music_brainz_artist_id", ItemKey::MusicBrainzArtistId),
    // MusicBrainz Release Artist ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#id22>
    ("music_brainz_release_artist_id", ItemKey::MusicBrainzReleaseArtistId),
    // MusicBrainz Work ID
    //
    // Textual representation of the UUID.
    //
    // Reference: <https://picard-docs.musicbrainz.org/en/appendices/tag_mapping.html#musicbrainz-work-id>
    ("music_brainz_work_id", ItemKey::MusicBrainzWorkId),
    //////////////////////////////////////////

    // Flags
    ("flag_compilation", ItemKey::FlagCompilation),
    ("flag_podcast", ItemKey::FlagPodcast),
    // File Information
    ("file_owner", ItemKey::FileOwner),
    ("tagging_time", ItemKey::TaggingTime),
    ("length", ItemKey::Length),
    ("original_file_name", ItemKey::OriginalFileName),
    ("original_media_type", ItemKey::OriginalMediaType),
    // Encoder information
    ("encoded_by", ItemKey::EncodedBy),
    ("encoder_software", ItemKey::EncoderSoftware),
    ("encoder_settings", ItemKey::EncoderSettings),
    ("encoding_time", ItemKey::EncodingTime),
    ("replay_gain_album_gain", ItemKey::ReplayGainAlbumGain),
    ("replay_gain_album_peak", ItemKey::ReplayGainAlbumPeak),
    ("replay_gain_track_gain", ItemKey::ReplayGainTrackGain),
    ("replay_gain_track_peak", ItemKey::ReplayGainTrackPeak),
    // URLs
    ("audio_file_url", ItemKey::AudioFileUrl),
    ("audio_source_url", ItemKey::AudioSourceUrl),
    ("commercial_information_url", ItemKey::CommercialInformationUrl),
    ("copyright_url", ItemKey::CopyrightUrl),
    ("track_artist_url", ItemKey::TrackArtistUrl),
    ("radio_station_url", ItemKey::RadioStationUrl),
    ("payment_url", ItemKey::PaymentUrl),
    ("publisher_url", ItemKey::PublisherUrl),
    // Style
    ("genre", ItemKey::Genre),
    ("initial_key", ItemKey::InitialKey),
    ("color", ItemKey::Color),
    ("mood", ItemKey::Mood),
    // Decimal BPM value with arbitrary precision
    //
    // Only read and written if the tag format supports a field for decimal BPM values
    // that are not restricted to integer values.
    //
    // Not supported by ID3v2 that restricts BPM values to integers in `TBPM`.
    ("bpm", ItemKey::Bpm),
    // Non-fractional BPM value with integer precision
    //
    // Only read and written if the tag format has a field for integer BPM values,
    // e.g. ID3v2 ([`TBPM` frame](https://github.com/id3/ID3v2.4/blob/516075e38ff648a6390e48aff490abed987d3199/id3v2.4.0-frames.txt#L376))
    // and MP4 (`tmpo` integer atom).
    ("integer_bpm", ItemKey::IntegerBpm),
    // Legal
    ("copyright_message", ItemKey::CopyrightMessage),
    ("license", ItemKey::License),
    // Podcast
    ("podcast_description", ItemKey::PodcastDescription),
    ("podcast_series_category", ItemKey::PodcastSeriesCategory),
    ("podcast_url", ItemKey::PodcastUrl),
    ("podcast_global_unique_id", ItemKey::PodcastGlobalUniqueId),
    ("podcast_keywords", ItemKey::PodcastKeywords),
    // Miscellaneous
    ("comment", ItemKey::Comment),
    ("description", ItemKey::Description),
    ("language", ItemKey::Language),
    ("script", ItemKey::Script),
    ("lyrics", ItemKey::Lyrics),
    // Vendor-specific
    ("apple_xid", ItemKey::AppleXid),
    ("apple_id3v2_content_group", ItemKey::AppleId3v2ContentGroup), // GRP1
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_names_round_trip_and_are_unique() {
        for &(name, key) in TAG_NAMES {
            assert_eq!(parse_item_key(name).unwrap(), key);
            assert_eq!(canonical_tag_name(key), Some(name));
            assert_eq!(
                TAG_NAMES.iter().filter(|&&(n, _)| n == name).count(),
                1
            );
        }
    }

    #[test]
    fn explicit_aliases_and_computed_date() {
        for &(name, key) in ALIASES {
            assert_eq!(resolve_tag_name(name), Some(TagSource::Item(key)));
        }
        assert_eq!(resolve_tag_name("date"), Some(TagSource::DateFallback));
        assert!(parse_item_key("date").is_err());
    }

    #[test]
    fn rejects_legacy_spellings() {
        for name in [
            "TrackArtist",
            "TRACK_ARTIST",
            "trackartist",
            "track-artist",
            "AlbumSort",
            "not_a_tag",
        ] {
            assert!(parse_item_key(name).is_err(), "{name}");
        }
    }
}
