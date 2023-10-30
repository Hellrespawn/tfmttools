/// Common functions for reading audio file tags.
pub(crate) trait Tags: std::fmt::Debug + Send + Sync {
    /// The current `[AudioFile]`s album, if any.
    fn album(&self) -> Option<String>;

    /// The current `[AudioFile]`s album artist, if any.
    fn album_artist(&self) -> Option<String>;

    /// The current `[AudioFile]`s albumsort, if any.
    fn albumsort(&self) -> Option<String>;

    /// The current `[AudioFile]`s artist, if any.
    fn artist(&self) -> Option<String>;

    /// The current `[AudioFile]`s genre, if any.
    fn genre(&self) -> Option<String>;

    /// The current `[AudioFile]`s title, if any.
    fn title(&self) -> Option<String>;

    /// The current `[AudioFile]`s year, if any.
    fn year(&self) -> Option<String> {
        self.date()
    }

    /// The current `[AudioFile]`s date, if any.
    fn date(&self) -> Option<String>;

    /// The current `[AudioFile]`s track number, if any.
    fn track_number(&self) -> Option<String> {
        self.raw_track_number().map(|string| self.get_current(&string))
    }

    /// The current `[AudioFile]`s disc number, if any.
    fn disc_number(&self) -> Option<String> {
        self.raw_disc_number().map(|string| self.get_current(&string))
    }

    /// The current `[AudioFile]`s total amount of tracks, if any.
    fn total_track_number(&self) -> Option<String> {
        self.raw_track_number().and_then(|string| self.get_total(&string))
    }

    /// The current `[AudioFile]`s total amount of discs, if any.
    fn total_disc_number(&self) -> Option<String> {
        self.raw_disc_number().and_then(|string| self.get_total(&string))
    }

    /// The current `[AudioFile]`s raw disc number, if any.
    fn raw_disc_number(&self) -> Option<String>;

    /// The current `[AudioFile]`s raw track number, if any.
    fn raw_track_number(&self) -> Option<String>;

    /// Helper function that gets x from "x/y" or returns the string.
    fn get_current(&self, string: &str) -> String {
        if let Some((current, _)) = string.split_once('/') {
            current.to_owned()
        } else {
            string.to_owned()
        }
    }

    /// Helper function that gets y from "x/y"
    fn get_total(&self, string: &str) -> Option<String> {
        if let Some((_, total)) = string.split_once('/') {
            Some(total.to_owned())
        } else {
            None
        }
    }
}
