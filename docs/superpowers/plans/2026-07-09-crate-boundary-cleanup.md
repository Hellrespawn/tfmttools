# Crate Boundary Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `tfmttools-core` genuinely I/O-free and give `tfmttools-fs` its own error type, so the crate boundary between "domain model / planning" (`core`) and "filesystem execution" (`fs`) is real rather than aspirational.

**Architecture:** Introduce `FsError`/`FsResult` in `fs` and migrate every function that touches disk (moves, copies, deletes, directory walking, checksums, tag reads/writes, template file loading) onto it, leaving `core`'s `TFMTError` holding only errors raised by pure, in-memory computation. Strip the implicit `.exists()`/`.is_file()`/`.is_dir()` stat calls out of `core`'s smart constructors and validation code, replacing them with either an infallible constructor (`Utf8File`/`Utf8Directory`) or an explicit value injected by the caller (existing-target sets, pre-filtered directory lists). `fs` gains small new modules (`verify`, `existing_paths`, `audiofile`) that supply the disk-touching half of what those `core` APIs used to do internally.

**Tech Stack:** Rust workspace (edition 2024), `thiserror` for error enums, `camino` for UTF-8 paths, `lofty` for audio tags, `minijinja` for templates, existing `cargo xtask test`/`lint` tasks for verification.

## Global Constraints

- Every task must leave `cargo build --workspace` and the relevant `cargo test -p <crate>` green before moving on — this is a behavior-preserving refactor, not a feature change.
- Do not change any user-visible CLI behavior, error message text (beyond what's structurally required by moving an error variant), or file formats.
- `core` must not gain a dependency on `fs`. All new cross-crate calls flow the existing direction: `history → core → fs → tfmt`.
- Reuse `CaseInsensitivePathSet` (already in `core::action`) wherever an "existing paths" collection is needed — don't invent a second set type.
- Run `cargo +nightly fmt --all` before each commit if rustfmt nightly is available in this environment; otherwise run `cargo fmt --all` and note it in the commit if formatting can't be verified.

---

## File Structure

New files:
- `crates/fs/src/error.rs` — `FsError`/`FsResult`, owns every disk-I/O-shaped error variant.
- `crates/fs/src/verify.rs` — `verify_file`/`verify_directory`, the stat-checking constructors that used to live inside `core::util::Utf8File::new`/`Utf8Directory::new`.
- `crates/fs/src/existing_paths.rs` — `existing_target_paths`, builds the `CaseInsensitivePathSet` that rename validation now takes as a parameter instead of calling `.exists()` itself.
- `crates/fs/src/audiofile.rs` — `read_audio_file`, the disk-reading half of what used to be `core::AudioFile::new`.
- `crates/core/src/templates/source.rs` — `parse_template_source`, the pure frontmatter-splitting/deprecation-warning logic moved out of `fs::TemplateLoader`.

Modified files (grouped by task below): `crates/fs/src/{fs_handler,checksum,path_iterator,template,lib}.rs`, `crates/fs/src/action/{handler,executor,rename_planner}.rs`, `crates/fs/Cargo.toml`, `crates/core/src/{util,audiofile,error,lib}.rs`, `crates/core/src/action/{rename_action,validation,validation/collisions}.rs`, `crates/core/src/templates/mod.rs`, `crates/core/Cargo.toml`, `crates/tfmt/src/cli/{args,options}.rs`, `crates/tfmt/src/ui/term.rs`, `crates/tfmt/src/commands/{validate,rename/discovery,rename/finish,rename/apply,rename/preview}.rs`.

---

### Task 1: Introduce `FsError`/`FsResult` in the `fs` crate

**Files:**
- Create: `crates/fs/src/error.rs`
- Modify: `crates/fs/src/lib.rs`
- Modify: `crates/fs/Cargo.toml`

**Interfaces:**
- Produces: `tfmttools_fs::error::{FsError, FsResult}` (re-exported at crate root as `tfmttools_fs::{FsError, FsResult}`), with variants `NotADirectory(Utf8PathBuf)`, `NotAFile(Utf8PathBuf)`, `UnexpectedMoveError(Utf8PathBuf, Utf8PathBuf, String)`, `FileTooLargeError(Utf8PathBuf)`, `Lofty(Utf8PathBuf, lofty::error::LoftyError)`, `Io(#[from] std::io::Error)`, `Ignore(#[from] ignore::Error)`, `Camino(#[from] camino::FromPathBufError)`, `Core(#[from] tfmttools_core::error::TFMTError)`.

This task is purely additive — nothing calls into `FsError` yet, so the workspace must still build and every existing test must still pass after this task.

- [ ] **Step 1: Add `thiserror` to `fs`'s dependencies**

Edit `crates/fs/Cargo.toml`, in the `[dependencies]` block, add (alphabetically, after `serde`):

```toml
thiserror = { workspace = true }
```

- [ ] **Step 2: Write `FsError`/`FsResult`**

Create `crates/fs/src/error.rs`:

```rust
use camino::Utf8PathBuf;
use thiserror::Error;
use tfmttools_core::error::TFMTError;

pub type FsResult<T = (), E = FsError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum FsError {
    #[error("Path exists but is not a directory: {0}")]
    NotADirectory(Utf8PathBuf),

    #[error("Path exists but is not a file: {0}")]
    NotAFile(Utf8PathBuf),

    #[error("Unexpected error while trying to move {0} to {1}: {2} ")]
    UnexpectedMoveError(Utf8PathBuf, Utf8PathBuf, String),

    #[error("File is too big for checksum: {0}")]
    FileTooLargeError(Utf8PathBuf),

    #[error("Error while reading file: {0}\n{1}")]
    Lofty(Utf8PathBuf, lofty::error::LoftyError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    Camino(#[from] camino::FromPathBufError),

    #[error(transparent)]
    Core(#[from] TFMTError),
}
```

- [ ] **Step 3: Export it from the crate root**

Edit `crates/fs/src/lib.rs`, add `error` to the module list and export list:

```rust
mod action;
mod checksum;
mod error;
mod file_or_name;
mod fs_handler;
mod path_iterator;
mod template;

pub use action::{ActionExecutor, ActionHandler};
pub use checksum::{get_file_checksum, get_path_checksum};
pub use error::{FsError, FsResult};
pub use file_or_name::FileOrName;
pub use fs_handler::{FsHandler, RemoveDirResult, get_longest_common_prefix};
pub use path_iterator::{PathIterator, PathIteratorOptions};
pub use template::TemplateLoader;
```

- [ ] **Step 4: Verify the workspace still builds**

Run: `cargo build --workspace`
Expected: succeeds with only a possible "unused" warning for the new `FsError`/`FsResult` (fine — they're consumed starting next task).

- [ ] **Step 5: Commit**

```bash
git add crates/fs/Cargo.toml crates/fs/src/error.rs crates/fs/src/lib.rs
git commit -m "fs: add FsError/FsResult (unused until callers migrate)"
```

---

### Task 2: Migrate `fs_handler.rs` to `FsResult`/`FsError`

**Files:**
- Modify: `crates/fs/src/fs_handler.rs`

**Interfaces:**
- Consumes: `FsError`, `FsResult` from Task 1.
- Produces: `FsHandler::{write, move_file, copy_file, remove_file, create_dir, remove_dir, remove_dir_all, remove_empty_subdirectories}` now return `std::io::Result`/`FsResult` instead of `std::io::Result`/`TFMTResult`. `handle_move_error` returns `FsResult<MoveFileResult>`.

`FsHandler` is the crate's lowest-level disk-mutation type — every one of its non-`write` methods already only ever raises `NotADirectory`, `UnexpectedMoveError`, or a passthrough `io::Error`, all of which now live on `FsError`.

- [ ] **Step 1: Confirm current tests pass before touching anything**

Run: `cargo test -p tfmttools-fs fs_handler`
Expected: all `fs_handler::tests::*` pass (baseline).

- [ ] **Step 2: Swap the error type**

Edit `crates/fs/src/fs_handler.rs`. Replace the import and every `TFMTResult`/`TFMTError` occurrence:

```rust
use std::io::ErrorKind;
use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use tfmttools_core::util::FSMode;
use tracing::trace;

use crate::error::{FsError, FsResult};
use crate::PathIterator;
use crate::path_iterator::PathIteratorOptions;
```

Then change every method signature and `Err(...)` construction from `TFMTResult`/`TFMTError` to `FsResult`/`FsError`, e.g.:

```rust
    pub fn move_file(
        &self,
        source: &Utf8Path,
        target: &Utf8Path,
    ) -> FsResult<MoveFileResult> {
```

```rust
    pub fn copy_file(
        &self,
        source: &Utf8Path,
        target: &Utf8Path,
    ) -> FsResult<CopyFileResult> {
```

```rust
    pub fn remove_file(&self, path: &Utf8Path) -> FsResult<RemoveFileResult> {
```

```rust
    pub fn create_dir(&self, path: &Utf8Path) -> FsResult<CreateDirResult> {
        if matches!(self.fs_mode, FSMode::DryRun) {
            Ok(CreateDirResult::DryRun)
        } else if path.is_dir() {
            Ok(CreateDirResult::Exists)
        } else if path.exists() {
            Err(FsError::NotADirectory(path.to_owned()))
        } else {
            fs_err::create_dir(path)?;

            Ok(CreateDirResult::Created)
        }
    }
```

```rust
    pub fn remove_dir(&self, path: &Utf8Path) -> FsResult<RemoveDirResult> {
```

```rust
    pub fn remove_dir_all(
        &self,
        path: &Utf8Path,
    ) -> FsResult<RemoveDirResult> {
```

```rust
    pub fn remove_empty_subdirectories(
        &self,
        path: &Utf8Path,
        recursion_depth: usize,
    ) -> FsResult<Vec<(Utf8PathBuf, RemoveDirResult)>> {
        let dirs = gather_subdirectories(path, recursion_depth)
            .into_iter()
            .rev()
            .map(|p| {
                let removed = self.remove_dir(&p)?;

                trace!("Removing dir: {p} => {removed:?}");

                Ok((p, removed))
            })
            .collect::<FsResult<Vec<_>>>()?;

        Ok(dirs)
    }
```

```rust
fn handle_move_error(
    source: &Utf8Path,
    target: &Utf8Path,
    err: &std::io::Error,
) -> FsResult<MoveFileResult> {
    if err.kind() == ErrorKind::CrossesDevices {
        fs_err::copy(source, target)?;
        fs_err::remove_file(source)?;
        Ok(MoveFileResult::CopiedAndRemoved)
    } else {
        Err(FsError::UnexpectedMoveError(
            source.to_owned(),
            target.to_owned(),
            err.to_string(),
        ))
    }
}
```

In the `#[cfg(test)] mod tests` block, replace `use tfmttools_core::error::TFMTError;` with `use crate::error::FsError;` and update the one match: `assert!(matches!(result, Err(TFMTError::UnexpectedMoveError(_, _, _))));` becomes `assert!(matches!(result, Err(FsError::UnexpectedMoveError(_, _, _))));`.

- [ ] **Step 3: Run tests**

Run: `cargo test -p tfmttools-fs fs_handler`
Expected: same tests pass as Step 1.

- [ ] **Step 4: Build the whole workspace to see downstream fallout**

Run: `cargo build --workspace 2>&1 | head -80`
Expected: compile errors in `crates/fs/src/action/{handler,executor}.rs` (they call `FsHandler` methods and are typed `TFMTResult`) — this is expected; Task 5 fixes it. Confirm there are no *other* unexpected errors (e.g. in `tfmt` directly) at this point — if there are, `FsHandler`'s public surface changed somewhere not accounted for here; investigate before continuing.

- [ ] **Step 5: Commit**

```bash
git add crates/fs/src/fs_handler.rs
git commit -m "fs: migrate FsHandler to FsResult/FsError"
```

---

### Task 3: Migrate `checksum.rs` to `FsResult`/`FsError`

**Files:**
- Modify: `crates/fs/src/checksum.rs`

**Interfaces:**
- Produces: `get_file_checksum`, `get_path_checksum` now return `FsResult<String>`.

- [ ] **Step 1: Read the current file to confirm the exact shape**

Run: `cat crates/fs/src/checksum.rs`
Expected: a `TFMTError::FileTooLargeError` construction and two functions typed `TFMTResult<String>`.

- [ ] **Step 2: Swap the error type**

Edit `crates/fs/src/checksum.rs`: replace `use tfmttools_core::error::{TFMTError, TFMTResult};` with `use crate::error::{FsError, FsResult};`, change both function return types from `TFMTResult<String>` to `FsResult<String>`, and change `TFMTError::FileTooLargeError(path.to_owned())` to `FsError::FileTooLargeError(path.to_owned())`.

- [ ] **Step 3: Run tests**

Run: `cargo test -p tfmttools-fs checksum`
Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add crates/fs/src/checksum.rs
git commit -m "fs: migrate checksum functions to FsResult/FsError"
```

---

### Task 4: Migrate `path_iterator.rs` to `FsResult`

**Files:**
- Modify: `crates/fs/src/path_iterator.rs`

**Interfaces:**
- Produces: `impl Iterator for PathIterator { type Item = FsResult<Utf8PathBuf>; }`

`PathIterator` wraps `ignore::Walk` and converts each entry's path via `Utf8PathBuf::try_from`, so it's the one place `ignore::Error` and `camino::FromPathBufError` genuinely originate — exactly the two `#[from]` variants that moved onto `FsError` in Task 1.

- [ ] **Step 1: Swap the error type**

Edit `crates/fs/src/path_iterator.rs`, replace `use tfmttools_core::error::TFMTResult;` with `use crate::error::FsResult;`, and change:

```rust
impl Iterator for PathIterator {
    type Item = FsResult<Utf8PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .0
            .next()?
            .map_err(std::convert::Into::into)
            .and_then(|d| Ok(d.into_path().try_into()?));

        Some(result)
    }
}
```

(the body is unchanged — only the `Item` type and the `use` changed; `map_err(Into::into)` and `?` now target `FsError`'s `From<ignore::Error>`/`From<camino::FromPathBufError>` impls instead of `TFMTError`'s).

- [ ] **Step 2: Build fs and see what breaks downstream**

Run: `cargo build -p tfmttools-fs 2>&1 | head -60`
Expected: compile errors in `crates/fs/src/fs_handler.rs` (`gather_subdirectories` collects `PathIterator` output via `.flatten()` — unaffected, `.flatten()` doesn't care about the error type) and in `crates/fs/src/template.rs` (`PathIterator::single_directory(...).flatten()` — also unaffected). The real breakage is anywhere something explicitly names `TFMTResult<Utf8PathBuf>` when consuming `PathIterator` — check `crates/fs/src/action/executor.rs` line ~131 (uses `.collect::<TFMTResult<Vec<_>>>()` but that's collecting `Action` results from `apply_rename_actions`, not `PathIterator` directly — leave as-is, Task 5 handles it) and confirm no `fs`-crate file other than `action/*` fails to compile. If something else fails, note it and fix inline before continuing.

- [ ] **Step 3: Run tests**

Run: `cargo test -p tfmttools-fs path_iterator`
Expected: pass (there may be no dedicated `path_iterator` tests — that's fine, confirm via `cargo test -p tfmttools-fs` that nothing regresses in modules that use `PathIterator`, e.g. `fs_handler`).

- [ ] **Step 4: Commit**

```bash
git add crates/fs/src/path_iterator.rs
git commit -m "fs: migrate PathIterator to FsResult"
```

---

### Task 5: Migrate `fs/action/{handler,executor}.rs` to `FsResult`/`FsError`, fix tfmt call sites

**Files:**
- Modify: `crates/fs/src/action/handler.rs`
- Modify: `crates/fs/src/action/executor.rs`
- Modify: `crates/tfmt/src/commands/rename/finish.rs`
- Modify: `crates/tfmt/src/commands/rename/apply.rs` (verify only — likely no change needed)

**Interfaces:**
- Consumes: `FsError`, `FsResult` from Task 1.
- Produces: `ActionHandler::{rename, apply, undo, redo}` and `ActionExecutor::{apply_rename_actions, apply_actions, remove_directories}` now surface `FsResult`/`FsError` instead of `TFMTResult`/`TFMTError`. `NoPrimaryTag` (a `core`-owned domain check, unaffected) still flows through automatically via `FsError`'s `#[from] TFMTError`.

- [ ] **Step 1: Confirm baseline tests pass**

Run: `cargo test -p tfmttools-fs action`
Expected: all `action::*::tests::*` pass.

- [ ] **Step 2: Swap `handler.rs`'s error type**

Edit `crates/fs/src/action/handler.rs`. Change the import block:

```rust
use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile as LoftyAudioFile, TaggedFileExt};
use lofty::id3::v2::{Frame, Id3v2Tag};
use lofty::tag::{ItemKey, ItemValue, TagExt, TagItem, TagType};
use tfmttools_core::action::{
    Action, RenameAction, TagValueChange, TagValueKind,
};
use tfmttools_core::item_keys::ItemKeys;
use tfmttools_core::util::{MoveMode, Utf8PathExt};
use tracing::trace;

use crate::error::{FsError, FsResult};
use crate::fs_handler::{FsHandler, MoveFileResult};
```

Change every `TFMTResult`/`TFMTResult<Vec<Action>>` return type to `FsResult`/`FsResult<Vec<Action>>` on `rename`, `apply`, `undo`, `redo`, `apply_forward`, `apply_tag_changes`, `apply_tag_change`, `tag_with_encoding_changes`.

Update the two explicit `Lofty` constructions and the `NoPrimaryTag` construction in `apply_tag_changes`:

```rust
fn apply_tag_changes(
    path: &camino::Utf8Path,
    changes: &[TagValueChange],
    direction: TagChangeDirection,
) -> FsResult {
    let mut tagged_file = lofty::read_from_path(path)
        .map_err(|err| FsError::Lofty(path.to_owned(), err))?;
    let tag = tagged_file.primary_tag_mut().ok_or_else(|| {
        tfmttools_core::error::TFMTError::NoPrimaryTag(path.to_owned())
    })?;

    for change in changes {
        apply_tag_change(tag, change, direction)?;
    }
    let id3v2_tag_with_encoding_changes =
        tag_with_encoding_changes(tag, changes, direction)?;

    tagged_file.save_to_path(path, WriteOptions::default())
        .map_err(|err| FsError::Lofty(path.to_owned(), err))?;

    if let Some(id3v2_tag) = id3v2_tag_with_encoding_changes {
        id3v2_tag.save_to_path(path, WriteOptions::default())
            .map_err(|err| FsError::Lofty(path.to_owned(), err))?;
    }

    Ok(())
}
```

Note `NoPrimaryTag` deliberately stays as `tfmttools_core::error::TFMTError::NoPrimaryTag(...)` — it's a domain check on an already-loaded `TaggedFile`, not an I/O failure, and it converts to `FsError` automatically via `?` (no `.map_err` needed) because of `FsError`'s `#[from] TFMTError`.

`apply_tag_change`, `tag_with_encoding_changes` return `FsResult`/`FsResult<Option<Id3v2Tag>>` — their bodies are unchanged (they only construct `TFMTError` via `ItemKeys::from_string`, which stays a core error and auto-converts).

In the `#[cfg(test)] mod tests` block, `use tfmttools_core::error::TFMTResult;` becomes `use crate::error::FsResult;`, and `.collect::<TFMTResult<Vec<_>>>()` calls (if any appear there) become `.collect::<FsResult<Vec<_>>>()`.

- [ ] **Step 3: Swap `executor.rs`'s error type**

Edit `crates/fs/src/action/executor.rs`. Change the import:

```rust
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::util::{MoveMode, Utf8Directory, Utf8PathExt};

use super::PlannedAction;
use super::handler::ActionHandler;
use super::rename_planner::RenamePlanner;
use crate::error::FsResult;
use crate::fs_handler::FsHandler;
```

Change the two public method signatures:

```rust
    pub fn apply_rename_actions(
        &self,
        rename_actions: Vec<RenameAction>,
    ) -> impl Iterator<Item = FsResult<Action>> + '_ {
```

```rust
    pub fn apply_actions(
        &self,
        actions: impl IntoIterator<Item = Action>,
    ) -> FsResult<Vec<Action>> {
```

```rust
    pub fn remove_directories(
        &self,
        directories: Vec<Utf8Directory>,
    ) -> FsResult<Vec<Action>> {
```

In the `#[cfg(test)] mod tests` block, replace `use tfmttools_core::error::TFMTResult;` with `use crate::error::FsResult;` and update `.collect::<TFMTResult<Vec<_>>>()` → `.collect::<FsResult<Vec<_>>>()` in the `apply_actions` test helper.

- [ ] **Step 4: Run fs's action tests**

Run: `cargo test -p tfmttools-fs action`
Expected: pass.

- [ ] **Step 5: Fix `tfmt`'s call sites that name the error type explicitly**

Run: `grep -n "TFMTResult" crates/tfmt/src/commands/rename/finish.rs`
Expected: two hits — `move_files`'s `.collect::<TFMTResult<_>>()?` and `discover_remaining_items`'s `.collect::<TFMTResult<Vec<_>>>()?` (the latter collects `PathIterator` output, already `FsResult` since Task 4).

Edit `crates/tfmt/src/commands/rename/finish.rs`: replace `use tfmttools_fs::{ActionExecutor, PathIterator, PathIteratorOptions, get_file_checksum, get_longest_common_prefix};` with:

```rust
use tfmttools_fs::{
    ActionExecutor, FsResult, PathIterator, PathIteratorOptions,
    get_file_checksum, get_longest_common_prefix,
};
```

and remove `tfmttools_core::error::TFMTResult` from its `use tfmttools_core::error::TFMTResult;`-style import if that line becomes unused (check with `cargo build -p tfmt` after the edit — the file currently imports `TFMTResult` via `use tfmttools_core::error::TFMTResult;`; keep that import only if `TFMTResult` is still named elsewhere in the file, otherwise delete the line). Change both turbofish sites:

```rust
    let remaining = PathIterator::new(&options)
        .filter_ok(|path| !protected_paths.contains(path))
        .collect::<FsResult<Vec<_>>>()?;
```

```rust
    Ok(executor
        .apply_rename_actions(rename_actions)
        .collect::<FsResult<_>>()?)
```

- [ ] **Step 6: Confirm `apply.rs` needs no change**

Run: `grep -n "TFMTResult\|TFMTError" crates/tfmt/src/commands/rename/apply.rs`
Expected: no output — `apply.rs` consumes `ActionExecutor::apply_rename_actions`'s `Result<Action, _>` items via `if let Ok(action) = result` and `Err(err) => ... err.into()` without ever naming the concrete error type, so it needs no source change (the `Into<color_eyre::Report>` blanket impl covers `FsError` the same way it covered `TFMTError`).

- [ ] **Step 7: Build and test the whole workspace**

Run: `cargo build --workspace 2>&1 | head -80`
Expected: succeeds, or shows only unrelated fallout you haven't reached yet — compare against Task 2 Step 4's baseline list.

Run: `cargo test -p tfmt --bin tfmt`
Expected: passes.

- [ ] **Step 8: Commit**

```bash
git add crates/fs/src/action/handler.rs crates/fs/src/action/executor.rs crates/tfmt/src/commands/rename/finish.rs
git commit -m "fs+tfmt: migrate ActionHandler/ActionExecutor to FsResult/FsError"
```

---

### Task 6: Make `Utf8File`/`Utf8Directory` infallible in `core`; add `fs::verify_file`/`verify_directory`

**Files:**
- Modify: `crates/core/src/util.rs`
- Modify: `crates/core/src/action/rename_action.rs` (test helpers only)
- Modify: `crates/core/src/action/validation.rs` (test helpers only)
- Modify: `crates/fs/src/action/handler.rs` (test helper only)
- Modify: `crates/fs/src/action/executor.rs` (test helper only)
- Create: `crates/fs/src/verify.rs`
- Modify: `crates/fs/src/lib.rs`
- Modify: `crates/tfmt/src/cli/args.rs`
- Modify: `crates/tfmt/src/cli/options.rs`
- Modify: `crates/tfmt/src/ui/term.rs`

**Interfaces:**
- Produces: `Utf8File::new(path) -> Utf8File` and `Utf8Directory::new(path) -> Utf8Directory` (both now infallible, replacing the old fallible `new` + `new_unchecked` pair with a single infallible constructor). `Utf8File::new_unchecked`/`Utf8Directory::new_unchecked` are removed (folded into `new`).
- Produces: `tfmttools_fs::{verify_file, verify_directory}` — `fn verify_file(path: impl AsRef<Utf8Path>) -> FsResult<Utf8File>`, `fn verify_directory(path: impl AsRef<Utf8Path>) -> FsResult<Utf8Directory>`, doing the stat check the old `core` constructors used to do.

This is the core "purity" step: `core::util` currently does `path.is_dir() || !path.exists()` and `path.is_file() || !path.exists()` inside its constructors. After this task, `core`'s types carry no such check, and any caller that needs "does this path exist and is it the right kind" calls into `fs`.

- [ ] **Step 1: Confirm baseline**

Run: `cargo test -p tfmttools-core util && cargo test -p tfmttools-core action`
Expected: pass.

- [ ] **Step 2: Make the constructors infallible in `core`**

Edit `crates/core/src/util.rs`. Remove the `TFMTError`/`TFMTResult` import (check at the end of this task whether `crate::error::{TFMTError, TFMTResult}` is still used elsewhere in the file — it won't be) and change:

```rust
impl Utf8Directory {
    #[must_use]
    pub fn new(path: impl AsRef<Utf8Path>) -> Self {
        Self(path.as_ref().to_owned())
    }

    #[must_use]
    pub fn ancestors(self) -> Vec<Utf8Directory> {
        self.0.ancestors().map(Utf8Directory::new).collect()
    }

    #[must_use]
    pub fn join(&self, path: impl AsRef<Utf8Path>) -> Utf8Directory {
        Utf8Directory::new(self.as_path().join(path))
    }

    #[must_use]
    pub fn join_file(&self, path: impl AsRef<Utf8Path>) -> Utf8File {
        Utf8File::new(self.as_path().join(path))
    }
}
```

Note `join`/`join_file` no longer return `TFMTResult` either — they were only fallible because `Utf8Directory::new`/`Utf8File::new` were. Same for `Utf8File`:

```rust
impl Utf8File {
    #[must_use]
    pub fn new(path: impl AsRef<Utf8Path>) -> Self {
        Self(path.as_ref().to_owned())
    }

    #[must_use]
    pub fn parent(&self) -> Utf8Directory {
        let path = self.0.parent().expect("Utf8File should have parent");

        Utf8Directory::new(path)
    }

    ...
}
```

(`components`, `extension`, `file_name` are unchanged.)

- [ ] **Step 3: Fix every caller in `core` that now gets a bare value instead of a `Result`**

Run: `grep -rn "Utf8File::new\|Utf8Directory::new\|\.join(\|\.join_file(" crates/core/src | grep -v test`
Expected: `crates/core/src/audiofile.rs:28` (`Utf8File::new(&path)?`), `crates/core/src/action/rename_action.rs` (`RenameAction::from_path_bufs`).

Edit `crates/core/src/audiofile.rs` line 28: `let file = Utf8File::new(&path)?;` → `let file = Utf8File::new(&path);` (drop the `?`; Task 7 removes this whole method anyway, but keep the crate compiling at every step).

Edit `crates/core/src/action/rename_action.rs`, `from_path_bufs`:

```rust
    pub fn from_path_bufs(
        source: Utf8PathBuf,
        target: Utf8PathBuf,
    ) -> Self {
        Self { source: Utf8File::new(source), target: Utf8File::new(target) }
    }
```

Drop the `-> TFMTResult<Self>` return type and the now-unused `crate::error::TFMTResult` import if nothing else in the file needs it. Search for callers of `from_path_bufs`:

Run: `grep -rn "from_path_bufs" --include="*.rs" crates`
Expected: if any caller does `RenameAction::from_path_bufs(a, b)?`, drop the `?` there too.

- [ ] **Step 4: Update test helpers in `core` that called the old fallible constructors**

`crates/core/src/action/rename_action.rs`'s `#[cfg(test)] mod test` currently does e.g. `.map(Utf8File::new).collect::<TFMTResult<_>>().unwrap();` — since `Utf8File::new` no longer returns a `Result`, this becomes:

```rust
        let paths: Vec<Utf8File> =
            paths.iter().map(Utf8File::new).collect();
```

(remove the `TFMTResult` turbofish and trailing `.unwrap()`; do the same for the `Utf8Directory::new` reference-list construction in the same tests, replacing `.map(Utf8Directory::new).collect::<TFMTResult<Vec<_>>>().unwrap()` with `.map(Utf8Directory::new).collect::<Vec<_>>()`).

`crates/core/src/action/validation.rs`'s `#[cfg(test)] mod test` calls `Utf8File::new("...").unwrap()` roughly 50 times. Run this mechanical substitution:

Run: `sed -i -E 's/Utf8File::new\((("[^"]*")|(format!\([^)]*\)))\)\.unwrap\(\)/Utf8File::new(\1)/g' crates/core/src/action/validation.rs`
Expected: no output (silent success). Verify with `grep -c "\.unwrap()" crates/core/src/action/validation.rs` before/after to confirm the count dropped by the number of `Utf8File::new(...).unwrap()` occurrences and nothing else was touched.

Run: `grep -n "Utf8File::new" crates/core/src/action/validation.rs`
Expected: every remaining occurrence has no trailing `.unwrap()`. Manually fix any the `sed` pattern missed (e.g. the two `format!(...)` cases at the "path too long" test — verify those specifically by eye since they span the pattern's parenthesis-matching limits).

- [ ] **Step 5: Build `core` in isolation**

Run: `cargo build -p tfmttools-core && cargo test -p tfmttools-core`
Expected: builds and all tests pass. If `sed` left a malformed line, fix it by hand now.

- [ ] **Step 6: Add `fs::verify_file`/`verify_directory`**

Create `crates/fs/src/verify.rs`:

```rust
use camino::Utf8Path;
use tfmttools_core::util::{Utf8Directory, Utf8File};

use crate::error::{FsError, FsResult};

pub fn verify_file(path: impl AsRef<Utf8Path>) -> FsResult<Utf8File> {
    let path = path.as_ref();

    if path.is_file() || !path.exists() {
        Ok(Utf8File::new(path))
    } else {
        Err(FsError::NotAFile(path.to_owned()))
    }
}

pub fn verify_directory(
    path: impl AsRef<Utf8Path>,
) -> FsResult<Utf8Directory> {
    let path = path.as_ref();

    if path.is_dir() || !path.exists() {
        Ok(Utf8Directory::new(path))
    } else {
        Err(FsError::NotADirectory(path.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use camino::Utf8PathBuf;
    use color_eyre::Result;

    use super::*;

    #[test]
    fn verify_file_accepts_missing_path() -> Result<()> {
        let path = Utf8PathBuf::from("/does/not/exist.mp3");

        assert!(verify_file(&path).is_ok());

        Ok(())
    }

    #[test]
    fn verify_file_rejects_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let path = Utf8PathBuf::try_from(temp_dir.path().to_owned())?;

        let error = verify_file(&path).unwrap_err();

        assert!(matches!(error, FsError::NotAFile(_)));

        Ok(())
    }

    #[test]
    fn verify_directory_rejects_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let path = Utf8PathBuf::try_from(temp_dir.path().join("f.mp3"))?;
        fs_err::write(&path, "x")?;

        let error = verify_directory(&path).unwrap_err();

        assert!(matches!(error, FsError::NotADirectory(_)));

        Ok(())
    }
}
```

Edit `crates/fs/src/lib.rs` to add the module:

```rust
mod action;
mod checksum;
mod error;
mod file_or_name;
mod fs_handler;
mod path_iterator;
mod template;
mod verify;

pub use action::{ActionExecutor, ActionHandler};
pub use checksum::{get_file_checksum, get_path_checksum};
pub use error::{FsError, FsResult};
pub use file_or_name::FileOrName;
pub use fs_handler::{FsHandler, RemoveDirResult, get_longest_common_prefix};
pub use path_iterator::{PathIterator, PathIteratorOptions};
pub use template::TemplateLoader;
pub use verify::{verify_directory, verify_file};
```

- [ ] **Step 7: Run the new tests**

Run: `cargo test -p tfmttools-fs verify`
Expected: 3 new tests pass.

- [ ] **Step 8: Fix `fs`-internal test helpers that relied on the old fallible constructors**

Run: `grep -n "Utf8File::new" crates/fs/src/action/handler.rs crates/fs/src/action/executor.rs`
Expected: one hit each, in test helper functions constructing a `RenameAction` from strings.

Edit `crates/fs/src/action/handler.rs`'s test `rename_action` helper:

```rust
    fn rename_action(
        source: &str,
        target: &str,
    ) -> Result<(RenameAction, Utf8PathBuf, Utf8PathBuf)> {
        let source = Utf8PathBuf::from(source);
        let target = Utf8PathBuf::from(target);
        let action =
            RenameAction::new(Utf8File::new(&source), Utf8File::new(&target));

        Ok((action, source, target))
    }
```

Apply the same edit to `crates/fs/src/action/executor.rs`'s equivalent `rename_action` helper (drop the trailing `?` on each `Utf8File::new(...)`  call; the function can keep returning `Result<...>` since other parts of the helper may still be fallible — check each file; if nothing else in the helper is fallible after this edit, simplify the return type accordingly, but only if it compiles cleanly either way).

- [ ] **Step 9: Update `tfmt`'s CLI path-validation call sites**

Run: `grep -n "Utf8Directory::new\|Utf8File::new" crates/tfmt/src/cli/args.rs crates/tfmt/src/cli/options.rs crates/tfmt/src/ui/term.rs`
Expected: 1 hit in `args.rs`, 6 in `options.rs`, 1 in `term.rs` — these are all CLI-supplied paths that must genuinely exist and be the right kind, so they switch to the new verifying constructors.

Edit `crates/tfmt/src/cli/args.rs`: add `tfmttools_fs::verify_directory` to the imports and change:

```rust
                let template_directory = list_templates_args
                    .custom_template_directory
                    .map(tfmttools_fs::verify_directory)
                    .unwrap_or(Ok(app_options.config_directory().to_owned()))?;
```

Wait — `verify_directory` returns `FsResult<Utf8Directory>` while the branch's `Ok(app_options.config_directory().to_owned())` needs a matching type. Use `.transpose()`-free form by matching the existing pattern exactly:

```rust
                let template_directory = match list_templates_args
                    .custom_template_directory
                {
                    Some(path) => tfmttools_fs::verify_directory(path)?,
                    None => app_options.config_directory().to_owned(),
                };
```

Edit `crates/tfmt/src/cli/options.rs`. Add `use tfmttools_fs::{verify_directory, verify_file};` and change each of the six call sites from `Utf8Directory::new(...)`/`Utf8File::new(...)` to `verify_directory(...)`/`verify_file(...)`, keeping every `?` already present (they were already propagating a `Result` before this change, so the call shape is unchanged — only the callee and its concrete error type change):

```rust
    pub fn default_application_dir() -> Result<Utf8Directory> {
        let path = dirs::home_dir()
            .ok_or(eyre!("Unable to determine home directory."))?
            .join(format!(".{}", crate::PKG_NAME));

        let utf8_path = Utf8PathBuf::try_from(path)?;

        Ok(verify_directory(utf8_path)?)
    }

    pub fn history_file_path(&self) -> Result<Utf8File> {
        let filename = format!("{}.hist", crate::PKG_NAME);
        let path = self.config_directory.as_path().join(filename);

        Ok(verify_file(path)?)
    }

    fn path_or_default(path: Option<&Utf8Path>) -> Result<Utf8Directory> {
        if let Some(path) = path {
            Ok(verify_directory(path)?)
        } else {
            Ok(Self::default_application_dir()?)
        }
    }
```

Apply the equivalent `Utf8Directory::new(...)` → `verify_directory(...)` swap at the remaining four call sites around lines 239, 260, 267, 274/276 (input_directory, template_directory, bin_directory) — read each surrounding function first to keep the `?`/error-propagation shape identical, only swapping the callee.

Edit `crates/tfmt/src/ui/term.rs` line 33: `Ok(Utf8Directory::new(utf8_path)?)` → `Ok(verify_directory(utf8_path)?)`, adding the `tfmttools_fs::verify_directory` import.

- [ ] **Step 10: Build and test the whole workspace**

Run: `cargo build --workspace 2>&1 | head -100`
Expected: succeeds (modulo fallout you haven't reached in later tasks — compare against the running baseline).

Run: `cargo test --workspace --exclude tfmt && cargo test -p tfmt --bin tfmt`
Expected: pass.

- [ ] **Step 11: Commit**

```bash
git add crates/core/src/util.rs crates/core/src/audiofile.rs crates/core/src/action/rename_action.rs crates/core/src/action/validation.rs crates/fs/src/verify.rs crates/fs/src/lib.rs crates/fs/src/action/handler.rs crates/fs/src/action/executor.rs crates/tfmt/src/cli/args.rs crates/tfmt/src/cli/options.rs crates/tfmt/src/ui/term.rs
git commit -m "core+fs+tfmt: make Utf8File/Utf8Directory infallible, add fs::verify_*"
```

---

### Task 7: Split `AudioFile::new` into a pure core constructor + `fs::read_audio_file`

**Files:**
- Modify: `crates/core/src/audiofile.rs`
- Create: `crates/fs/src/audiofile.rs`
- Modify: `crates/fs/src/lib.rs`
- Modify: `crates/tfmt/src/commands/rename/discovery.rs`
- Modify: `crates/tfmt/src/commands/validate.rs`

**Interfaces:**
- Consumes: `Utf8File::new` (infallible, from Task 6), `FsError`/`FsResult` (Task 1).
- Produces: `core::AudioFile::from_tagged_file(file: Utf8File, tagged_file: &lofty::file::TaggedFile) -> TFMTResult<AudioFile>` (pure — no disk access). `tfmttools_fs::read_audio_file(path: Utf8PathBuf) -> FsResult<AudioFile>` (does the actual `lofty::read_from_path` call).

- [ ] **Step 1: Confirm baseline**

Run: `cargo test -p tfmttools-core audiofile`
Expected: pass (there may be no dedicated tests for `AudioFile` today — confirm by running `cargo test -p tfmttools-core` broadly and noting the baseline is green).

- [ ] **Step 2: Replace `AudioFile::new` with a pure constructor**

Edit `crates/core/src/audiofile.rs`. Change the imports and `new`:

```rust
use camino::{Utf8Path, Utf8PathBuf};
use lofty::file::{TaggedFile, TaggedFileExt};
use lofty::tag::Tag;

use crate::error::{TFMTError, TFMTResult};
use crate::templates::Template;
use crate::util::{Utf8Directory, Utf8File, normalize_separators};
use crate::warning::Warning;

#[derive(Clone)]
pub struct AudioFile {
    file: Utf8File,
    tag: Tag,
}

impl std::fmt::Debug for AudioFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioFile")
            .field("path", &self.file)
            .finish_non_exhaustive()
    }
}

impl AudioFile {
    pub const SUPPORTED_EXTENSIONS: [&'static str; 3] = ["mp3", "ogg", "m4a"];

    pub fn from_tagged_file(
        file: Utf8File,
        tagged_file: &TaggedFile,
    ) -> TFMTResult<AudioFile> {
        match tagged_file.primary_tag() {
            Some(tag) => Ok(AudioFile { file: file.clone(), tag: tag.clone() }),
            None => Err(TFMTError::NoPrimaryTag(file.into_path_buf())),
        }
    }
    ...
```

Note the `NoPrimaryTag` variant currently takes `Utf8PathBuf`; keep that — `file.into_path_buf()` needs `Utf8PathExt` in scope, so add `crate::util::Utf8PathExt` to the imports. The rest of `impl AudioFile` (`file`, `extension`, `tag`, `construct_target_path`, `tag_mut`, `path_predicate`) is unchanged.

- [ ] **Step 3: Add `fs::read_audio_file`**

Create `crates/fs/src/audiofile.rs`:

```rust
use camino::Utf8PathBuf;
use tfmttools_core::audiofile::AudioFile;
use tfmttools_core::util::Utf8File;

use crate::error::{FsError, FsResult};

pub fn read_audio_file(path: Utf8PathBuf) -> FsResult<AudioFile> {
    let tagged_file = lofty::read_from_path(&path)
        .map_err(|err| FsError::Lofty(path.clone(), err))?;

    Ok(AudioFile::from_tagged_file(Utf8File::new(&path), &tagged_file)?)
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use camino::Utf8PathBuf;
    use color_eyre::Result;

    use super::*;

    #[test]
    fn read_audio_file_errors_on_nonexistent_path() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let path = Utf8PathBuf::try_from(temp_dir.path().join("missing.mp3"))?;

        let error = read_audio_file(path).unwrap_err();

        assert!(matches!(error, FsError::Lofty(_, _)));

        Ok(())
    }
}
```

The trailing `?` on `AudioFile::from_tagged_file(...)?` relies on `FsError`'s `#[from] TFMTError` from Task 1 to convert a `NoPrimaryTag` failure automatically.

Edit `crates/fs/src/lib.rs`, add the module:

```rust
mod action;
mod audiofile;
mod checksum;
mod error;
mod file_or_name;
mod fs_handler;
mod path_iterator;
mod template;
mod verify;

pub use action::{ActionExecutor, ActionHandler};
pub use audiofile::read_audio_file;
pub use checksum::{get_file_checksum, get_path_checksum};
pub use error::{FsError, FsResult};
pub use file_or_name::FileOrName;
pub use fs_handler::{FsHandler, RemoveDirResult, get_longest_common_prefix};
pub use path_iterator::{PathIterator, PathIteratorOptions};
pub use template::TemplateLoader;
pub use verify::{verify_directory, verify_file};
```

- [ ] **Step 4: Run the new test and existing core tests**

Run: `cargo test -p tfmttools-fs audiofile && cargo test -p tfmttools-core`
Expected: pass.

- [ ] **Step 5: Update `discovery.rs`**

Edit `crates/tfmt/src/commands/rename/discovery.rs`. Replace `use tfmttools_core::error::TFMTResult;` with `use tfmttools_fs::FsResult;` (check whether `TFMTResult` is still used elsewhere in the file first — it currently isn't, once this call site changes), add `read_audio_file` to the `tfmttools_fs` import, and change:

```rust
use tfmttools_fs::{PathIterator, read_audio_file};
...
    let audio_files = file_paths
        .into_iter()
        .inspect(|_| {
            bar.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        })
        .map(|path| {
            let audio_file = read_audio_file(path)?;

            trace!("Found audio file: {audio_file:?}");

            Ok(audio_file)
        })
        .collect::<FsResult<Vec<_>>>();
```

- [ ] **Step 6: Update `validate.rs`**

Edit `crates/tfmt/src/commands/validate.rs`. Replace `use tfmttools_core::audiofile::AudioFile;` + `use tfmttools_core::error::TFMTError;` with:

```rust
use tfmttools_core::audiofile::AudioFile;
use tfmttools_fs::{FsError, read_audio_file};
```

Change both call sites:

```rust
        if let Ok(audio_file) = read_audio_file(path.clone()) {
```

```rust
        match read_audio_file(path.clone()) {
```

Change the struct field:

```rust
#[derive(Debug)]
struct ValidationReadError {
    path: Utf8PathBuf,
    error: FsError,
}
```

- [ ] **Step 7: Build and test**

Run: `cargo build --workspace 2>&1 | head -100`
Expected: succeeds (or shows only fallout for tasks not yet reached).

Run: `cargo test -p tfmt --bin tfmt`
Expected: passes.

- [ ] **Step 8: Commit**

```bash
git add crates/core/src/audiofile.rs crates/fs/src/audiofile.rs crates/fs/src/lib.rs crates/tfmt/src/commands/rename/discovery.rs crates/tfmt/src/commands/validate.rs
git commit -m "core+fs+tfmt: split AudioFile disk read out of core into fs::read_audio_file"
```

---

### Task 8: Make `RenameAction`'s make-dir planning pure; move the existence filter into `fs`

**Files:**
- Modify: `crates/core/src/action/rename_action.rs`
- Modify: `crates/core/src/action/mod.rs` (only if the removed `Action`-returning method was re-exported — verify)
- Modify: `crates/fs/src/action/rename_planner.rs`

**Interfaces:**
- Produces: `core::RenameAction::intermediate_directories(rename_actions: &[RenameAction]) -> Vec<Utf8Directory>` (replaces `get_make_dir_actions`, drops the `.exists()` filter and the `Action::MakeDir` wrapping — both moved to `fs`).

- [ ] **Step 1: Confirm baseline**

Run: `cargo test -p tfmttools-core action && cargo test -p tfmttools-fs action`
Expected: pass.

- [ ] **Step 2: Rename and de-filter `get_make_dir_actions`**

Edit `crates/core/src/action/rename_action.rs`:

```rust
    #[must_use]
    pub fn intermediate_directories(
        rename_actions: &[RenameAction],
    ) -> Vec<Utf8Directory> {
        let target_paths =
            rename_actions.iter().map(RenameAction::target).collect::<Vec<_>>();

        Self::list_all_intermediate_paths_of_files(&target_paths)
    }
```

The `use crate::action::Action;` import and the `Action::MakeDir` construction move out of this file entirely; if `Action` is otherwise unused in `rename_action.rs` after this edit, remove the import (check with `cargo build -p tfmttools-core` — an unused-import warning will tell you).

- [ ] **Step 3: Update `fs`'s `RenamePlanner` to do the existence filter and `Action` wrapping**

Edit `crates/fs/src/action/rename_planner.rs`:

```rust
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::util::Utf8PathExt;

use super::PlannedAction;
use super::rename_cycles::RenameCycleDetector;
use super::rename_staging::StagedRenamePlanner;

pub(super) struct RenamePlanner {
    rename_actions: Vec<RenameAction>,
}

impl RenamePlanner {
    pub(super) fn new(rename_actions: Vec<RenameAction>) -> Self {
        Self { rename_actions }
    }

    pub(super) fn plan(self) -> Vec<PlannedAction> {
        let make_dir_actions =
            RenameAction::intermediate_directories(&self.rename_actions)
                .into_iter()
                .filter(|dir| !dir.as_path().exists())
                .map(|dir| {
                    PlannedAction::Action(Action::MakeDir(dir.into_path_buf()))
                });

        let move_actions = self.plan_move_actions();

        make_dir_actions.chain(move_actions).collect()
    }

    fn plan_move_actions(self) -> Vec<PlannedAction> {
        if RenameCycleDetector::new(&self.rename_actions)
            .needs_temporary_staging()
        {
            StagedRenamePlanner::new(&self.rename_actions)
                .plan(self.rename_actions)
        } else {
            self.rename_actions.into_iter().map(PlannedAction::Rename).collect()
        }
    }
}
```

- [ ] **Step 4: Write a regression test for the existence filter, since it previously had no direct coverage**

Add to `crates/fs/src/action/executor.rs`'s `#[cfg(test)] mod tests` (it already has the `TempDir`/`rename_action`/`apply_actions` helpers this test needs):

```rust
    #[test]
    fn skips_make_dir_for_directory_that_already_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let existing_dir = temp_dir.path().join("existing");
        fs_err::create_dir(&existing_dir)?;

        let source = temp_path(&temp_dir, "A.mp3")?;
        let target =
            Utf8PathBuf::try_from(existing_dir.join("B.mp3"))?;
        write_file(&source, "a")?;

        let fs_handler = FsHandler::new(FSMode::Default);
        // Should not error even though the intermediate directory
        // ("existing") is already present on disk.
        apply_actions(&fs_handler, vec![rename_action(&source, &target)?])?;

        assert_eq!(read_file(&target)?, "a");

        Ok(())
    }
```

- [ ] **Step 5: Run the test, confirm it fails without the filter, then confirm it passes with it**

Run: `cargo test -p tfmttools-fs skips_make_dir_for_directory_that_already_exists`
Expected: passes as written (the filter is already in place from Step 3 — this test documents and locks in behavior that existed implicitly before the refactor). If you want to see it exercise the filter meaningfully, temporarily comment out the `.filter(|dir| !dir.as_path().exists())` line, confirm the test still passes (since `FsHandler::create_dir` already tolerates an existing directory via its own `path.is_dir() => Exists` branch) — this confirms the filter is an optimization/planning-purity concern, not a correctness requirement, which is worth knowing. Restore the filter afterward.

- [ ] **Step 6: Run full test suites**

Run: `cargo test -p tfmttools-core action && cargo test -p tfmttools-fs action`
Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/action/rename_action.rs crates/fs/src/action/rename_planner.rs crates/fs/src/action/executor.rs
git commit -m "core+fs: move make-dir existence filtering out of core into RenamePlanner"
```

---

### Task 9: Inject existing-target-paths into `validate_rename_actions`

**Files:**
- Modify: `crates/core/src/action/validation.rs`
- Modify: `crates/core/src/action/validation/collisions.rs`
- Create: `crates/fs/src/existing_paths.rs`
- Modify: `crates/fs/src/lib.rs`
- Modify: `crates/tfmt/src/commands/rename/preview.rs`

**Interfaces:**
- Produces: `core::validate_rename_actions(rename_actions: &[RenameAction], existing_targets: &CaseInsensitivePathSet) -> Vec<ValidationError>` (was: `validate_rename_actions(rename_actions: &[RenameAction])`).
- Produces: `tfmttools_fs::existing_target_paths(rename_actions: &[RenameAction]) -> CaseInsensitivePathSet`.

- [ ] **Step 1: Confirm baseline**

Run: `cargo test -p tfmttools-core validation`
Expected: pass.

- [ ] **Step 2: Add a test for `TargetExists` before touching the implementation**

`validate_existing_files`/`ValidationError::TargetExists` currently has no dedicated test (it was implicitly untestable without touching a real filesystem). Add to `crates/core/src/action/validation.rs`'s `#[cfg(test)] mod test`:

```rust
    #[test]
    fn test_validate_target_exists() {
        let action = RenameAction::new(
            Utf8File::new("input/a.mp3"),
            Utf8File::new("music/b.mp3"),
        );

        let mut existing = CaseInsensitivePathSet::new();
        existing.insert("music/b.mp3");

        let errors = validate_rename_actions(&[action], &existing);

        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::TargetExists(_)));
    }

    #[test]
    fn test_validate_target_exists_ignores_unrelated_paths() {
        let action = RenameAction::new(
            Utf8File::new("input/a.mp3"),
            Utf8File::new("music/b.mp3"),
        );

        let existing = CaseInsensitivePathSet::new();

        assert_valid(&[action], &existing);
    }
```

Note: `Utf8File::new` no longer returns a `Result` (Task 6), so these construct directly without `.unwrap()`.

- [ ] **Step 3: Run it to confirm it fails to compile (signature doesn't accept the second argument yet)**

Run: `cargo test -p tfmttools-core validation`
Expected: FAIL to compile — "this function takes 1 argument but 2 arguments were supplied" (and `assert_valid`/other test helpers also need updating, which is expected — proceed to Step 4).

- [ ] **Step 4: Thread the existing-targets parameter through**

Edit `crates/core/src/action/validation/collisions.rs`. Add `CaseInsensitivePathSet` to the import and change `validate_existing_files`:

```rust
use crate::action::{CaseInsensitivePathKey, CaseInsensitivePathSet, RenameAction};
use crate::util::Utf8PathExt;
...
pub(super) fn validate_existing_files<'a>(
    rename_actions: &'a [RenameAction],
    existing_targets: &CaseInsensitivePathSet,
) -> Vec<ValidationError<'a>> {
    let sources =
        rename_actions.iter().map(RenameAction::source).collect::<Vec<_>>();

    rename_actions
        .iter()
        .filter(|m| {
            existing_targets.contains(m.target())
                && m.target() != m.source()
                && !sources.iter().any(|source| {
                    CaseInsensitivePathKey::new(source)
                        == CaseInsensitivePathKey::new(m.target())
                })
        })
        .map(ValidationError::TargetExists)
        .collect()
}
```

(the `Utf8PathExt` import may already be present for `Utf8File`/`Utf8Directory` methods elsewhere in the file — check before adding a duplicate; the existing file only imports `crate::util::Utf8PathExt` inside `#[cfg(test)] mod test`, so add it at module scope now since `m.target()` needs `Display`, not `Utf8PathExt`— actually `CaseInsensitivePathSet::contains`/`insert` take `impl Display`, and `Utf8File`/`&Utf8File` already implement `Display` directly, so no new import is needed for that call; only keep imports that are actually used, verify with `cargo build`).

Edit `crates/core/src/action/validation.rs`:

```rust
use crate::action::CaseInsensitivePathSet;

#[must_use]
pub fn validate_rename_actions<'a>(
    rename_actions: &'a [RenameAction],
    existing_targets: &CaseInsensitivePathSet,
) -> Vec<ValidationError<'a>> {
    let mut errors = Vec::new();

    errors.extend(validate_double_separators(rename_actions));
    errors.extend(validate_collisions(rename_actions));
    errors.extend(validate_case_insensitive_collisions(rename_actions));
    errors.extend(validate_existing_files(rename_actions, existing_targets));
    errors.extend(validate_reserved_names(rename_actions));
    errors.extend(
        validate_forbidden_leading_or_trailing_characters_in_path_component(
            rename_actions,
            &FORBIDDEN_LEADING_OR_TRAILING_CHARACTERS,
        ),
    );
    errors.extend(validate_target_path_too_long(rename_actions));

    errors
}
```

- [ ] **Step 5: Update the existing test helpers to pass an empty set**

In `crates/core/src/action/validation.rs`'s test module, update the three helpers:

```rust
    fn assert_valid(
        rename_actions: &[RenameAction],
        existing_targets: &CaseInsensitivePathSet,
    ) {
        assert!(
            validate_rename_actions(rename_actions, existing_targets)
                .is_empty()
        );
    }

    fn assert_single_error<'a>(
        rename_actions: &'a [RenameAction],
    ) -> ValidationError<'a> {
        let mut errors = validate_rename_actions(
            rename_actions,
            &CaseInsensitivePathSet::new(),
        );

        assert!(errors.len() == 1);

        errors.pop().unwrap()
    }

    fn assert_n_errors(
        rename_actions: &'_ [RenameAction],
        n: usize,
    ) -> Vec<ValidationError<'_>> {
        let errors = validate_rename_actions(
            rename_actions,
            &CaseInsensitivePathSet::new(),
        );

        let len = errors.len();

        assert_eq!(len, n, "expected {n} errors, got {len}.");

        errors
    }
```

Note `assert_valid` now takes the set explicitly (needed by the new `test_validate_target_exists_ignores_unrelated_paths` test); every *other* existing call to `assert_valid(&valid)` in the file needs a second argument. Run:

Run: `sed -i -E 's/assert_valid\(&(valid)\);/assert_valid(\&\1, \&CaseInsensitivePathSet::new());/' crates/core/src/action/validation.rs`
Expected: updates every `assert_valid(&valid);` call. Verify with `grep -n "assert_valid(" crates/core/src/action/validation.rs` that all call sites now pass two arguments (there are two such sites — `test_validate_double_separators` — actually check the exact variable name used at each call site, since not all of them use `valid` as the binding name; read the file after the `sed` and fix any call site the pattern missed by hand).

- [ ] **Step 6: Run core's validation tests**

Run: `cargo test -p tfmttools-core validation`
Expected: all pass, including the two new tests from Step 2.

- [ ] **Step 7: Add `fs::existing_target_paths`**

Create `crates/fs/src/existing_paths.rs`:

```rust
use tfmttools_core::action::{CaseInsensitivePathSet, RenameAction};
use tfmttools_core::util::Utf8PathExt;

#[must_use]
pub fn existing_target_paths(
    rename_actions: &[RenameAction],
) -> CaseInsensitivePathSet {
    let mut set = CaseInsensitivePathSet::new();

    for rename_action in rename_actions {
        if rename_action.target().exists() {
            set.insert(rename_action.target());
        }
    }

    set
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use camino::Utf8PathBuf;
    use color_eyre::Result;
    use tfmttools_core::util::Utf8File;

    use super::*;

    #[test]
    fn includes_only_targets_that_exist_on_disk() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let existing = temp_dir.path().join("existing.mp3");
        let missing = temp_dir.path().join("missing.mp3");
        fs_err::write(&existing, "x")?;

        let existing = Utf8File::new(Utf8PathBuf::try_from(existing)?);
        let missing = Utf8File::new(Utf8PathBuf::try_from(missing)?);

        let rename_actions = vec![
            RenameAction::new(Utf8File::new("a.mp3"), existing.clone()),
            RenameAction::new(Utf8File::new("b.mp3"), missing.clone()),
        ];

        let set = existing_target_paths(&rename_actions);

        assert!(set.contains(&existing));
        assert!(!set.contains(&missing));

        Ok(())
    }
}
```

Edit `crates/fs/src/lib.rs`, add the module:

```rust
mod action;
mod audiofile;
mod checksum;
mod error;
mod existing_paths;
mod file_or_name;
mod fs_handler;
mod path_iterator;
mod template;
mod verify;

pub use action::{ActionExecutor, ActionHandler};
pub use audiofile::read_audio_file;
pub use checksum::{get_file_checksum, get_path_checksum};
pub use error::{FsError, FsResult};
pub use existing_paths::existing_target_paths;
pub use file_or_name::FileOrName;
pub use fs_handler::{FsHandler, RemoveDirResult, get_longest_common_prefix};
pub use path_iterator::{PathIterator, PathIteratorOptions};
pub use template::TemplateLoader;
pub use verify::{verify_directory, verify_file};
```

- [ ] **Step 8: Run the new test**

Run: `cargo test -p tfmttools-fs existing_paths`
Expected: pass.

- [ ] **Step 9: Update `preview.rs`, the only caller of `validate_rename_actions`**

Edit `crates/tfmt/src/commands/rename/preview.rs`:

```rust
use tfmttools_core::action::{RenameAction, validate_rename_actions};
use tfmttools_core::util::{Utf8File, Utf8PathExt};
use tfmttools_core::warning::Warning;
use tfmttools_fs::existing_target_paths;
...
fn validate_rename_action_errors(
    rename_actions: &[RenameAction],
) -> Result<()> {
    let existing_targets = existing_target_paths(rename_actions);
    let validation_errors =
        validate_rename_actions(rename_actions, &existing_targets);

    if validation_errors.is_empty() {
        Ok(())
    } else {
        let error_string = validation_errors
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        Err(eyre!("Had validation errors:\n{error_string}"))
    }
}
```

- [ ] **Step 10: Build and test the whole workspace**

Run: `cargo build --workspace 2>&1 | head -100`
Expected: succeeds (or shows only fallout not yet reached).

Run: `cargo test -p tfmt --bin tfmt`
Expected: pass.

- [ ] **Step 11: Commit**

```bash
git add crates/core/src/action/validation.rs crates/core/src/action/validation/collisions.rs crates/fs/src/existing_paths.rs crates/fs/src/lib.rs crates/tfmt/src/commands/rename/preview.rs
git commit -m "core+fs+tfmt: inject existing-target-paths into validate_rename_actions"
```

---

### Task 10: Move pure template-source parsing from `fs::TemplateLoader` into `core::templates`

**Files:**
- Create: `crates/core/src/templates/source.rs`
- Modify: `crates/core/src/templates/mod.rs`
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/fs/src/template.rs`

**Interfaces:**
- Produces: `core::templates::parse_template_source(label: &str, source: String) -> TFMTResult<(String, Option<Frontmatter>, Vec<Warning>)>` — pure, replaces `TemplateLoader::split_frontmatter` + `TemplateLoader::deprecation_warnings`.
- Produces: `fs::TemplateLoader::{read_directory, read_filename, read_script, get_template, get_all_templates}` now return `FsResult` instead of `TFMTResult`.

- [ ] **Step 1: Confirm baseline**

Run: `cargo test -p tfmttools-fs template`
Expected: pass (this is the module holding all the `split_frontmatter`/`read_script` tests today).

- [ ] **Step 2: Add `regex` to `core`'s dependencies**

Edit `crates/core/Cargo.toml`, `[dependencies]` block, add (alphabetically, after `minijinja`):

```toml
regex = { workspace = true }
```

- [ ] **Step 3: Create the pure parsing module in `core`**

Create `crates/core/src/templates/source.rs`, moving the logic verbatim from `fs/src/template.rs`'s `split_frontmatter`, `deprecation_warnings`, `body_uses_indexed_args`, `description`, and the `FRONTMATTER_FENCE` constant, combined into one entry point:

```rust
use std::sync::LazyLock;

use regex::Regex;

use super::Frontmatter;
use crate::error::{TFMTError, TFMTResult};
use crate::warning::Warning;

const FRONTMATTER_FENCE: &str = "+++";

/// Splits a raw template source into its Jinja body and (optional)
/// frontmatter block, and computes any deprecation warnings implied by
/// the pre-split source. Pure: operates only on the given string.
pub fn parse_template_source(
    label: &str,
    source: String,
) -> TFMTResult<(String, Option<Frontmatter>, Vec<Warning>)> {
    let (body, frontmatter) = split_frontmatter(label, source)?;

    let warnings = if frontmatter.is_none() {
        deprecation_warnings(label, &body)
    } else {
        Vec::new()
    };

    Ok((body, frontmatter, warnings))
}

fn deprecation_warnings(label: &str, body: &str) -> Vec<Warning> {
    let mut warnings = Vec::new();

    if body_uses_indexed_args(body) {
        warnings.push(Warning::DeprecatedPositionalArgs {
            template: label.to_owned(),
        });
    }

    if description(body).is_some() {
        warnings.push(Warning::DeprecatedLeadingComment {
            template: label.to_owned(),
        });
    }

    warnings
}

fn split_frontmatter(
    label: &str,
    source: String,
) -> TFMTResult<(String, Option<Frontmatter>)> {
    // The regex crate doesn't support look-around, so the opening and
    // closing fences are matched with two separate anchored patterns
    // instead of one monolithic `open ... \r?\n ... close` capture. The
    // closing fence is found by searching for a line consisting solely
    // of `+++` (optionally followed by trailing spaces/tabs) starting
    // right after the opening fence. This lets the closing fence
    // immediately follow the opening fence's own newline when the
    // frontmatter block has no content (e.g. "+++\n+++\n"), since
    // `find_at` treats the position right after that newline as a valid
    // line start rather than requiring a second, independent `\r?\n`
    // between the two fences.
    static RE_OPENING_FENCE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\A\+\+\+[ \t]*\r?\n").unwrap());

    static RE_CLOSING_FENCE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^\+\+\+[ \t]*\r?$").unwrap());

    if !source.starts_with(FRONTMATTER_FENCE) {
        return Ok((source, None));
    }

    let Some(opening) = RE_OPENING_FENCE.find(&source) else {
        return Err(TFMTError::UnterminatedFrontmatter(label.to_owned()));
    };

    let Some(closing) = RE_CLOSING_FENCE.find_at(&source, opening.end())
    else {
        return Err(TFMTError::UnterminatedFrontmatter(label.to_owned()));
    };

    let toml_text = &source[opening.end()..closing.start()];

    let frontmatter = Frontmatter::parse(toml_text, label)?;

    let mut body_start = closing.end();

    if let Some(rest) = source[body_start..].strip_prefix("\r\n") {
        body_start = source.len() - rest.len();
    } else if let Some(rest) = source[body_start..].strip_prefix('\n') {
        body_start = source.len() - rest.len();
    }

    let body = source[body_start..].to_owned();

    if body_uses_indexed_args(&body) {
        return Err(TFMTError::IndexedArgsWithFrontmatter(label.to_owned()));
    }

    Ok((body, Some(frontmatter)))
}

fn body_uses_indexed_args(body: &str) -> bool {
    static RE_ARGS_INDEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bargs\s*\[").unwrap());

    RE_ARGS_INDEX.is_match(body)
}

fn description(source: &str) -> Option<String> {
    const COMMENT_START: &str = "{#";
    const COMMENT_END: &str = "#}";

    if source.trim().starts_with(COMMENT_START) {
        source.split_once(COMMENT_END).map(|(left, _)| {
            left.replace(COMMENT_START, "")
                .replace(COMMENT_END, "")
                .trim()
                .to_owned()
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_returns_none_when_absent() {
        let source = "{{ artist }}/{{ title }}".to_owned();

        let (body, frontmatter) =
            split_frontmatter("test", source.clone()).unwrap();

        assert_eq!(body, source);
        assert!(frontmatter.is_none());
    }

    #[test]
    fn split_frontmatter_parses_present_block() {
        let source = "+++\nname = \"Test\"\n+++\n{{ artist }}".to_owned();

        let (body, frontmatter) =
            split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert_eq!(frontmatter.unwrap().name(), Some("Test"));
    }

    #[test]
    fn split_frontmatter_handles_empty_toml_block() {
        let source = "+++\n+++\n{{ artist }}".to_owned();

        let (body, frontmatter) =
            split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert!(frontmatter.is_some());
        assert_eq!(frontmatter.unwrap().name(), None);
    }

    #[test]
    fn split_frontmatter_handles_empty_toml_block_crlf() {
        let source = "+++\r\n+++\r\n{{ artist }}".to_owned();

        let (body, frontmatter) =
            split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert!(frontmatter.is_some());
        assert_eq!(frontmatter.unwrap().name(), None);
    }

    #[test]
    fn split_frontmatter_errors_when_unterminated() {
        let source = "+++\nname = \"Test\"\n{{ artist }}".to_owned();

        let error = split_frontmatter("test", source).unwrap_err();

        assert!(matches!(error, TFMTError::UnterminatedFrontmatter(_)));
    }

    #[test]
    fn split_frontmatter_errors_when_body_uses_indexed_args() {
        let source = "+++\nname = \"Test\"\n+++\n{{ args[0] }}".to_owned();

        let error = split_frontmatter("test", source).unwrap_err();

        assert!(matches!(error, TFMTError::IndexedArgsWithFrontmatter(_)));
    }

    #[test]
    fn split_frontmatter_allows_kwargs_identifier_with_frontmatter() {
        let source = "+++\nname = \"Test\"\n+++\n{{ kwargs[0] }}".to_owned();

        let (body, frontmatter) =
            split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ kwargs[0] }}");
        assert!(frontmatter.is_some());
    }

    #[test]
    fn split_frontmatter_allows_indexed_args_without_frontmatter() {
        let source = "{{ args[0] }}".to_owned();

        let (body, frontmatter) =
            split_frontmatter("test", source.clone()).unwrap();

        assert_eq!(body, source);
        assert!(frontmatter.is_none());
    }

    #[test]
    fn parse_template_source_without_frontmatter_using_indexed_args_returns_warning()
     {
        let (_, _, warnings) =
            parse_template_source("script", "{{ args[0] }}".to_owned())
                .unwrap();

        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            Warning::DeprecatedPositionalArgs { ref template }
            if template == "script"
        ));
    }

    #[test]
    fn parse_template_source_without_frontmatter_with_leading_comment_returns_warning()
     {
        let (_, _, warnings) = parse_template_source(
            "script",
            "{# A description #}\n{{ artist }}".to_owned(),
        )
        .unwrap();

        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            Warning::DeprecatedLeadingComment { ref template }
            if template == "script"
        ));
    }

    #[test]
    fn parse_template_source_with_frontmatter_returns_no_warnings() {
        let (_, _, warnings) = parse_template_source(
            "script",
            "+++\nname = \"Test\"\n+++\n{{ artist }}".to_owned(),
        )
        .unwrap();

        assert!(warnings.is_empty());
    }
}
```

- [ ] **Step 4: Export it from `core::templates`**

Edit `crates/core/src/templates/mod.rs`:

```rust
mod context;
mod frontmatter;
mod source;
mod template;

pub use frontmatter::{ArgKind, ArgSpec, Frontmatter};
pub use source::parse_template_source;
pub use template::Template;
```

- [ ] **Step 5: Run the relocated tests**

Run: `cargo test -p tfmttools-core templates::source`
Expected: all 11 tests pass.

- [ ] **Step 6: Strip the moved logic out of `fs::TemplateLoader` and delegate**

Edit `crates/fs/src/template.rs`. Remove `split_frontmatter`, `deprecation_warnings`, `body_uses_indexed_args`, `description`, and the `FRONTMATTER_FENCE` constant and the two `LazyLock<Regex>`/`Regex` imports they used (keep the `year`/`zero_pad` filters' own regex usage). Change the imports:

```rust
use std::collections::HashMap;

use camino::Utf8Path;
use fs_err as fs;
use minijinja::{Environment, Value, escape_formatter};
use regex::Regex;
use tfmttools_core::templates::{Frontmatter, Template, parse_template_source};
use tfmttools_core::util::{Utf8Directory, Utf8PathExt};
use tfmttools_core::warning::Warning;

use crate::PathIterator;
use crate::error::FsResult;

pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];
```

Change `register_template` to call the new pure function instead of the two removed private methods:

```rust
    fn register_template(
        environment: &mut Environment<'tl>,
        frontmatters: &mut HashMap<String, Frontmatter>,
        name: &str,
        source: String,
    ) -> FsResult<Vec<Warning>> {
        let (body, frontmatter, warnings) =
            parse_template_source(name, source)?;

        if let Some(frontmatter) = frontmatter {
            frontmatters.insert(name.to_owned(), frontmatter);
        }

        environment.add_template_owned(name.to_owned(), body)?;

        Ok(warnings)
    }
```

Change every other `TFMTResult`-typed method on `TemplateLoader` (`read_directory`, `read_filename`, `read_script`, `build`, `get_template`, `get_all_templates`) to `FsResult`. `get_all_templates` currently isn't fallible (returns `Vec<Template<...>>` directly, not a `Result`) — leave it as-is, only change the four/five methods that currently say `-> TFMTResult<...>`.

- [ ] **Step 7: Fix the remaining tests in `fs::template`**

The tests that directly exercised `TemplateLoader::split_frontmatter` (now removed) are already covered by the relocated tests in `core::templates::source` — delete them from `crates/fs/src/template.rs`'s test module: `split_frontmatter_returns_none_when_absent`, `split_frontmatter_parses_present_block`, `split_frontmatter_handles_empty_toml_block`, `split_frontmatter_handles_empty_toml_block_crlf`, `split_frontmatter_errors_when_unterminated`, `split_frontmatter_errors_when_body_uses_indexed_args`, `split_frontmatter_allows_kwargs_identifier_with_frontmatter`, `split_frontmatter_allows_indexed_args_without_frontmatter`.

Keep the remaining tests (`read_script_*`, `get_template_*`, `get_all_templates_*`, `description_comes_only_from_frontmatter_when_present`, `display_name_*`) — they test `TemplateLoader`'s integration behavior, which still belongs in `fs`. Update their imports: replace `use tfmttools_core::error::TFMTError;` with `use crate::error::FsError;` and `use tfmttools_core::warning::Warning;` stays as-is.

Fix the one test that matches on a specific error variant, `get_template_errors_on_missing_required_argument`:

```rust
    #[test]
    fn get_template_errors_on_missing_required_argument() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\", required = true }]\n+++\n{{ prefix }}";

        let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();

        let error = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap_err();

        assert!(matches!(
            error,
            FsError::Core(tfmttools_core::error::TFMTError::MissingRequiredArgument(_, _, _))
        ));
    }
```

- [ ] **Step 8: Run `fs`'s template tests**

Run: `cargo test -p tfmttools-fs template`
Expected: pass (fewer tests than before — the 8 relocated ones are gone from this module, present in `core` instead).

- [ ] **Step 9: Build and test the whole workspace**

Run: `cargo build --workspace 2>&1 | head -100`
Expected: succeeds — `TemplateLoader`'s callers in `tfmt` (`list_templates.rs`, `template_resolution.rs`) all consume it via `?` inside `color_eyre::Result`-returning functions, so no source changes should be needed there (confirm by checking the build output doesn't mention those files).

Run: `cargo test -p tfmt --bin tfmt`
Expected: pass.

- [ ] **Step 10: Commit**

```bash
git add crates/core/Cargo.toml crates/core/src/templates/mod.rs crates/core/src/templates/source.rs crates/fs/src/template.rs
git commit -m "core+fs: move pure template-source parsing into core::templates"
```

---

### Task 11: Trim `core::error::TFMTError` to domain-only variants; drop unused `ignore` dependency

**Files:**
- Modify: `crates/core/src/error.rs`
- Modify: `crates/core/Cargo.toml`

**Interfaces:**
- Produces: `TFMTError` retains only `NoPrimaryTag`, `UnknownTag`, `NotADirectory`/`NotAFile` — **removed**, `ForbiddenCharacterError`, `FrontmatterParse`, `UnterminatedFrontmatter`, `DuplicateArgumentName`, `MissingRequiredArgument`, `TooManyArguments`, `InvalidArgumentValue`, `IndexedArgsWithFrontmatter`, `Minijinja(#[from])`.

By this point, every producer of `NotADirectory`, `NotAFile`, `UnexpectedMoveError`, `FileTooLargeError`, `Lofty`, `Io`, `Ignore`, `Camino` has migrated to `FsError` (Tasks 2–7). This task is the payoff: deleting the now-dead variants from `core`, which the compiler will verify for us.

- [ ] **Step 1: Attempt to remove the variants and let the compiler find any stragglers**

Edit `crates/core/src/error.rs`:

```rust
use camino::Utf8PathBuf;
use thiserror::Error;

pub type TFMTResult<T = (), E = TFMTError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum TFMTError {
    #[error("No primary tag")]
    NoPrimaryTag(Utf8PathBuf),

    #[error("Unknown tag: '{0}'")]
    UnknownTag(String),

    #[error("Interpolated value contains a forbidden character: '{0}'")]
    ForbiddenCharacterError(String),

    #[error("Failed to parse frontmatter TOML in template '{0}': {1}")]
    FrontmatterParse(String, toml::de::Error),

    #[error(
        "Unterminated frontmatter block in template '{0}': missing closing '+++'"
    )]
    UnterminatedFrontmatter(String),

    #[error(
        "Duplicate argument name '{1}' declared in frontmatter of template '{0}'"
    )]
    DuplicateArgumentName(String, String),

    #[error("Missing required argument '{1}' for template '{0}': {2}")]
    MissingRequiredArgument(String, String, String),

    #[error(
        "Template '{0}' accepts at most {1} argument(s), but {2} were supplied"
    )]
    TooManyArguments(String, usize, usize),

    #[error(
        "Argument '{1}' for template '{0}' has an invalid value '{3}': {2}"
    )]
    InvalidArgumentValue(String, String, String, String),

    #[error(
        "Template '{0}' uses indexed `args[N]` access, which is not allowed once a frontmatter block is present"
    )]
    IndexedArgsWithFrontmatter(String),

    #[error(transparent)]
    Minijinja(#[from] minijinja::Error),
}
```

- [ ] **Step 2: Build the whole workspace**

Run: `cargo build --workspace 2>&1 | head -150`
Expected: either succeeds outright, or fails with a small number of "no variant `X` on enum `TFMTError`" errors. If any appear, they mark a producer this plan's earlier tasks missed — locate it (`grep -rn "TFMTError::<variant>" crates`), and fix it using the same pattern as the task that should have covered it (migrate to the equivalent `FsError` variant, or use `FsError::Core(...)` / rely on `?`'s auto-conversion if the call site is in `fs`). Do not reintroduce the removed variant into `core::error::TFMTError` to make the error disappear — that would undo this task's purpose.

- [ ] **Step 3: Remove the now-unused `ignore` dependency from `core`**

Run: `grep -rn "ignore::" crates/core/src`
Expected: no output — `ignore::Error` was only ever referenced by the now-deleted `TFMTError::Ignore` variant.

Edit `crates/core/Cargo.toml`, remove the line:

```toml
ignore = { workspace = true }
```

- [ ] **Step 4: Confirm `core` still builds without the dependency**

Run: `cargo build -p tfmttools-core`
Expected: succeeds.

- [ ] **Step 5: Run the full workspace test suite**

Run: `cargo test --workspace --exclude tfmt`
Run: `cargo test -p tfmt --bin tfmt`
Run: `cargo test -p tfmt --test integration -- --nocapture`
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/error.rs crates/core/Cargo.toml
git commit -m "core: trim TFMTError to domain-only variants, drop unused ignore dep"
```

---

### Task 12: Full workspace verification pass

**Files:** none (verification only).

- [ ] **Step 1: Full build**

Run: `cargo build --workspace`
Expected: clean build, no warnings about unused imports/dead code (these would indicate a leftover from one of the migrations above — fix them if present).

- [ ] **Step 2: Full test suite via `xtask`**

Run: `cargo xtask test`
Expected: all of `cargo test --workspace --exclude tfmt`, `cargo test -p tfmt --bin tfmt`, and `cargo test -p tfmt --test integration -- --nocapture` pass.

- [ ] **Step 3: Lint**

Run: `cargo xtask lint`
Expected: `cargo +nightly fmt --all --check` and `cargo +nightly clippy --workspace --all-targets` both pass. If nightly isn't installed in this environment, run `cargo clippy --workspace --all-targets` and `cargo fmt --all --check` instead and note the substitution when reporting results.

- [ ] **Step 4: Spot-check the boundary is actually clean**

Run: `grep -rn "\.exists()\|\.is_file()\|\.is_dir()" crates/core/src`
Expected: no output — this is the concrete, checkable proof that `core` no longer touches the filesystem.

Run: `grep -rn "lofty::read_from_path\|fs_err::\|std::fs::" crates/core/src`
Expected: no output.

Run: `cargo tree -p tfmttools-core -e normal | grep -i "ignore"`
Expected: no output — `ignore` is no longer even a transitive dependency of `core` (it may still appear via `fs`, which is fine).

- [ ] **Step 5: Report**

Summarize in the final message to the user: which of the three original findings were resolved (error-type ownership, core I/O purity, template-parsing placement), what moved where, and the total commit count for this cleanup.

---

## Self-Review Notes

- **Spec coverage:** All three findings from the boundary analysis are addressed — error ownership (Tasks 1–5, 11), core I/O purity (Tasks 6–9), template-parsing placement (Task 10). Task 12 gives a mechanical, greppable proof the purity goal was met rather than just asserting it.
- **Placeholder scan:** every step above shows the actual code being written or the actual command being run; no "similar to Task N" or "add appropriate handling" placeholders remain.
- **Type consistency:** `FsError`/`FsResult` (Task 1) is the single error type introduced and is threaded consistently through Tasks 2–10; `Utf8File::new`/`Utf8Directory::new` (Task 6) keep their names but change arity/fallibility exactly once, and every downstream task was written against the post-Task-6 infallible signature.
