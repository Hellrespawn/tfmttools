use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use lofty::{Tag, TaggedFileExt};

use crate::templates::Template;
use crate::util::normalize_separators;

#[derive(Clone)]
pub struct AudioFile {
    path: Utf8PathBuf,
    extension: String,
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

    pub fn new(path: &Utf8Path) -> Result<AudioFile> {
        let path = path.to_owned();
        let tagged_file = lofty::read_from_path(&path)?;
        let tag = tagged_file
            .primary_tag()
            .ok_or_else(|| eyre!("Unable to read primary tag for '{}'", path))?
            .clone();

        let extension = path.extension().unwrap().to_string();

        Ok(AudioFile { path, extension, tag })
    }

    pub fn path_predicate(path: &Utf8Path) -> bool {
        path.extension().map_or(false, |extension| {
            for supported_extension in AudioFile::SUPPORTED_EXTENSIONS {
                if extension == supported_extension {
                    return true;
                }
            }

            false
        })
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn extension(&self) -> &str {
        self.extension.as_ref()
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    pub fn construct_target_path(
        &self,
        template: &Template,
        relative_path: &Utf8Path,
    ) -> Result<Utf8PathBuf> {
        let string = template.render(self)?;

        let string = normalize_separators(&string);

        let target_path =
            Utf8PathBuf::from(format!("{string}.{}", self.extension()));

        // If target_path is an absolute path, join will clobber the
        // relative_path, so this is always safe.
        Ok(relative_path.join(target_path))
    }
}
