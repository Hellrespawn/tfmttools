use camino::Utf8Path;
use lofty::config::ParseOptions;
use lofty::file::{AudioFile, FileType};
use lofty::id3::v2::Id3v2Tag;

use crate::{FsError, FsResult};

pub fn read_native_id3v2(
    path: &Utf8Path,
    format: FileType,
) -> FsResult<Option<Id3v2Tag>> {
    if !matches!(format, FileType::Mpeg | FileType::Wav | FileType::Aiff) {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path)?;
    let options = ParseOptions::new().read_properties(false);
    let tag = match format {
        FileType::Mpeg => {
            lofty::mpeg::MpegFile::read_from(&mut file, options)
                .map_err(|error| FsError::Lofty(path.to_owned(), error))?
                .id3v2()
                .cloned()
        },
        FileType::Wav => {
            lofty::iff::wav::WavFile::read_from(&mut file, options)
                .map_err(|error| FsError::Lofty(path.to_owned(), error))?
                .id3v2()
                .cloned()
        },
        FileType::Aiff => {
            lofty::iff::aiff::AiffFile::read_from(&mut file, options)
                .map_err(|error| FsError::Lofty(path.to_owned(), error))?
                .id3v2()
                .cloned()
        },
        _ => unreachable!(),
    };
    Ok(tag)
}
