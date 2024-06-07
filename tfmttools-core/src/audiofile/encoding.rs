use camino::Utf8PathBuf;
use id3::{Tag, TagLike};

use crate::error::TFMTResult;

pub fn convert_encoding_to_utf8(path: &Utf8PathBuf) -> TFMTResult<()> {
    let mut tag = Tag::read_from_path(path)?;

    let new_frames = tag
        .frames()
        .map(|f| f.clone().set_encoding(Some(id3::Encoding::UTF8)))
        .collect::<Vec<_>>();

    for frame in new_frames {
        tag.remove(frame.id());
        tag.add_frame(frame);
    }

    tag.write_to_path(path, tag.version())?;

    Ok(())
}
