use camino::{Utf8Path, Utf8PathBuf};
use lofty::file::TaggedFileExt;
use lofty::tag::Tag;

use crate::error::{TFMTError, TFMTResult};
use crate::templates::Template;
use crate::util::normalize_separators;

#[derive(Clone)]
pub struct AudioFile {
    path: Utf8PathBuf,
    tag: Tag,
}

impl std::fmt::Debug for AudioFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioFile")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl AudioFile {
    pub const SUPPORTED_EXTENSIONS: [&'static str; 2] = ["mp3", "ogg"];

    pub fn new(path: Utf8PathBuf) -> TFMTResult<AudioFile> {
        let tagged_file = match lofty::read_from_path(&path) {
            Ok(tagged_file) => tagged_file,
            Err(err) => return Err(TFMTError::Lofty(path, err)),
        };

        match tagged_file.primary_tag() {
            Some(tag) => Ok(AudioFile { path, tag: tag.clone() }),
            None => Err(TFMTError::NoPrimaryTag(path)),
        }
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn extension(&self) -> &str {
        self.path.extension().as_ref().unwrap()
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    pub fn construct_target_path(
        &self,
        template: &Template,
        relative_path: &Utf8Path,
    ) -> TFMTResult<Utf8PathBuf> {
        let string = template.render(self)?;

        let string = normalize_separators(&string);

        let target_path =
            Utf8PathBuf::from(format!("{string}.{}", self.extension()));

        // If target_path is an absolute path, join will clobber the
        // relative_path, so this is always safe.
        Ok(relative_path.join(target_path))
    }

    pub fn tag_mut(&mut self) -> &mut Tag {
        &mut self.tag
    }

    pub fn path_predicate(path: &Utf8Path) -> bool {
        path.extension().is_some_and(|extension| {
            for supported_extension in AudioFile::SUPPORTED_EXTENSIONS {
                if extension == supported_extension {
                    return true;
                }
            }

            false
        })
    }
}
