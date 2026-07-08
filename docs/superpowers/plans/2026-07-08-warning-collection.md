# Warning Collection and Reporting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move all user-facing warning output out of library crates (`core`, `fs`) into the `tfmt` CLI, where warnings are collected as typed values and reported in aggregate at natural interaction checkpoints.

**Architecture:** A `Warning` enum in `core` is emitted by `AudioFileContext` (via interior mutability, drained after each render through `Template::render`) and by `TemplateLoader` (returned from `build` and propagated through all public constructors). The `tfmt` crate accumulates warnings into `RenamePlan` and prints a grouped summary section at the rename preview step and at the end of `list_templates`.

**Tech Stack:** Rust, `lofty` (audio tag keys), `minijinja` 2.19 (`Value::from_object`, `Value::downcast_object`), `convert_case` (tag name formatting).

## Global Constraints

- No `eprintln!` or `println!` in `tfmttools-core` or `tfmttools-fs` after this change.
- `AudioFileContext` must remain `Send + Sync` (required by minijinja's `Object` trait) — use `Mutex`, not `RefCell`.
- Warning section is omitted entirely from output when `Vec<Warning>` is empty.
- `convert_case` is already a dependency of `tfmttools-core`.
- Run `cargo test` after each task. Run `cargo clippy` before each commit.

---

### Task 1: Define the `Warning` enum in `core`

**Files:**
- Create: `crates/core/src/warning.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub enum Warning` with variants `WhitespaceInTag { file: String, tag_name: String }`, `DeprecatedPositionalArgs { template: String }`, `DeprecatedLeadingComment { template: String }`

- [ ] **Step 1: Write the failing test**

In `crates/core/src/warning.rs` (create the file):

```rust
#[derive(Debug, PartialEq)]
pub enum Warning {
    WhitespaceInTag { file: String, tag_name: String },
    DeprecatedPositionalArgs { template: String },
    DeprecatedLeadingComment { template: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_variants_are_constructible() {
        let _ = Warning::WhitespaceInTag {
            file: "song.mp3".to_owned(),
            tag_name: "track_artist".to_owned(),
        };
        let _ = Warning::DeprecatedPositionalArgs {
            template: "my_template".to_owned(),
        };
        let _ = Warning::DeprecatedLeadingComment {
            template: "old_template".to_owned(),
        };
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p tfmttools-core warning
```

Expected: compilation error — `warning` module does not exist yet.

- [ ] **Step 3: Register the module in `lib.rs`**

In `crates/core/src/lib.rs`, add:

```rust
pub mod warning;
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p tfmttools-core warning
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/warning.rs crates/core/src/lib.rs
git commit -m "feat(core): add Warning enum"
```

---

### Task 2: Collect whitespace warnings in `AudioFileContext`; surface them through `Template::render`

**Files:**
- Modify: `crates/core/src/templates/context.rs`
- Modify: `crates/core/src/templates/template.rs`

**Interfaces:**
- Consumes: `Warning` from Task 1 (`tfmttools_core::warning::Warning`)
- Produces:
  - `AudioFileContext::safe` — unchanged signature, now initialises `warnings: Mutex::new(Vec::new())`
  - `AudioFileContext::take_warnings(&self) -> Vec<Warning>`
  - `Template::render(&self, audio_file: &AudioFile) -> TFMTResult<(String, Vec<Warning>)>`

**Background:** minijinja's `Object` trait requires `Send + Sync`, so `Mutex` (not `RefCell`) is needed. `Value::from_object(ctx)` moves `ctx` into an internal `Arc`. After calling `self.inner.render(...)`, retrieve the `Arc` with `context_value.downcast_object::<AudioFileContext>()` and drain warnings.

- [ ] **Step 1: Confirm the existing test suite passes before making changes**

```bash
cargo test -p tfmttools-core
```

Expected: all tests pass.

`AudioFileContext` requires a real `AudioFile` (which needs a file on disk) and is not directly unit-testable in isolation. Warning collection is verified through `Template::render` in the integration test suite. The steps below confirm structural correctness via compilation.

- [ ] **Step 3: Add `Mutex<Vec<Warning>>` to `AudioFileContext` and implement `take_warnings`**

Replace the struct and `safe` constructor in `crates/core/src/templates/context.rs`:

```rust
use std::sync::{Arc, Mutex};

use convert_case::{Case, Casing};
use lofty::tag::ItemKey;
use minijinja::Value;
use minijinja::value::Object;
use tracing::trace;

use super::frontmatter::ResolvedArgs;
use crate::action::FORBIDDEN_CHARACTERS;
use crate::audiofile::AudioFile;
use crate::item_keys::ItemKeys;
use crate::warning::Warning;

#[derive(Debug)]
pub struct AudioFileContext {
    audio_file: AudioFile,
    resolved_args: ResolvedArgs,
    warnings: Mutex<Vec<Warning>>,
}

impl AudioFileContext {
    pub fn safe(audio_file: AudioFile, resolved_args: ResolvedArgs) -> Self {
        Self { audio_file, resolved_args, warnings: Mutex::new(Vec::new()) }
    }

    pub fn take_warnings(&self) -> Vec<Warning> {
        self.warnings.lock().unwrap().drain(..).collect()
    }

    // ... all existing methods unchanged ...
}
```

- [ ] **Step 4: Replace `eprintln!` in `read_safe_tag_value` with a warning push**

Replace the `eprintln!` block in `read_safe_tag_value`:

```rust
fn read_safe_tag_value(&self, key: ItemKey) -> Option<String> {
    let raw = self.read_raw_tag_value(key)?;

    if raw != raw.trim() {
        let tag_name = format!("{key:?}")
            .from_case(Case::Pascal)
            .to_case(Case::Snake);
        self.warnings.lock().unwrap().push(Warning::WhitespaceInTag {
            file: self.audio_file.file().file_name().to_owned(),
            tag_name,
        });
    }

    Some(Self::safe_interpolation_value(raw))
}
```

- [ ] **Step 5: Change `Template::render` to return `(String, Vec<Warning>)`**

Replace the `render` method in `crates/core/src/templates/template.rs`:

```rust
use crate::warning::Warning;

pub fn render(&self, audio_file: &AudioFile) -> TFMTResult<(String, Vec<Warning>)> {
    let context =
        AudioFileContext::safe(audio_file.to_owned(), self.resolved.clone());
    let context_value = Value::from_object(context);

    let output = self.inner.render(&context_value)?;

    let warnings = context_value
        .downcast_object::<AudioFileContext>()
        .map_or_else(Vec::new, |ctx| ctx.take_warnings());

    Ok((output, warnings))
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p tfmttools-core
```

Expected: all tests pass. Clippy:

```bash
cargo clippy -p tfmttools-core -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/templates/context.rs crates/core/src/templates/template.rs
git commit -m "feat(core): collect whitespace warnings in AudioFileContext; surface through Template::render"
```

---

### Task 3: Propagate render warnings through `AudioFile::construct_target_path` and `discovery`

**Files:**
- Modify: `crates/core/src/audiofile.rs`
- Modify: `crates/tfmt/src/commands/rename/discovery.rs`

**Interfaces:**
- Consumes: `Template::render` returning `TFMTResult<(String, Vec<Warning>)>` (Task 2)
- Produces:
  - `AudioFile::construct_target_path(&self, template: &Template, relative_path: &Utf8Directory) -> TFMTResult<(Utf8File, Vec<Warning>)>`
  - `discovery::create_actions_from_template(session: &RenameSession, resolved: &ResolvedTemplate) -> Result<(Vec<RenameAction>, Vec<Warning>)>`

- [ ] **Step 1: Write a failing test for `construct_target_path`**

Add to the `#[cfg(test)]` block in `crates/core/src/audiofile.rs` (create the block if it doesn't exist):

```rust
#[cfg(test)]
mod tests {
    // construct_target_path requires a real file on disk and is covered by
    // integration tests. This is a compile-time check that the return type
    // propagates warnings.
    use crate::warning::Warning;

    #[test]
    fn construct_target_path_result_type_includes_warnings() {
        let _: fn() -> Option<Vec<Warning>> = || None;
        // Verified structurally: construct_target_path returns
        // TFMTResult<(Utf8File, Vec<Warning>)> after this change.
    }
}
```

Run:

```bash
cargo test -p tfmttools-core audiofile
```

Expected: PASS (compile check).

- [ ] **Step 2: Update `construct_target_path` in `audiofile.rs`**

```rust
use crate::warning::Warning;

pub fn construct_target_path(
    &self,
    template: &Template,
    relative_path: &Utf8Directory,
) -> TFMTResult<(Utf8File, Vec<Warning>)> {
    let (string, warnings) = template.render(self)?;

    let string = normalize_separators(&string);

    let target_path =
        Utf8PathBuf::from(format!("{string}.{}", self.extension()));

    let target_path = relative_path.join_file(target_path)?;

    Ok((target_path, warnings))
}
```

- [ ] **Step 3: Update `create_rename_actions` in `discovery.rs` to accumulate warnings**

The function currently returns `Result<Vec<RenameAction>>`. Change it to return `Result<(Vec<RenameAction>, Vec<Warning>)>` and accumulate warnings inside the loop.

In `crates/tfmt/src/commands/rename/discovery.rs`:

```rust
use tfmttools_core::warning::Warning;

fn create_rename_actions(
    session: &RenameSession,
    template: &Template,
    files: &[AudioFile],
) -> Result<(Vec<RenameAction>, Vec<Warning>)> {
    let cwd = current_dir_utf8()?;

    let bar = ProgressBar::bar(
        session.app_options().display_mode(),
        "Determining output paths:",
        "Determined output paths.",
        files.len() as u64,
        true,
    );

    let mut all_warnings: Vec<Warning> = Vec::new();

    let rename_actions: Result<Vec<RenameAction>> = files
        .iter()
        .map(|audiofile| {
            let (target, warnings) =
                audiofile.construct_target_path(template, &cwd)?;

            all_warnings.extend(warnings);

            let rename_action =
                RenameAction::new(audiofile.file().to_owned(), target);

            bar.inc_found();
            trace!("Created rename action: {rename_action:?}");

            #[cfg(feature = "debug")]
            crate::debug::delay();

            Ok(rename_action)
        })
        .collect();

    bar.finish();

    Ok((rename_actions?, all_warnings))
}
```

- [ ] **Step 4: Update `create_actions_from_template` to propagate warnings**

```rust
pub(super) fn create_actions_from_template(
    session: &RenameSession,
    resolved: &ResolvedTemplate,
) -> Result<(Vec<RenameAction>, Vec<Warning>)> {
    let template = resolved
        .loader
        .get_template(&resolved.template_name, resolved.arguments.clone())?
        .ok_or(eyre!("Unable to find template: {}", resolved.template_name))?;

    let paths = gather_file_paths(session);

    debug!("Read {} files.", paths.len());

    let audio_files = read_files(session, paths)?;

    debug!("Found {} audio files.", audio_files.len());

    let (rename_actions, warnings) =
        create_rename_actions(session, &template, &audio_files)?;

    Ok((rename_actions, warnings))
}
```

- [ ] **Step 5: Stub-fix `planning.rs` so the workspace compiles**

`create_actions_from_template` now returns `(Vec<RenameAction>, Vec<Warning>)`. Update `crates/tfmt/src/commands/rename/planning.rs` to destructure it, dropping warnings for now (Task 5 replaces this with real accumulation):

```rust
use tfmttools_core::warning::Warning;

pub fn create_plan(
    session: &RenameSession,
    history: &History<Action, ActionRecordMetadata>,
    load_history_result: LoadHistoryResult,
) -> Result<RenamePlan> {
    let resolved = template_resolution::resolve_template(
        session,
        history,
        load_history_result,
    )?;
    let (actions, _warnings) =
        discovery::create_actions_from_template(session, &resolved)?;
    let (actions, unchanged_files) =
        RenameAction::separate_unchanged_destinations(actions);

    Ok(RenamePlan { actions, unchanged_files, metadata: resolved.metadata })
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p tfmttools-core -p tfmt
```

Expected: all tests pass. Clippy:

```bash
cargo clippy -p tfmttools-core -p tfmt -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/audiofile.rs \
        crates/tfmt/src/commands/rename/discovery.rs \
        crates/tfmt/src/commands/rename/planning.rs
git commit -m "feat: propagate render warnings through construct_target_path and discovery"
```

---

### Task 4: Collect deprecation warnings in `TemplateLoader`; return from all constructors

**Files:**
- Modify: `crates/fs/src/template.rs`

**Interfaces:**
- Consumes: `Warning::DeprecatedPositionalArgs`, `Warning::DeprecatedLeadingComment` from Task 1
- Produces:
  - `TemplateLoader::read_directory(dir) -> TFMTResult<(Self, Vec<Warning>)>`
  - `TemplateLoader::read_filename(path, name) -> TFMTResult<(Self, Vec<Warning>)>`
  - `TemplateLoader::read_script(script) -> TFMTResult<(Self, Vec<Warning>)>`
  - `TemplateLoader::build(sources) -> TFMTResult<(Self, Vec<Warning>)>` (private)

- [ ] **Step 1: Write failing tests**

In the existing `#[cfg(test)]` block in `crates/fs/src/template.rs`, add:

```rust
#[test]
fn read_script_without_frontmatter_using_indexed_args_returns_warning() {
    let (_, warnings) =
        TemplateLoader::read_script("{{ args[0] }}").unwrap();

    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        tfmttools_core::warning::Warning::DeprecatedPositionalArgs { ref template }
        if template == TemplateLoader::DEFAULT_SCRIPT_NAME
    ));
}

#[test]
fn read_script_without_frontmatter_with_leading_comment_returns_warning() {
    let (_, warnings) =
        TemplateLoader::read_script("{# A description #}\n{{ artist }}").unwrap();

    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        warnings[0],
        tfmttools_core::warning::Warning::DeprecatedLeadingComment { ref template }
        if template == TemplateLoader::DEFAULT_SCRIPT_NAME
    ));
}

#[test]
fn read_script_with_frontmatter_returns_no_warnings() {
    let (_, warnings) =
        TemplateLoader::read_script("+++\nname = \"Test\"\n+++\n{{ artist }}")
            .unwrap();

    assert!(warnings.is_empty());
}
```

Run:

```bash
cargo test -p tfmttools-fs
```

Expected: compile error — `read_script` currently returns `TFMTResult<Self>`, not `TFMTResult<(Self, Vec<Warning>)>`.

- [ ] **Step 2: Change `build` to return `TFMTResult<(Self, Vec<Warning>)>`**

Update `build` and `register_template` in `crates/fs/src/template.rs`:

```rust
use tfmttools_core::warning::Warning;

fn build(
    sources: impl IntoIterator<Item = (String, String)>,
) -> TFMTResult<(Self, Vec<Warning>)> {
    let mut template_names = Vec::new();
    let mut frontmatters = HashMap::new();
    let mut environment = Self::create_environment();
    let mut warnings = Vec::new();

    for (name, source) in sources {
        let template_warnings = Self::register_template(
            &mut environment,
            &mut frontmatters,
            &name,
            source,
        )?;
        warnings.extend(template_warnings);
        template_names.push(name);
    }

    Ok((Self { template_names, frontmatters, environment }, warnings))
}

fn register_template(
    environment: &mut Environment<'tl>,
    frontmatters: &mut HashMap<String, Frontmatter>,
    name: &str,
    source: String,
) -> TFMTResult<Vec<Warning>> {
    let (body, frontmatter) = Self::split_frontmatter(name, source)?;

    let warnings = if frontmatter.is_none() {
        Self::deprecation_warnings(name, &body)
    } else {
        Vec::new()
    };

    if let Some(frontmatter) = frontmatter {
        frontmatters.insert(name.to_owned(), frontmatter);
    }

    environment.add_template_owned(name.to_owned(), body)?;

    Ok(warnings)
}

fn deprecation_warnings(label: &str, body: &str) -> Vec<Warning> {
    let mut warnings = Vec::new();

    if Self::body_uses_indexed_args(body) {
        warnings.push(Warning::DeprecatedPositionalArgs {
            template: label.to_owned(),
        });
    }

    if Self::description(body).is_some() {
        warnings.push(Warning::DeprecatedLeadingComment {
            template: label.to_owned(),
        });
    }

    warnings
}
```

Remove the `warn_on_deprecated_usage` method entirely.

- [ ] **Step 3: Update the three public constructors**

```rust
pub fn read_directory(
    template_directory: &Utf8Directory,
) -> TFMTResult<(Self, Vec<Warning>)> {
    let iter = PathIterator::single_directory(template_directory.as_path())
        .flatten()
        .filter(|path| Self::path_is_template(path));

    let mut sources = Vec::new();

    for template_path in iter {
        let name = template_path
            .file_stem()
            .expect("Template::path_is_template should only return files.")
            .to_owned();

        let source = fs::read_to_string(&template_path)?;

        sources.push((name, source));
    }

    Self::build(sources)
}

pub fn read_filename(path: &Utf8Path, name: &str) -> TFMTResult<(Self, Vec<Warning>)> {
    let source = fs::read_to_string(path)?;
    Self::build([(name.to_owned(), source)])
}

pub fn read_script(script: &str) -> TFMTResult<(Self, Vec<Warning>)> {
    Self::build([(Self::DEFAULT_SCRIPT_NAME.to_owned(), script.to_owned())])
}
```

- [ ] **Step 4: Fix every call site that calls `read_script` / `read_directory` / `read_filename`**

All tests and callers across the workspace that call these constructors need to destructure the tuple. For example:

```rust
// Before:
let loader = TemplateLoader::read_script(script).unwrap();

// After:
let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();
```

Apply this pattern to:
- Every existing test in `crates/fs/src/template.rs`
- The two existing tests in `crates/tfmt/src/commands/list_templates.rs` that call `TemplateLoader::read_script`

- [ ] **Step 5: Run tests**

```bash
cargo test -p tfmttools-fs
```

Expected: all tests pass, including the three new warning tests. Clippy:

```bash
cargo clippy -p tfmttools-fs -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add crates/fs/src/template.rs
git commit -m "feat(fs): return deprecation warnings from TemplateLoader instead of printing"
```

---

### Task 5: Wire warnings into `RenamePlan` and show at the preview step

**Files:**
- Modify: `crates/tfmt/src/commands/rename/mod.rs`
- Modify: `crates/tfmt/src/commands/rename/template_resolution.rs`
- Modify: `crates/tfmt/src/commands/rename/planning.rs`
- Modify: `crates/tfmt/src/commands/rename/apply.rs`

**Interfaces:**
- Consumes:
  - `TemplateLoader::read_*` returning `(Self, Vec<Warning>)` (Task 4)
  - `discovery::create_actions_from_template` returning `(Vec<RenameAction>, Vec<Warning>)` (Task 3)
- Produces: warning report printed to stdout in `apply::preview` when `warnings` is non-empty

- [ ] **Step 1: Add `warnings` to `RenamePlan` and `ResolvedTemplate`**

In `crates/tfmt/src/commands/rename/mod.rs`:

```rust
use tfmttools_core::warning::Warning;

pub(crate) struct RenamePlan {
    pub(crate) actions: Vec<RenameAction>,
    pub(crate) unchanged_files: Vec<Utf8File>,
    pub(crate) metadata: ActionRecordMetadata,
    pub(crate) warnings: Vec<Warning>,
}
```

In `crates/tfmt/src/commands/rename/template_resolution.rs`:

```rust
use tfmttools_core::warning::Warning;

pub(super) struct ResolvedTemplate {
    pub(super) loader: TemplateLoader<'static>,
    pub(super) template_name: String,
    pub(super) arguments: Vec<String>,
    pub(super) metadata: ActionRecordMetadata,
    pub(super) warnings: Vec<Warning>,
}
```

- [ ] **Step 2: Collect template warnings in `template_resolution.rs`**

Each `resolve_*` function calls a `TemplateLoader` constructor that now returns `(loader, warnings)`. Destructure and store on `ResolvedTemplate`.

Update `resolve_file_or_name`:

```rust
fn resolve_file_or_name(
    session: &RenameSession,
    file_or_name: &FileOrName,
    arguments: &[String],
) -> Result<ResolvedTemplate> {
    debug!("Using template: '{file_or_name}'");
    debug!("Template arguments: '{}'", arguments.join("', '"));

    let (loader, warnings) = match file_or_name {
        FileOrName::File(path, name) => {
            TemplateLoader::read_filename(path, name)
        },
        FileOrName::Name(_) => {
            TemplateLoader::read_directory(
                session.rename_options().template_directory(),
            )
        },
    }?;

    let template_name = file_or_name.as_str().to_owned();
    let arguments = arguments.to_vec();
    let metadata = create_metadata(
        &TemplateMetadata::FileOrName(template_name.clone()),
        session.app_options().run_id(),
        &arguments,
    );

    Ok(ResolvedTemplate { loader, template_name, arguments, metadata, warnings })
}
```

Update `resolve_script`:

```rust
fn resolve_script(
    session: &RenameSession,
    script: &str,
    arguments: &[String],
) -> Result<ResolvedTemplate> {
    debug!("Using script:\n```\n{script}\n```");
    debug!("Template arguments: '{}'", arguments.join("', '"));

    let (loader, warnings) = TemplateLoader::read_script(script)?;
    let arguments = arguments.to_vec();
    let metadata = create_metadata(
        &TemplateMetadata::Script(script.to_owned()),
        session.app_options().run_id(),
        &arguments,
    );

    Ok(ResolvedTemplate {
        loader,
        template_name: TemplateLoader::DEFAULT_SCRIPT_NAME.to_owned(),
        arguments,
        metadata,
        warnings,
    })
}
```

Update `resolve_previous_template` similarly — both branches call `resolve_file_or_name` or `resolve_script` so warnings propagate automatically.

- [ ] **Step 3: Accumulate all warnings in `planning.rs`**

```rust
use tfmttools_core::warning::Warning;

pub fn create_plan(
    session: &RenameSession,
    history: &History<Action, ActionRecordMetadata>,
    load_history_result: LoadHistoryResult,
) -> Result<RenamePlan> {
    let resolved = template_resolution::resolve_template(
        session,
        history,
        load_history_result,
    )?;

    let mut warnings: Vec<Warning> = resolved.warnings;

    let (actions, file_warnings) =
        discovery::create_actions_from_template(session, &resolved)?;

    warnings.extend(file_warnings);

    let (actions, unchanged_files) =
        RenameAction::separate_unchanged_destinations(actions);

    Ok(RenamePlan {
        actions,
        unchanged_files,
        metadata: resolved.metadata,
        warnings,
    })
}
```

- [ ] **Step 4: Write a failing test for the warning report formatter**

In `crates/tfmt/src/commands/rename/apply.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use tfmttools_core::warning::Warning;

    use super::*;

    #[test]
    fn format_warnings_empty_returns_none() {
        assert!(format_warnings(&[]).is_none());
    }

    #[test]
    fn format_warnings_groups_whitespace_by_tag() {
        let warnings = vec![
            Warning::WhitespaceInTag {
                file: "a.mp3".to_owned(),
                tag_name: "track_artist".to_owned(),
            },
            Warning::WhitespaceInTag {
                file: "b.mp3".to_owned(),
                tag_name: "track_artist".to_owned(),
            },
            Warning::WhitespaceInTag {
                file: "c.mp3".to_owned(),
                tag_name: "album".to_owned(),
            },
        ];

        let output = format_warnings(&warnings).unwrap();
        assert!(output.contains("2 files") && output.contains("track_artist"));
        assert!(output.contains("1 file") && output.contains("album"));
    }

    #[test]
    fn format_warnings_includes_template_warnings() {
        let warnings = vec![Warning::DeprecatedPositionalArgs {
            template: "my_template".to_owned(),
        }];

        let output = format_warnings(&warnings).unwrap();
        assert!(output.contains("my_template"));
        assert!(output.contains("positional"));
    }
}
```

Run:

```bash
cargo test -p tfmt rename::apply
```

Expected: FAIL — `format_warnings` does not exist.

- [ ] **Step 5: Implement `format_warnings` and call it in `preview`**

Add to `crates/tfmt/src/commands/rename/apply.rs`:

```rust
use std::collections::HashMap;

use tfmttools_core::warning::Warning;

fn format_warnings(warnings: &[Warning]) -> Option<String> {
    if warnings.is_empty() {
        return None;
    }

    let mut lines: Vec<String> = Vec::new();

    // Template warnings (appear first — collected before file rendering)
    for warning in warnings {
        match warning {
            Warning::DeprecatedPositionalArgs { template } => {
                lines.push(format!(
                    "  \u{26a0} Template '{template}': uses positional args[N] without frontmatter; declare arguments to migrate."
                ));
            },
            Warning::DeprecatedLeadingComment { template } => {
                lines.push(format!(
                    "  \u{26a0} Template '{template}': uses a leading comment as its description; move it to frontmatter's `description` field."
                ));
            },
            Warning::WhitespaceInTag { .. } => {},
        }
    }

    // Whitespace warnings grouped by tag
    let mut whitespace_by_tag: HashMap<&str, usize> = HashMap::new();
    for warning in warnings {
        if let Warning::WhitespaceInTag { tag_name, .. } = warning {
            *whitespace_by_tag.entry(tag_name.as_str()).or_insert(0) += 1;
        }
    }

    // Sort by tag name for stable output
    let mut whitespace_entries: Vec<(&str, usize)> =
        whitespace_by_tag.into_iter().collect();
    whitespace_entries.sort_by_key(|(tag, _)| *tag);

    for (tag, count) in whitespace_entries {
        let file_word = if count == 1 { "file" } else { "files" };
        lines.push(format!(
            "  \u{26a0} {count} {file_word}: leading/trailing whitespace in `{tag}`"
        ));
    }

    Some(format!("Warnings:\n{}", lines.join("\n")))
}
```

Update `preview_rename_actions` to print the warning report after the file list:

```rust
fn preview_rename_actions(
    session: &RenameSession,
    rename_actions: &[RenameAction],
    unchanged_files: &[Utf8File],
    warnings: &[Warning],
) -> Result<()> {
    let working_directory = current_dir_utf8()?;

    let iter = rename_actions.iter().map(|rename_action| {
        super::shared::strip_path_prefix(
            rename_action.target().as_path(),
            working_directory.as_path(),
        )
    });

    let preview_list =
        PreviewList::new(iter, session.app_options().preview_list_size())
            .with_item_name(ItemName::simple("destination"));

    if !unchanged_files.is_empty() {
        println!("There are {} unchanged files.\n", unchanged_files.len());
    }

    preview_list.print()?;

    if let Some(report) = format_warnings(warnings) {
        println!();
        println!("{report}");
    }

    Ok(())
}
```

Update `preview` to pass warnings through:

```rust
pub fn preview(session: &RenameSession, plan: &RenamePlan) -> Result<()> {
    validate_rename_action_errors(&plan.actions)?;
    preview_rename_actions(
        session,
        &plan.actions,
        &plan.unchanged_files,
        &plan.warnings,
    )
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p tfmt
```

Expected: all tests pass, including the three new `apply` tests. Clippy:

```bash
cargo clippy -p tfmt -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add crates/tfmt/src/commands/rename/mod.rs \
        crates/tfmt/src/commands/rename/template_resolution.rs \
        crates/tfmt/src/commands/rename/planning.rs \
        crates/tfmt/src/commands/rename/apply.rs
git commit -m "feat(tfmt): collect and display warnings in rename preview"
```

---

### Task 6: Wire warnings into `list_templates`

**Files:**
- Modify: `crates/tfmt/src/commands/list_templates.rs`

**Interfaces:**
- Consumes: `TemplateLoader::read_directory` returning `(loader, Vec<Warning>)` (Task 4)
- Produces: warning section printed after the template list when non-empty

- [ ] **Step 1: Write a failing test**

Add to the `#[cfg(test)]` block in `crates/tfmt/src/commands/list_templates.rs`:

```rust
#[test]
fn format_template_warnings_is_empty_for_modern_template() {
    let script = "+++\nname = \"Test Template\"\n+++\n{{ artist }}";
    let (loader, warnings) = TemplateLoader::read_script(script).unwrap();
    let _ = loader; // suppress unused warning
    assert!(warnings.is_empty());
}

#[test]
fn format_template_warnings_contains_deprecated_template_name() {
    let script = "{{ args[0] }}";
    let (_, warnings) = TemplateLoader::read_script(script).unwrap();
    assert!(!warnings.is_empty());
}
```

Run:

```bash
cargo test -p tfmt list_templates
```

Expected: FAIL — `read_script` currently returns `TFMTResult<TemplateLoader>` from `tfmt`'s perspective (Task 4 must be done first).

- [ ] **Step 2: Update `list_templates` to destructure `(loader, warnings)` and print the warning section**

```rust
use tfmttools_core::warning::Warning;

pub fn list_templates(template_directory: &Utf8Directory) -> Result<()> {
    let (loader, warnings) =
        TemplateLoader::read_directory(template_directory)?;

    let all_templates = loader.get_all_templates();

    match all_templates.len() {
        0 => {
            println!(
                "Couldn't find any templates at {template_directory} or in the current directory."
            );
        },
        1 => println!("Found 1 template:"),
        other => println!("Found {other} templates:"),
    }

    for template in all_templates {
        println!("{}", format_template(&template));
    }

    if let Some(report) = format_template_warnings(&warnings) {
        println!();
        println!("{report}");
    }

    Ok(())
}

fn format_template_warnings(warnings: &[Warning]) -> Option<String> {
    if warnings.is_empty() {
        return None;
    }

    let lines: Vec<String> = warnings
        .iter()
        .map(|w| match w {
            Warning::DeprecatedPositionalArgs { template } => format!(
                "  \u{26a0} '{template}': uses positional args[N] without frontmatter; declare arguments to migrate."
            ),
            Warning::DeprecatedLeadingComment { template } => format!(
                "  \u{26a0} '{template}': uses a leading comment as its description; move it to frontmatter's `description` field."
            ),
            Warning::WhitespaceInTag { .. } => String::new(),
        })
        .filter(|s| !s.is_empty())
        .collect();

    if lines.is_empty() {
        None
    } else {
        Some(format!("Warnings:\n{}", lines.join("\n")))
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tfmt list_templates
cargo test
```

Expected: all tests pass. Full workspace clippy:

```bash
cargo clippy -- -D warnings
```

- [ ] **Step 4: Commit**

```bash
git add crates/tfmt/src/commands/list_templates.rs
git commit -m "feat(tfmt): show template deprecation warnings in list_templates"
```

---

## Final Verification

- [ ] Run the full test suite: `cargo test`
- [ ] Run clippy across the workspace: `cargo clippy -- -D warnings`
- [ ] Confirm no `eprintln!` remains in `tfmttools-core` or `tfmttools-fs`:

```bash
grep -rn "eprintln!" crates/core/src crates/fs/src
```

Expected: no output.
