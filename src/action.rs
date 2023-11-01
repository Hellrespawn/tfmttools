use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use fs_err as fs;

pub(crate) struct Move {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
}

impl Move {
    pub(crate) fn new(source: Utf8PathBuf, target: Utf8PathBuf) -> Self {
        Self { source, target }
    }

    pub(crate) fn source(&self) -> &Utf8Path {
        &self.source
    }

    pub(crate) fn target(&self) -> &Utf8Path {
        &self.target
    }

    pub(crate) fn source_equals_target(&self) -> bool {
        self.source() == self.target()
    }

    pub(crate) fn source_differs_from_target(&self) -> bool {
        !self.source_equals_target()
    }

    pub(crate) fn apply(self, dry_run: bool) -> Result<Vec<Action>> {
        let mut actions = Self::create_directory_if_not_exists(
            dry_run,
            self.target()
                .parent()
                .expect("Move target should always be a file with a parent."),
        )?;

        if !dry_run {
            actions.extend(self.copy_or_move_file()?);
        }

        todo!()
    }

    fn copy_or_move_file(self) -> Result<Option<Action>> {
        if self.source_equals_target() {
            Ok(None)
        } else {
            if let Err(err) = fs::rename(self.source(), self.target()) {
                // Can't rename across filesystem boundaries. Checks for
                // the appropriate error and copies/deletes instead.
                // Error codes are correct on Windows 10 20H2 and Arch
                // Linux.
                // UPSTREAM Use ErrorKind::CrossesDevices when it enters stable

                if let Some(error_code) = err.raw_os_error() {
                    #[cfg(windows)]
                    let expected_error_code = 17;

                    #[cfg(unix)]
                    let expected_error_code = 18;

                    if expected_error_code == error_code {
                        fs::copy(self.source(), self.target())?;
                        fs::remove_file(self.source())?;
                    } else {
                        return Err(err.into());
                    }
                }
            }

            let action = Action::Move(self);

            Ok(Some(action))
        }
    }

    fn create_directory_if_not_exists(
        dry_run: bool,
        path: &Utf8Path,
    ) -> Result<Vec<Action>> {
        if path.is_dir() {
            Ok(Vec::new())
        } else {
            let mut actions = Vec::new();

            if let Some(parent) = path.parent() {
                actions.extend(Self::create_directory_if_not_exists(
                    dry_run, parent,
                )?);
            }

            if !dry_run {
                fs::create_dir(path)?;
            }

            actions.push(Action::MakeDir(path.to_owned()));

            Ok(actions)
        }
    }

    fn remove_empty_source_directories() -> Result<Vec<Action>> {
        todo!()
    }
}

pub(crate) enum Action {
    Move(Move),
    MakeDir(Utf8PathBuf),
    RemoveDir(Utf8PathBuf),
}
