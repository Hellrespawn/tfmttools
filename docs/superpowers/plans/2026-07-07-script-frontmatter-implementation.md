# Script Frontmatter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional TOML frontmatter to `.tfmt`/`.jinja`/`.j2` script files that declares a display name, description, and typed/validated argument specs, replacing today's untyped, unsanitized `args[N]` convention while keeping scripts without frontmatter fully backward compatible.

**Architecture:** A new `crates/core/src/templates/frontmatter.rs` module owns the `Frontmatter`/`ArgSpec`/`ArgKind` data model, TOML parsing, and argument resolution (matching, defaulting, type coercion, sanitization) into a private `ResolvedArgs` value scoped to `core::templates`. `crates/fs/src/template.rs` (`TemplateLoader`) strips the `+++...+++` block from raw script source before registering it with minijinja, stores parsed `Frontmatter` in a side table keyed by template name, and performs the two deprecation-warning scans plus the frontmatter-presence hard-error scan for indexed `args[N]` usage. `Template` and `AudioFileContext` (both in `crates/core/src/templates/`) thread resolved argument values through so they're accessible in script bodies both by name and by index. Listing templates (`tfmt list-templates`) never triggers argument resolution, so scripts with required arguments can still be listed without supplying values.

**Tech Stack:** Rust workspace (edition 2024), `toml` crate (new dependency, core only), `regex` (already a dependency), `minijinja`, `thiserror`.

## Global Constraints

- `toml` is added to `[workspace.dependencies]` in the root `Cargo.toml` and consumed via `{ workspace = true }`, matching every other dependency in this workspace — no crate has a non-workspace dependency today.
- `toml` is a dependency of `crates/core` only. `crates/fs` and `crates/tfmt` never import the `toml` crate directly; they only see the `Frontmatter`/`ArgSpec`/`ArgKind` types re-exported from `tfmttools_core::templates`.
- New code must satisfy `clippy::pedantic` (workspace lint, `warn` level) — see `Cargo.toml:67-71` for the exact lint config (`missing_errors_doc`, `missing_panics_doc`, `module_name_repetitions` are allowed).
- Argument sanitization for `String`/`Path` kinds must reuse the existing forbidden-character table (`crates/core/src/action/validation/forbidden.rs::FORBIDDEN_CHARACTERS`) via `AudioFileContext`'s existing `remove_forbidden_characters` function — do not duplicate the character list.
- Frontmatter presence, not argument declaration, is what triggers the hard error for indexed `args[N]` access: even a frontmatter block with an empty/absent `args` list disallows `args[N]` in the body. This is the compat-table's explicit "even if `args` is empty" rule, and it takes precedence over the looser wording in the design doc's "Argument resolution" section — see the design doc `docs/superpowers/specs/2026-07-06-script-frontmatter-design.md` lines 127-129 vs. 151-154. Resolved by treating "no frontmatter" as the only escape hatch for the `args[N]` hard error; the "too many positional arguments" check (design doc line 113-114) separately exempts frontmatter with zero declared args.
- `TemplateLoader::get_all_templates` (used only by `tfmt list-templates`) must never fail or hard-error because a script declares a required argument with no default — listing must always succeed. It builds `Template`s for display only, without calling argument resolution.
- New `TFMTError` variants that need to identify a script use a `String` label (the template's lookup name, or `TemplateLoader::DEFAULT_SCRIPT_NAME` for inline `--script` invocations), not `Utf8PathBuf`, since inline scripts have no file path.

---

## File Structure

- `crates/core/src/error.rs` (modify): seven new `TFMTError` variants for frontmatter parsing/validation/resolution failures.
- `crates/core/Cargo.toml` (modify): add `toml` dependency.
- `Cargo.toml` (modify): add `toml` to `[workspace.dependencies]`.
- `crates/core/src/templates/frontmatter.rs` (new): `Frontmatter`, `ArgSpec`, `ArgKind` data model; `Frontmatter::parse`, `Frontmatter::resolve`; private `ResolvedArgs`.
- `crates/core/src/templates/context.rs` (modify): `remove_forbidden_characters` becomes `pub(super)`; `AudioFileContext` holds `ResolvedArgs` instead of raw `Vec<String>`; `get_value` checks named args before tag lookup.
- `crates/core/src/templates/template.rs` (modify): `Template` gains `declared_args`/`resolved` fields; `Template::new` becomes fallible and resolves arguments; new `Template::for_display` constructor skips resolution.
- `crates/core/src/templates/mod.rs` (modify): export `Frontmatter`, `ArgSpec`, `ArgKind`.
- `crates/fs/src/template.rs` (modify): `TemplateLoader` gains a `frontmatters` side table; frontmatter stripping/parsing, deprecation warnings, hard-error scan; `get_template` returns `TFMTResult<Option<Template>>`; `get_all_templates` builds display-only templates.
- `crates/tfmt/src/commands/rename/discovery.rs` (modify): adapt to `get_template`'s new `Result`-wrapped return type.
- `crates/tfmt/src/commands/list_templates.rs` (modify): `format_template` prints declared argument specs.
- `README.md` (modify): remove the frontmatter TODO line; add a "Script Frontmatter" section.
- `examples/stef.tfmt` (modify): rewritten to use frontmatter with a `path`-typed `prefix` argument.
- `tests/fixtures/cli/template/frontmatter_prefix.tfmt` (new): frontmatter-declared demo script for CLI fixtures.
- `tests/fixtures/cli/cases/frontmatter_prefix.case.json` (new): success-path fixture.
- `tests/fixtures/cli/cases/frontmatter_prefix_missing_required.case.json` (new): missing-required-argument failure fixture.

---

### Task 1: `TFMTError` variants for frontmatter

**Files:**
- Modify: `crates/core/src/error.rs`

**Interfaces:**
- Produces: `TFMTError::FrontmatterParse(String, toml::de::Error)`, `TFMTError::UnterminatedFrontmatter(String)`, `TFMTError::DuplicateArgumentName(String, String)`, `TFMTError::MissingRequiredArgument(String, String, String)`, `TFMTError::TooManyArguments(String, usize, usize)`, `TFMTError::InvalidArgumentValue(String, String, String, String)`, `TFMTError::IndexedArgsWithFrontmatter(String)` — all consumed by Task 3 and Task 7.

This task only adds error variants (no tests of its own — they're exercised by Tasks 3 and 7's tests). Since `error.rs` has no existing test module and the variants are pure data, skip the TDD test-first cycle here and verify via `cargo check` instead.

- [ ] **Step 1: Add the new variants**

Edit `crates/core/src/error.rs`, inserting after the existing `ForbiddenCharacterError` variant (before the "Passthrough errors" comment):

```rust
    #[error("Interpolated value contains a forbidden character: '{0}'")]
    ForbiddenCharacterError(String),

    #[error("Failed to parse frontmatter TOML in template '{0}': {1}")]
    FrontmatterParse(String, toml::de::Error),

    #[error("Unterminated frontmatter block in template '{0}': missing closing '+++'")]
    UnterminatedFrontmatter(String),

    #[error("Duplicate argument name '{1}' declared in frontmatter of template '{0}'")]
    DuplicateArgumentName(String, String),

    #[error("Missing required argument '{1}' for template '{0}': {2}")]
    MissingRequiredArgument(String, String, String),

    #[error("Template '{0}' accepts at most {1} argument(s), but {2} were supplied")]
    TooManyArguments(String, usize, usize),

    #[error("Argument '{1}' for template '{0}' has an invalid value '{3}': {2}")]
    InvalidArgumentValue(String, String, String, String),

    #[error(
        "Template '{0}' uses indexed `args[N]` access, which is not allowed once a frontmatter block is present"
    )]
    IndexedArgsWithFrontmatter(String),

    // Passthrough errors
```

- [ ] **Step 2: Verify it doesn't compile yet (expected — `toml` isn't a dependency)**

Run: `cargo check -p tfmttools-core`
Expected: FAIL with `` error[E0433]: failed to resolve: use of undeclared crate or module `toml` ``

This confirms Step 1 wired in the `toml::de::Error` reference; Task 2 adds the dependency.

- [ ] **Step 3: Commit**

```bash
git add crates/core/src/error.rs
git commit -m "Add TFMTError variants for script frontmatter"
```

---

### Task 2: Add `toml` dependency

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/core/Cargo.toml`

**Interfaces:**
- Produces: `toml` crate available to `crates/core` via `toml::from_str`, `toml::de::Error`, consumed by Task 3.

- [ ] **Step 1: Discover the current resolvable version**

Run: `cargo add toml -p tfmttools-core --dry-run`
Expected: output showing a line like `Adding toml vX.Y.Z to dependencies` (note the exact version printed).

- [ ] **Step 2: Add it the workspace way**

In `Cargo.toml`, insert into `[workspace.dependencies]` alphabetically between `thiserror` and `tracing` (using the version discovered in Step 1, e.g. if it printed `0.9.8`):

```toml
thiserror = "2.0.18"
toml = "0.9.8"
tracing = "0.1.44"
```

In `crates/core/Cargo.toml`, insert into `[dependencies]` alphabetically between `thiserror` and `tracing`:

```toml
thiserror = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p tfmttools-core`
Expected: PASS (no errors — Task 1's `toml::de::Error` reference now resolves)

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/core/Cargo.toml
git commit -m "Add toml dependency to crates/core"
```

---

### Task 3: `Frontmatter`/`ArgSpec`/`ArgKind` data model, parsing, and resolution

**Files:**
- Create: `crates/core/src/templates/frontmatter.rs`
- Modify: `crates/core/src/templates/context.rs` (`remove_forbidden_characters` visibility only)
- Modify: `crates/core/src/templates/mod.rs`
- Test: unit tests inside `crates/core/src/templates/frontmatter.rs`

**Interfaces:**
- Consumes: `super::context::remove_forbidden_characters(String) -> String` (made `pub(super)` in this task).
- Produces: `pub struct Frontmatter` with `pub fn parse(toml_str: &str, label: &str) -> TFMTResult<Self>`, `pub fn name(&self) -> Option<&str>`, `pub fn description(&self) -> Option<&str>`, `pub fn args(&self) -> &[ArgSpec]`, `pub(crate) fn resolve(&self, label: &str, positional: &[String]) -> TFMTResult<ResolvedArgs>`. `pub struct ArgSpec` with `pub fn name(&self) -> &str`, `pub fn kind(&self) -> ArgKind`, `pub fn required(&self) -> bool`, `pub fn default(&self) -> Option<&str>`, `pub fn description(&self) -> Option<&str>`. `pub enum ArgKind { String, Int, Path }` implementing `Display`. `pub(super) struct ResolvedArgs` with `pub(super) fn raw(arguments: Vec<String>) -> Self`, `pub(super) fn get_named(&self, name: &str) -> Option<minijinja::Value>`, `pub(super) fn positional(&self) -> minijinja::Value`. All consumed by Task 4 (`context.rs`) and Task 5 (`template.rs`); `Frontmatter`/`ArgSpec`/`ArgKind` also consumed by Task 7 (`fs`) and Task 9 (`tfmt`).

- [ ] **Step 1: Make the sanitization helper reusable**

In `crates/core/src/templates/context.rs`, change:

```rust
    fn remove_forbidden_characters(value: String) -> String {
```

to:

```rust
    pub(super) fn remove_forbidden_characters(value: String) -> String {
```

- [ ] **Step 2: Write the failing tests**

Create `crates/core/src/templates/frontmatter.rs` with just the data model, module declaration, and tests (implementation of `parse`/`resolve` comes in Step 4):

```rust
use std::collections::{HashMap, HashSet};

use minijinja::Value;
use serde::Deserialize;

use super::context::remove_forbidden_characters;
use crate::error::{TFMTError, TFMTResult};

#[derive(Debug, Deserialize)]
pub struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    args: Vec<ArgSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArgSpec {
    name: String,
    #[serde(rename = "type", default)]
    kind: ArgKind,
    #[serde(default)]
    required: bool,
    default: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgKind {
    #[default]
    String,
    Int,
    Path,
}

impl std::fmt::Display for ArgKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ArgKind::String => "string",
            ArgKind::Int => "int",
            ArgKind::Path => "path",
        })
    }
}

impl Frontmatter {
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[must_use]
    pub fn args(&self) -> &[ArgSpec] {
        &self.args
    }
}

impl ArgSpec {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn kind(&self) -> ArgKind {
        self.kind
    }

    #[must_use]
    pub fn required(&self) -> bool {
        self.required
    }

    #[must_use]
    pub fn default(&self) -> Option<&str> {
        self.default.as_deref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

fn describe(description: Option<&str>) -> String {
    description
        .map(std::borrow::ToOwned::to_owned)
        .unwrap_or_else(|| "(no description provided)".to_owned())
}

#[derive(Debug, Clone, Default)]
pub(super) struct ResolvedArgs {
    named: HashMap<String, Value>,
    positional: Vec<Value>,
}

impl ResolvedArgs {
    pub(super) fn raw(arguments: Vec<String>) -> Self {
        Self {
            named: HashMap::new(),
            positional: arguments.into_iter().map(Value::from).collect(),
        }
    }

    pub(super) fn get_named(&self, name: &str) -> Option<Value> {
        self.named.get(name).cloned()
    }

    pub(super) fn positional(&self) -> Value {
        Value::from(self.positional.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_frontmatter() {
        let toml = r#"
name = "Stef's layout"
description = "Group by artist and album."

args = [
    { name = "prefix", type = "path", required = true, description = "Directory prefix." },
    { name = "extra", type = "string", required = false, default = "", description = "Optional suffix." },
]
"#;

        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        assert_eq!(frontmatter.name(), Some("Stef's layout"));
        assert_eq!(
            frontmatter.description(),
            Some("Group by artist and album.")
        );
        assert_eq!(frontmatter.args().len(), 2);
        assert_eq!(frontmatter.args()[0].name(), "prefix");
        assert_eq!(frontmatter.args()[0].kind(), ArgKind::Path);
        assert!(frontmatter.args()[0].required());
        assert_eq!(frontmatter.args()[1].kind(), ArgKind::String);
        assert!(!frontmatter.args()[1].required());
    }

    #[test]
    fn parse_missing_arg_name_is_error() {
        let toml = "args = [{ type = \"string\" }]";

        assert!(Frontmatter::parse(toml, "test").is_err());
    }

    #[test]
    fn parse_unknown_type_is_error() {
        let toml = "args = [{ name = \"prefix\", type = \"float\" }]";

        assert!(Frontmatter::parse(toml, "test").is_err());
    }

    #[test]
    fn parse_duplicate_arg_names_is_error() {
        let toml = r#"
args = [
    { name = "prefix", type = "string" },
    { name = "prefix", type = "int" },
]
"#;

        let error = Frontmatter::parse(toml, "test").unwrap_err();

        assert!(matches!(error, TFMTError::DuplicateArgumentName(_, _)));
    }

    #[test]
    fn parse_defaults_kind_to_string_and_required_to_false() {
        let toml = "args = [{ name = \"prefix\" }]";

        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        assert_eq!(frontmatter.args()[0].kind(), ArgKind::String);
        assert!(!frontmatter.args()[0].required());
    }

    #[test]
    fn resolve_uses_default_when_argument_omitted() {
        let toml = "args = [{ name = \"suffix\", type = \"string\", default = \"tag\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let resolved = frontmatter.resolve("test", &[]).unwrap();

        assert_eq!(resolved.get_named("suffix").unwrap().to_string(), "tag");
    }

    #[test]
    fn resolve_errors_on_missing_required_argument() {
        let toml =
            "args = [{ name = \"prefix\", type = \"string\", required = true }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let error = frontmatter.resolve("test", &[]).unwrap_err();

        assert!(matches!(error, TFMTError::MissingRequiredArgument(_, _, _)));
    }

    #[test]
    fn resolve_errors_on_too_many_positional_arguments() {
        let toml = "args = [{ name = \"prefix\", type = \"string\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let error = frontmatter
            .resolve("test", &["a".to_owned(), "b".to_owned()])
            .unwrap_err();

        assert!(matches!(error, TFMTError::TooManyArguments(_, _, _)));
    }

    #[test]
    fn resolve_allows_extra_positional_arguments_when_no_args_declared() {
        let toml = "name = \"No args\"";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let resolved = frontmatter
            .resolve("test", &["a".to_owned(), "b".to_owned()])
            .unwrap();

        assert!(resolved.get_named("a").is_none());
    }

    #[test]
    fn resolve_errors_on_int_parse_failure() {
        let toml = "args = [{ name = \"count\", type = \"int\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let error = frontmatter
            .resolve("test", &["not-a-number".to_owned()])
            .unwrap_err();

        assert!(matches!(error, TFMTError::InvalidArgumentValue(_, _, _, _)));
    }

    #[test]
    fn resolve_sanitizes_string_argument() {
        let toml = "args = [{ name = \"tag\", type = \"string\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let resolved =
            frontmatter.resolve("test", &["a:b*c.".to_owned()]).unwrap();

        assert_eq!(resolved.get_named("tag").unwrap().to_string(), "abc");
    }

    #[test]
    fn resolve_normalizes_path_argument_trailing_separators() {
        let toml = "args = [{ name = \"prefix\", type = \"path\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        for raw in ["a/b", "a/b/", "a/b//", "a\\b\\"] {
            let resolved =
                frontmatter.resolve("test", &[raw.to_owned()]).unwrap();

            assert_eq!(
                resolved.get_named("prefix").unwrap().to_string(),
                "a/b/"
            );
        }
    }

    #[test]
    fn resolve_sanitizes_each_path_segment() {
        let toml = "args = [{ name = \"prefix\", type = \"path\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let resolved =
            frontmatter.resolve("test", &["a:b/c*d".to_owned()]).unwrap();

        assert_eq!(
            resolved.get_named("prefix").unwrap().to_string(),
            "ab/cd/"
        );
    }
}
```

Register the module in `crates/core/src/templates/mod.rs`:

```rust
mod context;
mod frontmatter;
mod template;

pub use frontmatter::{ArgKind, ArgSpec, Frontmatter};
pub use template::Template;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p tfmttools-core templates::frontmatter -- --nocapture`
Expected: FAIL to compile with `` no function `parse` found for struct `Frontmatter` `` (and similarly for `resolve`)

- [ ] **Step 4: Implement `parse` and `resolve`**

Add to `crates/core/src/templates/frontmatter.rs`, after the `describe` free function:

```rust
impl Frontmatter {
    pub fn parse(toml_str: &str, label: &str) -> TFMTResult<Self> {
        let frontmatter: Frontmatter = toml::from_str(toml_str)
            .map_err(|error| TFMTError::FrontmatterParse(label.to_owned(), error))?;

        let mut seen = HashSet::new();

        for arg in &frontmatter.args {
            if !seen.insert(arg.name.as_str()) {
                return Err(TFMTError::DuplicateArgumentName(
                    label.to_owned(),
                    arg.name.clone(),
                ));
            }
        }

        Ok(frontmatter)
    }

    pub(crate) fn resolve(
        &self,
        label: &str,
        positional: &[String],
    ) -> TFMTResult<ResolvedArgs> {
        if !self.args.is_empty() && positional.len() > self.args.len() {
            return Err(TFMTError::TooManyArguments(
                label.to_owned(),
                self.args.len(),
                positional.len(),
            ));
        }

        let mut named = HashMap::new();
        let mut ordered = Vec::with_capacity(self.args.len());

        for (index, spec) in self.args.iter().enumerate() {
            let raw = positional.get(index).cloned().or_else(|| spec.default.clone());

            let value = match raw {
                Some(raw) => spec.coerce(label, &raw)?,
                None if spec.required => {
                    return Err(TFMTError::MissingRequiredArgument(
                        label.to_owned(),
                        spec.name.clone(),
                        describe(spec.description.as_deref()),
                    ));
                },
                None => Value::UNDEFINED,
            };

            named.insert(spec.name.clone(), value.clone());
            ordered.push(value);
        }

        Ok(ResolvedArgs { named, positional: ordered })
    }
}

impl ArgSpec {
    fn coerce(&self, label: &str, raw: &str) -> TFMTResult<Value> {
        match self.kind {
            ArgKind::Int => raw.parse::<i64>().map(Value::from).map_err(|_| {
                TFMTError::InvalidArgumentValue(
                    label.to_owned(),
                    self.name.clone(),
                    describe(self.description.as_deref()),
                    raw.to_owned(),
                )
            }),
            ArgKind::String => Ok(Value::from(remove_forbidden_characters(raw.to_owned()))),
            ArgKind::Path => Ok(Value::from(sanitize_path(raw))),
        }
    }
}

fn sanitize_path(raw: &str) -> String {
    let segments: Vec<String> = raw
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| remove_forbidden_characters(segment.to_owned()))
        .collect();

    if segments.is_empty() {
        String::new()
    } else {
        format!("{}/", segments.join("/"))
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p tfmttools-core templates::frontmatter`
Expected: PASS, 13 tests passed

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/templates/frontmatter.rs crates/core/src/templates/mod.rs crates/core/src/templates/context.rs
git commit -m "Add Frontmatter/ArgSpec/ArgKind data model with parsing and resolution"
```

---

### Task 4: Wire `ResolvedArgs` into `AudioFileContext`

**Files:**
- Modify: `crates/core/src/templates/context.rs`

**Interfaces:**
- Consumes: `super::frontmatter::ResolvedArgs` (from Task 3): `ResolvedArgs::get_named(&self, &str) -> Option<Value>`, `ResolvedArgs::positional(&self) -> Value`.
- Produces: `AudioFileContext::safe(audio_file: AudioFile, resolved_args: ResolvedArgs) -> Self` — consumed by Task 5.

This is a small, mechanical field-swap. There's no new independently-testable behavior beyond what Task 3's `resolve()` tests already cover and what Task 5/7's integration tests will cover, so this task is verified by compilation plus the existing `templates` test suite staying green, not a new test.

- [ ] **Step 1: Update the struct and constructor**

In `crates/core/src/templates/context.rs`, change:

```rust
use crate::action::FORBIDDEN_CHARACTERS;
use crate::audiofile::AudioFile;
use crate::item_keys::ItemKeys;

#[derive(Debug)]
pub struct AudioFileContext {
    audio_file: AudioFile,
    arguments: Vec<String>,
}

impl AudioFileContext {
    pub fn safe(audio_file: AudioFile, arguments: Vec<String>) -> Self {
        Self { audio_file, arguments }
    }
```

to:

```rust
use super::frontmatter::ResolvedArgs;
use crate::action::FORBIDDEN_CHARACTERS;
use crate::audiofile::AudioFile;
use crate::item_keys::ItemKeys;

#[derive(Debug)]
pub struct AudioFileContext {
    audio_file: AudioFile,
    resolved_args: ResolvedArgs,
}

impl AudioFileContext {
    pub fn safe(audio_file: AudioFile, resolved_args: ResolvedArgs) -> Self {
        Self { audio_file, resolved_args }
    }
```

- [ ] **Step 2: Update `get_value`**

Change:

```rust
impl Object for AudioFileContext {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let field = key.as_str()?;
        let normalized_field = field.to_lowercase();

        match normalized_field.as_str() {
            "args" | "arguments" => Some(self.arguments.clone().into()),
            "date" => self.get_date(),
```

to:

```rust
impl Object for AudioFileContext {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let field = key.as_str()?;
        let normalized_field = field.to_lowercase();

        if let Some(value) = self.resolved_args.get_named(&normalized_field) {
            return Some(value);
        }

        match normalized_field.as_str() {
            "args" | "arguments" => Some(self.resolved_args.positional()),
            "date" => self.get_date(),
```

(the trailing `_ => { ... }` arm is unchanged)

- [ ] **Step 3: Verify compilation (expect failure — `Template::render` still passes a `Vec<String>`)**

Run: `cargo check -p tfmttools-core`
Expected: FAIL with a type mismatch in `template.rs` (`Vec<String>` vs `ResolvedArgs`) — resolved by Task 5.

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/templates/context.rs
git commit -m "Thread ResolvedArgs through AudioFileContext"
```

(Committing mid-red is acceptable here since Task 5 is the very next task and fixes compilation; if you prefer a strictly-green history, squash Tasks 4 and 5's commits together instead.)

---

### Task 5: Wire `Frontmatter` resolution into `Template`

**Files:**
- Modify: `crates/core/src/templates/template.rs`

**Interfaces:**
- Consumes: `Frontmatter::resolve(&self, label: &str, positional: &[String]) -> TFMTResult<ResolvedArgs>` and `Frontmatter::args(&self) -> &[ArgSpec]` (Task 3); `AudioFileContext::safe(AudioFile, ResolvedArgs) -> AudioFileContext` (Task 4).
- Produces: `Template::new(inner, lookup_name: &str, display_name: String, description: Option<String>, arguments: Vec<String>, frontmatter: Option<&Frontmatter>) -> TFMTResult<Self>`, `Template::for_display(inner, display_name: String, description: Option<String>, declared_args: Vec<ArgSpec>) -> Self`, `Template::declared_args(&self) -> &[ArgSpec]` — consumed by Task 7 (`TemplateLoader`) and Task 9 (`list_templates.rs`).

- [ ] **Step 1: Write the failing test**

Create a test module at the bottom of `crates/core/src/templates/template.rs` (there is no existing test module in this file):

```rust
#[cfg(test)]
mod tests {
    use minijinja::Environment;

    use super::*;
    use crate::templates::Frontmatter;

    fn build_minijinja_template(
        env: &Environment<'static>,
        name: &'static str,
    ) -> minijinja::Template<'_, 'static> {
        env.get_template(name).unwrap()
    }

    #[test]
    fn new_without_frontmatter_keeps_raw_positional_arguments() {
        let mut env = Environment::new();
        env.add_template("t", "{{ args[0] }}").unwrap();
        let inner = build_minijinja_template(&env, "t");

        let template = Template::new(
            inner,
            "t",
            "t".to_owned(),
            None,
            vec!["raw:value".to_owned()],
            None,
        )
        .unwrap();

        assert_eq!(template.declared_args().len(), 0);
    }

    #[test]
    fn new_with_frontmatter_errors_on_missing_required_argument() {
        let mut env = Environment::new();
        env.add_template("t", "{{ prefix }}").unwrap();
        let inner = build_minijinja_template(&env, "t");

        let frontmatter = Frontmatter::parse(
            "args = [{ name = \"prefix\", type = \"string\", required = true }]",
            "t",
        )
        .unwrap();

        let error = Template::new(
            inner,
            "t",
            "t".to_owned(),
            None,
            Vec::new(),
            Some(&frontmatter),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            crate::error::TFMTError::MissingRequiredArgument(_, _, _)
        ));
    }

    #[test]
    fn for_display_never_resolves_arguments() {
        let mut env = Environment::new();
        env.add_template("t", "{{ prefix }}").unwrap();
        let inner = build_minijinja_template(&env, "t");

        let frontmatter = Frontmatter::parse(
            "args = [{ name = \"prefix\", type = \"string\", required = true }]",
            "t",
        )
        .unwrap();

        let template = Template::for_display(
            inner,
            "t".to_owned(),
            None,
            frontmatter.args().to_vec(),
        );

        assert_eq!(template.declared_args().len(), 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p tfmttools-core templates::template -- --nocapture`
Expected: FAIL to compile — `Template::new` still takes the old 4-argument signature and returns `Self`, not `TFMTResult<Self>`; `declared_args`/`for_display` don't exist yet.

- [ ] **Step 3: Implement the new `Template`**

Replace the full contents of `crates/core/src/templates/template.rs` with:

```rust
use minijinja::Value;

use super::context::AudioFileContext;
use super::frontmatter::ResolvedArgs;
use super::{ArgSpec, Frontmatter};
use crate::audiofile::AudioFile;
use crate::error::TFMTResult;

#[derive(Debug)]
pub struct Template<'templates, 'source> {
    inner: minijinja::Template<'templates, 'source>,
    name: String,
    description: Option<String>,
    declared_args: Vec<ArgSpec>,
    resolved: ResolvedArgs,
}

impl<'templates, 'source> Template<'templates, 'source> {
    pub fn new(
        inner: minijinja::Template<'templates, 'source>,
        lookup_name: &str,
        display_name: String,
        description: Option<String>,
        arguments: Vec<String>,
        frontmatter: Option<&Frontmatter>,
    ) -> TFMTResult<Self> {
        let (declared_args, resolved) = match frontmatter {
            Some(frontmatter) => (
                frontmatter.args().to_vec(),
                frontmatter.resolve(lookup_name, &arguments)?,
            ),
            None => (Vec::new(), ResolvedArgs::raw(arguments)),
        };

        Ok(Self { inner, name: display_name, description, declared_args, resolved })
    }

    #[must_use]
    pub fn for_display(
        inner: minijinja::Template<'templates, 'source>,
        display_name: String,
        description: Option<String>,
        declared_args: Vec<ArgSpec>,
    ) -> Self {
        Self {
            inner,
            name: display_name,
            description,
            declared_args,
            resolved: ResolvedArgs::default(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    #[must_use]
    pub fn declared_args(&self) -> &[ArgSpec] {
        &self.declared_args
    }

    pub fn render(&self, audio_file: &AudioFile) -> TFMTResult<String> {
        let context =
            AudioFileContext::safe(audio_file.to_owned(), self.resolved.clone());

        let context_value = Value::from_object(context);

        let output = self.inner.render(&context_value)?;

        Ok(output)
    }
}
```

Note `ResolvedArgs` needs `Default` (already derived in Task 3).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tfmttools-core templates::template`
Expected: PASS, 3 tests passed

- [ ] **Step 5: Verify the whole `core` crate compiles**

Run: `cargo check -p tfmttools-core`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/templates/template.rs
git commit -m "Resolve frontmatter arguments in Template::new; add Template::for_display"
```

---

### Task 6: `TemplateLoader` — frontmatter stripping, side table, deprecation warnings, hard error

**Files:**
- Modify: `crates/fs/src/template.rs`
- Test: unit tests inside `crates/fs/src/template.rs` (new `#[cfg(test)] mod tests` — none exists in this file today)

**Interfaces:**
- Consumes: `tfmttools_core::templates::{Frontmatter, Template}` (Tasks 3 and 5); `Frontmatter::parse(&str, &str) -> TFMTResult<Frontmatter>`; `Template::new(..) -> TFMTResult<Template>`; `Template::for_display(..) -> Template`.
- Produces: `TemplateLoader::get_template(&self, name: &str, arguments: Vec<String>) -> TFMTResult<Option<Template>>` (return type changed from `Option<Template>`) and `TemplateLoader::get_all_templates(&self) -> Vec<Template>` (unchanged signature, now display-only) — consumed by Task 8 (`discovery.rs`).

- [ ] **Step 1: Write the failing tests**

Add to the bottom of `crates/fs/src/template.rs`:

```rust
#[cfg(test)]
mod tests {
    use tfmttools_core::error::TFMTError;

    use super::*;

    #[test]
    fn split_frontmatter_returns_none_when_absent() {
        let source = "{{ artist }}/{{ title }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source.clone()).unwrap();

        assert_eq!(body, source);
        assert!(frontmatter.is_none());
    }

    #[test]
    fn split_frontmatter_parses_present_block() {
        let source = "+++\nname = \"Test\"\n+++\n{{ artist }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert_eq!(frontmatter.unwrap().name(), Some("Test"));
    }

    #[test]
    fn split_frontmatter_errors_when_unterminated() {
        let source = "+++\nname = \"Test\"\n{{ artist }}".to_owned();

        let error = TemplateLoader::split_frontmatter("test", source).unwrap_err();

        assert!(matches!(error, TFMTError::UnterminatedFrontmatter(_)));
    }

    #[test]
    fn split_frontmatter_errors_when_body_uses_indexed_args() {
        let source = "+++\nname = \"Test\"\n+++\n{{ args[0] }}".to_owned();

        let error = TemplateLoader::split_frontmatter("test", source).unwrap_err();

        assert!(matches!(error, TFMTError::IndexedArgsWithFrontmatter(_)));
    }

    #[test]
    fn split_frontmatter_allows_indexed_args_without_frontmatter() {
        let source = "{{ args[0] }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source.clone()).unwrap();

        assert_eq!(body, source);
        assert!(frontmatter.is_none());
    }

    #[test]
    fn read_script_populates_frontmatter_side_table() {
        let script = "+++\nname = \"Test\"\n+++\n{{ artist }}";

        let loader = TemplateLoader::read_script(script).unwrap();

        assert_eq!(
            loader
                .frontmatters
                .get(TemplateLoader::DEFAULT_SCRIPT_NAME)
                .unwrap()
                .name(),
            Some("Test")
        );
    }

    #[test]
    fn read_script_without_frontmatter_has_empty_side_table() {
        let loader = TemplateLoader::read_script("{{ args[0] }}").unwrap();

        assert!(loader.frontmatters.is_empty());
    }

    #[test]
    fn get_template_errors_on_missing_required_argument() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\", required = true }]\n+++\n{{ prefix }}";

        let loader = TemplateLoader::read_script(script).unwrap();

        let error = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap_err();

        assert!(matches!(error, TFMTError::MissingRequiredArgument(_, _, _)));
    }

    #[test]
    fn get_template_resolves_declared_arguments() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\" }]\n+++\n{{ prefix }}";

        let loader = TemplateLoader::read_script(script).unwrap();

        let template = loader
            .get_template(
                TemplateLoader::DEFAULT_SCRIPT_NAME,
                vec!["a".to_owned()],
            )
            .unwrap();

        assert!(template.is_some());
    }

    #[test]
    fn get_all_templates_never_errors_for_required_arguments() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\", required = true }]\n+++\n{{ prefix }}";

        let loader = TemplateLoader::read_script(script).unwrap();

        let templates = loader.get_all_templates();

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].declared_args().len(), 1);
    }

    #[test]
    fn description_comes_only_from_frontmatter_when_present() {
        let script = "+++\ndescription = \"From frontmatter.\"\n+++\n{# Leading comment #}\n{{ artist }}";

        let loader = TemplateLoader::read_script(script).unwrap();
        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap()
            .unwrap();

        assert_eq!(template.description(), Some(&"From frontmatter.".to_owned()));
    }

    #[test]
    fn display_name_falls_back_to_lookup_name_without_override() {
        let loader = TemplateLoader::read_script("{{ artist }}").unwrap();
        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap()
            .unwrap();

        assert_eq!(template.name(), TemplateLoader::DEFAULT_SCRIPT_NAME);
    }

    #[test]
    fn display_name_uses_frontmatter_override() {
        let script = "+++\nname = \"Pretty Name\"\n+++\n{{ artist }}";

        let loader = TemplateLoader::read_script(script).unwrap();
        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap()
            .unwrap();

        assert_eq!(template.name(), "Pretty Name");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p tfmttools-fs template:: -- --nocapture`
Expected: FAIL to compile — `split_frontmatter`, the `frontmatters` field, and the new `get_template` return type don't exist yet.

- [ ] **Step 3: Implement**

Replace the full contents of `crates/fs/src/template.rs` with:

```rust
use std::collections::HashMap;
use std::sync::LazyLock;

use camino::Utf8Path;
use fs_err as fs;
use minijinja::{Environment, Value, escape_formatter};
use regex::Regex;
use tfmttools_core::error::{TFMTError, TFMTResult};
use tfmttools_core::templates::{Frontmatter, Template};
use tfmttools_core::util::{Utf8Directory, Utf8PathExt};

use crate::PathIterator;

pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

const FRONTMATTER_FENCE: &str = "+++";

#[derive(Debug)]
pub struct TemplateLoader<'tl> {
    template_names: Vec<String>,
    frontmatters: HashMap<String, Frontmatter>,
    environment: Environment<'tl>,
}

impl<'tl> TemplateLoader<'tl> {
    pub const DEFAULT_SCRIPT_NAME: &'static str = "script";

    pub fn read_directory(
        template_directory: &Utf8Directory,
    ) -> TFMTResult<Self> {
        let mut template_names = Vec::new();
        let mut frontmatters = HashMap::new();
        let mut environment = Self::create_environment();

        let iter = PathIterator::single_directory(template_directory.as_path())
            .flatten()
            .filter(|path| Self::path_is_template(path));

        for template_path in iter {
            let name = template_path
                .file_stem()
                .expect("Template::path_is_template should only return files.")
                .to_owned();

            let source = fs::read_to_string(&template_path)?;

            Self::register_template(
                &mut environment,
                &mut frontmatters,
                &name,
                source,
            )?;

            template_names.push(name);
        }

        Ok(Self { template_names, frontmatters, environment })
    }

    pub fn read_filename(path: &Utf8Path, name: &str) -> TFMTResult<Self> {
        let mut frontmatters = HashMap::new();
        let mut environment = Self::create_environment();

        let source = fs::read_to_string(path)?;

        Self::register_template(&mut environment, &mut frontmatters, name, source)?;

        Ok(Self {
            template_names: vec![name.to_owned()],
            frontmatters,
            environment,
        })
    }

    pub fn read_script(script: &str) -> TFMTResult<Self> {
        let mut frontmatters = HashMap::new();
        let mut environment = Self::create_environment();

        Self::register_template(
            &mut environment,
            &mut frontmatters,
            Self::DEFAULT_SCRIPT_NAME,
            script.to_owned(),
        )?;

        Ok(Self {
            template_names: vec![Self::DEFAULT_SCRIPT_NAME.to_owned()],
            frontmatters,
            environment,
        })
    }

    pub fn get_template(
        &'_ self,
        name: &str,
        arguments: Vec<String>,
    ) -> TFMTResult<Option<Template<'_, '_>>> {
        let Ok(minijinja_template) = self.environment.get_template(name) else {
            return Ok(None);
        };

        let frontmatter = self.frontmatters.get(name);

        let description = match frontmatter {
            Some(frontmatter) => frontmatter.description().map(ToOwned::to_owned),
            None => Self::description(minijinja_template.source()),
        };

        let display_name = frontmatter
            .and_then(|frontmatter| frontmatter.name())
            .map_or_else(|| name.to_owned(), ToOwned::to_owned);

        let template = Template::new(
            minijinja_template,
            name,
            display_name,
            description,
            arguments,
            frontmatter,
        )?;

        Ok(Some(template))
    }

    pub fn get_all_templates(&'_ self) -> Vec<Template<'_, '_>> {
        self.template_names
            .iter()
            .map(|name| {
                let minijinja_template = self.environment.get_template(name).expect(
                    "TemplateLoader::template_names should not contain names of non-existent templates.",
                );

                let frontmatter = self.frontmatters.get(name);

                let description = match frontmatter {
                    Some(frontmatter) => frontmatter.description().map(ToOwned::to_owned),
                    None => Self::description(minijinja_template.source()),
                };

                let display_name = frontmatter
                    .and_then(|frontmatter| frontmatter.name())
                    .map_or_else(|| name.to_owned(), ToOwned::to_owned);

                let declared_args = frontmatter
                    .map(|frontmatter| frontmatter.args().to_vec())
                    .unwrap_or_default();

                Template::for_display(
                    minijinja_template,
                    display_name,
                    description,
                    declared_args,
                )
            })
            .collect()
    }

    fn register_template(
        environment: &mut Environment<'tl>,
        frontmatters: &mut HashMap<String, Frontmatter>,
        name: &str,
        source: String,
    ) -> TFMTResult<()> {
        let (body, frontmatter) = Self::split_frontmatter(name, source)?;

        if frontmatter.is_none() {
            Self::warn_on_deprecated_usage(name, &body);
        }

        if let Some(frontmatter) = frontmatter {
            frontmatters.insert(name.to_owned(), frontmatter);
        }

        environment.add_template_owned(name.to_owned(), body)?;

        Ok(())
    }

    fn split_frontmatter(
        label: &str,
        source: String,
    ) -> TFMTResult<(String, Option<Frontmatter>)> {
        static RE_FRONTMATTER: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?s)\A\+\+\+[ \t]*\r?\n(?P<toml>.*?)\r?\n\+\+\+[ \t]*\r?\n?")
                .unwrap()
        });

        if !source.starts_with(FRONTMATTER_FENCE) {
            return Ok((source, None));
        }

        let Some(captures) = RE_FRONTMATTER.captures(&source) else {
            return Err(TFMTError::UnterminatedFrontmatter(label.to_owned()));
        };

        let whole_match = captures.get(0).expect("group 0 always matches");
        let toml_text = &captures["toml"];

        let frontmatter = Frontmatter::parse(toml_text, label)?;

        let body = source[whole_match.end()..].to_owned();

        if Self::body_uses_indexed_args(&body) {
            return Err(TFMTError::IndexedArgsWithFrontmatter(label.to_owned()));
        }

        Ok((body, Some(frontmatter)))
    }

    fn body_uses_indexed_args(body: &str) -> bool {
        static RE_ARGS_INDEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"args\s*\[").unwrap());

        RE_ARGS_INDEX.is_match(body)
    }

    fn warn_on_deprecated_usage(label: &str, body: &str) {
        if Self::body_uses_indexed_args(body) {
            tracing::warn!(
                "Template '{label}' uses positional `args[N]` without frontmatter; declare arguments to migrate."
            );
        }

        if Self::description(body).is_some() {
            tracing::warn!(
                "Template '{label}' uses a leading comment as its description; move it to frontmatter's `description` field."
            );
        }
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

    fn path_is_template(path: &Utf8Path) -> bool {
        path.extension()
            .is_some_and(|string| TEMPLATE_EXTENSIONS.contains(&string))
    }

    fn create_environment() -> Environment<'tl> {
        let mut env = Environment::new();

        env.set_formatter(|out, state, value| {
            escape_formatter(
                out,
                state,
                if value.is_none() { &Value::UNDEFINED } else { value },
            )
        });

        env.add_filter("year", Self::year);
        env.add_filter("zero_pad", Self::zero_pad);

        env
    }

    fn year(date: &Value) -> Result<String, minijinja::Error> {
        static RE_ISO: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(\d{4})-\d{2}-\d{2}").unwrap());

        static RE_AMBIGUOUS: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\d{2}-\d{2}-(\d{4})").unwrap());

        static RE_YEAR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(\d{4})").unwrap());

        let date = date.to_string();

        if let Some(m) = RE_ISO.find(&date) {
            let year = &m.as_str()[0..4];

            Ok(year.to_owned())
        } else if let Some(m) = RE_AMBIGUOUS.find(&date) {
            let string = m.as_str();

            let year = &string[string.len() - 4..string.len()];

            Ok(year.to_owned())
        } else if let Some(m) = RE_YEAR.find(&date) {
            Ok(m.as_str().to_owned())
        } else {
            Err(minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                format!("Unable to parse date: {date}"),
            ))
        }
    }

    fn zero_pad(value: &Value, width: usize) -> String {
        format!("{value:0>width$}")
    }
}
```

(The `#[cfg(test)] mod tests { ... }` block from Step 1 stays appended after this.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tfmttools-fs template::`
Expected: PASS, 14 tests passed

- [ ] **Step 5: Commit**

```bash
git add crates/fs/src/template.rs
git commit -m "Strip and parse script frontmatter in TemplateLoader"
```

---

### Task 7: Update `discovery.rs` for `get_template`'s new return type

**Files:**
- Modify: `crates/tfmt/src/commands/rename/discovery.rs`

**Interfaces:**
- Consumes: `TemplateLoader::get_template(&self, &str, Vec<String>) -> TFMTResult<Option<Template>>` (Task 6).

No new test — this crate's rename flow is covered by the CLI fixtures added in Task 10. Verify via `cargo check`/`cargo build`.

- [ ] **Step 1: Update the call site**

In `crates/tfmt/src/commands/rename/discovery.rs`, change:

```rust
    let template = resolved
        .loader
        .get_template(&resolved.template_name, resolved.arguments.clone())
        .ok_or(eyre!("Unable to find template: {}", resolved.template_name))?;
```

to:

```rust
    let template = resolved
        .loader
        .get_template(&resolved.template_name, resolved.arguments.clone())?
        .ok_or(eyre!("Unable to find template: {}", resolved.template_name))?;
```

(single added `?` after the `get_template(...)` call — `TFMTError` converts into `color_eyre::Report` automatically since it implements `std::error::Error`)

- [ ] **Step 2: Verify the whole workspace compiles**

Run: `cargo build --workspace`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/tfmt/src/commands/rename/discovery.rs
git commit -m "Propagate frontmatter argument-resolution errors from get_template"
```

---

### Task 8: `list-templates` displays declared arguments

**Files:**
- Modify: `crates/tfmt/src/commands/list_templates.rs`

**Interfaces:**
- Consumes: `Template::declared_args(&self) -> &[ArgSpec]` (Task 5); `ArgSpec::{name, kind, required, default, description}` and `ArgKind: Display` (Task 3).

- [ ] **Step 1: Write the failing test**

Add to the bottom of `crates/tfmt/src/commands/list_templates.rs`:

```rust
#[cfg(test)]
mod tests {
    use tfmttools_fs::TemplateLoader;

    use super::*;

    #[test]
    fn format_template_lists_declared_arguments() {
        let script = "+++\nname = \"Test Template\"\ndescription = \"A test template.\"\n\nargs = [\n    { name = \"prefix\", type = \"path\", required = true, description = \"Directory prefix.\" },\n    { name = \"suffix\", type = \"string\", required = false, default = \"\", description = \"Optional suffix.\" },\n]\n+++\n{{- prefix -}}{{- suffix -}}";

        let loader = TemplateLoader::read_script(script).unwrap();
        let templates = loader.get_all_templates();

        let formatted = format_template(&templates[0]);

        assert!(formatted.contains("Test Template: A test template."));
        assert!(formatted.contains("prefix"));
        assert!(formatted.contains("path"));
        assert!(formatted.contains("required"));
        assert!(formatted.contains("suffix"));
        assert!(formatted.contains("string"));
        assert!(formatted.contains("default"));
        assert!(formatted.contains("Optional suffix."));
    }

    #[test]
    fn format_template_without_args_has_no_arg_lines() {
        let loader = TemplateLoader::read_script("{{ artist }}").unwrap();
        let templates = loader.get_all_templates();

        let formatted = format_template(&templates[0]);

        assert_eq!(formatted, TemplateLoader::DEFAULT_SCRIPT_NAME);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p tfmttools list_templates:: -- --nocapture`
Expected: FAIL — first test fails because `format_template` doesn't yet print argument lines (only the header assertion passes; the `prefix`/`path`/etc. assertions fail).

- [ ] **Step 3: Implement**

Replace `format_template` in `crates/tfmt/src/commands/list_templates.rs`:

```rust
use color_eyre::Result;
use textwrap::Options;
use tfmttools_core::templates::{ArgSpec, Template};
use tfmttools_core::util::Utf8Directory;
use tfmttools_fs::TemplateLoader;

use crate::ui::terminal_width;

pub fn list_templates(template_directory: &Utf8Directory) -> Result<()> {
    let loader = TemplateLoader::read_directory(template_directory)?;

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

    Ok(())
}

fn format_template(template: &Template) -> String {
    let name = template.name();

    let header_string = if let Some(description) = template.description() {
        format!("{name}: {description}")
    } else {
        name.to_owned()
    };

    let header = textwrap::fill(
        &header_string,
        Options::new(terminal_width())
            .subsequent_indent(&" ".repeat(name.len() + 2)),
    );

    let arg_lines: Vec<String> =
        template.declared_args().iter().map(format_arg).collect();

    if arg_lines.is_empty() {
        header
    } else {
        format!("{header}\n{}", arg_lines.join("\n"))
    }
}

fn format_arg(arg: &ArgSpec) -> String {
    let requirement = if arg.required() {
        "required".to_owned()
    } else if let Some(default) = arg.default() {
        format!("default: {default:?}")
    } else {
        "optional".to_owned()
    };

    let description = arg
        .description()
        .map(|description| format!(" - {description}"))
        .unwrap_or_default();

    format!("    {} ({}, {}){}", arg.name(), arg.kind(), requirement, description)
}
```

(The `#[cfg(test)] mod tests { ... }` block from Step 1 stays appended after this.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tfmttools list_templates::`
Expected: PASS, 2 tests passed

- [ ] **Step 5: Commit**

```bash
git add crates/tfmt/src/commands/list_templates.rs
git commit -m "Show declared frontmatter arguments in tfmt list-templates"
```

---

### Task 9: Update `README.md`

**Files:**
- Modify: `README.md`

No tests — documentation only. Verify by re-reading the rendered section for accuracy against Task 1-8's actual behavior.

- [ ] **Step 1: Remove the TODO line**

In `README.md`, change:

```markdown
## TODO

- Add frontmatter to script with basic types.
- Testing on windows?
```

to:

```markdown
## TODO

- Testing on windows?
```

- [ ] **Step 2: Add the Script Frontmatter section**

In `README.md`, insert a new section directly after the existing `### Templates` section (after the paragraph ending "...tags cannot accidentally create extra directories.") and before `### Safety`:

```markdown
### Script Frontmatter

A script may start with an optional TOML frontmatter block, fenced by `+++` on
its own line at the very start of the file:

```jinja
+++
name = "Stef's layout"
description = "Group by artist and album, with a directory prefix."

args = [
    { name = "prefix", type = "path", required = true, description = "Directory prefix to place output under." },
    { name = "extra", type = "string", required = false, default = "", description = "Optional suffix tag." },
]
+++
{{- prefix -}}
{{- albumartist or artist -}}
...
```

- `name` overrides the display name shown by `tfmt list-templates`. The
  lookup name used by `--template <name>` is always the filename stem.
- `description` becomes the template's description; when frontmatter is
  present, no other description source (such as a leading comment) is used.
- Each entry in `args` is matched to CLI-supplied positional values by
  declaration order and is available in the script both by name (`{{ prefix }}`)
  and by index (`{{ args[0] }}`).
  - `type` is one of `string` (default), `int`, or `path`.
  - `required = true` makes omitting the argument (with no `default`) a hard
    error before rendering.
  - `string` and `path` values are sanitized with the same forbidden-character
    rules as tag values (see Filename Sanitization below); `path` values are
    additionally split on `/`/`\`, sanitized segment-by-segment, and rejoined
    with exactly one trailing `/`.
  - `int` values that fail to parse are a hard error before rendering.
- Supplying more positional arguments than a script declares is a hard error,
  unless the script declares no `args` at all.
- Scripts without a frontmatter block are unaffected: `args[N]` remains raw,
  unsanitized, and unlimited in count, exactly as before. Using `args[N]`
  without frontmatter, or relying on a leading comment as the description,
  now logs a deprecation warning steering scripts toward frontmatter. Using
  `args[N]` in a script that *does* have a frontmatter block is a hard error,
  even if that block declares no `args`.
```

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "Document script frontmatter in README"
```

---

### Task 10: Update `examples/stef.tfmt`

**Files:**
- Modify: `examples/stef.tfmt`

No tests — this is a real-world demo script referenced by the README's usage examples (`tfmt --dry-run rename -t examples/stef.tfmt`), not by any automated fixture.

- [ ] **Step 1: Rewrite with frontmatter**

Replace the full contents of `examples/stef.tfmt`:

```jinja
+++
name = "Stef's layout"
description = "Group by artist and album, with a directory prefix."

args = [
    { name = "prefix", type = "path", required = true, description = "Directory prefix to place output under." },
]
+++
{{- prefix -}}
{{- albumartist or artist -}}
/
{%- if album -%}
    {%- if date -%}
      {{- date | year -}}
      {{- albumsort and "." ~ (albumsort | zero_pad(2)) -}}
      {{- " - " -}}
    {%- endif -%}
    {{- album -}}
    /
{%- endif -%}
{{- discnumber and discnumber | zero_pad(1) -}}
{{- tracknumber and tracknumber | zero_pad(2) ~ " - " -}}
{{- albumartist and artist ~ " - " -}}
{{- title -}}
```

- [ ] **Step 2: Manually verify it loads**

Run: `cargo run -p tfmttools --bin tfmt -- list-templates -t examples/`

(Adjust the exact flag for pointing at a single file vs. directory to whatever this repo's CLI actually exposes for ad hoc paths — check `tfmt --help rename` and `tfmt list-templates --help` if unsure of flag names.)

Expected: Output includes `Stef's layout: Group by artist and album, with a directory prefix.` followed by a `prefix (path, required) - Directory prefix to place output under.` line, with no errors.

- [ ] **Step 3: Commit**

```bash
git add examples/stef.tfmt
git commit -m "Convert examples/stef.tfmt to use frontmatter"
```

---

### Task 11: CLI integration fixtures for frontmatter

**Files:**
- Create: `tests/fixtures/cli/template/frontmatter_prefix.tfmt`
- Create: `tests/fixtures/cli/cases/frontmatter_prefix.case.json`
- Create: `tests/fixtures/cli/cases/frontmatter_prefix_missing_required.case.json`

These fixtures reuse the exact audio inputs and known-correct checksums from the existing `typical_input.case.json` fixture (`tests/fixtures/cli/cases/typical_input.case.json`), since `frontmatter_prefix.tfmt` renders identically to `typical_input.tfmt` when given the same `output_dir/` prefix value (the `path`-type sanitization of `"output_dir/"` — a string with no forbidden characters — is a no-op).

- [ ] **Step 1: Create the frontmatter-declared template**

Create `tests/fixtures/cli/template/frontmatter_prefix.tfmt`:

```jinja
+++
name = "Frontmatter Prefix Demo"
description = "Group by artist and album under a required directory prefix, declared via frontmatter."

args = [
    { name = "prefix", type = "path", required = true, description = "Directory prefix to place output under." },
]
+++
{{- prefix -}}
{{- albumartist or artist -}}
/
{%- if album -%}
    {%- if date -%}
      {{- date | year -}}
      {{- albumsort and "." ~ (albumsort | zero_pad(2)) -}}
      {{- " - " -}}
    {%- endif -%}
    {{- album -}}
    /
{%- endif -%}
{{- discnumber and discnumber | zero_pad(1) -}}
{{- tracknumber and tracknumber | zero_pad(2) ~ " - " -}}
{{- albumartist and artist ~ " - " -}}
{{- title -}}
```

- [ ] **Step 2: Create the success-path fixture case**

Create `tests/fixtures/cli/cases/frontmatter_prefix.case.json` (identical `initial-state`/`apply` file lists to `typical_input.case.json`, since the rendered output paths and checksums are unchanged):

```json
{
  "description": "Single apply, undo and redo of frontmatter_prefix.tfmt, exercising a resolved required frontmatter argument",
  "expectations": {
    "initial-state": [
      { "path": "input/Amon Amarth - Under Siege.mp3", "checksum": "64E9A6E7" },
      {
        "path": "input/Damjan Mravunac - Welcome To Heaven.ogg",
        "checksum": "FFC58E16"
      },
      {
        "path": "input/Die Antwoord - Gucci Coochie (feat. Dita Von Teese).mp3",
        "checksum": "3490B59A"
      },
      {
        "path": "input/Lindemann - Ich Weiß Es Nicht.mp3",
        "checksum": "40C9CD73"
      },
      { "path": "input/MASTER BOOT RECORD - Dune.mp3", "checksum": "5F144F41" },
      {
        "path": "input/MASTER BOOT RECORD - MYTH.NFO.mp3",
        "checksum": "D4FA9F57"
      },
      {
        "path": "input/MASTER BOOT RECORD - RAMDRIVE.SYS.mp3",
        "checksum": "C3149CF3"
      },
      {
        "path": "input/MASTER BOOT RECORD - SET MIDI=SYNTH1 MAPG MODE1.mp3",
        "checksum": "C756AE33"
      },
      {
        "path": "input/Nightwish - Elvenpath (Live).mp3",
        "checksum": "781AABD2"
      },
      { "path": "input/Nightwish - Nemo.mp3", "checksum": "820F92DA" },
      {
        "path": "input/Nightwish - While Your Lips Are Still Red.mp3",
        "checksum": "DCBC7996"
      },
      {
        "path": "input/Van Pletzen - Benaaihilism.m4a",
        "checksum": "166D0587"
      },
      { "path": "input/extra/cover.jpg" },
      { "path": "input/extra/description.txt" }
    ],
    "apply": [
      {
        "path": "output_dir/Amon Amarth/2013 - Deceiver of the Gods/105 - Under Siege.mp3",
        "checksum": "64E9A6E7"
      },
      {
        "path": "output_dir/The Talos Principle/2015 - The Talos Principle OST/01 - Damjan Mravunac - Welcome To Heaven.ogg",
        "checksum": "FFC58E16"
      },
      {
        "path": "output_dir/Die Antwoord/2016 - Mount Ninji and da Nice Time Kid/05 - Gucci Coochie (feat. Dita Von Teese).mp3",
        "checksum": "3490B59A"
      },
      {
        "path": "output_dir/Lindemann/2019 - F & M/02 - Ich Weiß Es Nicht.mp3",
        "checksum": "40C9CD73"
      },
      {
        "path": "output_dir/MASTER BOOT RECORD/WAREZ/Dune.mp3",
        "checksum": "5F144F41"
      },
      {
        "path": "output_dir/MASTER BOOT RECORD/2017.01 - C-COPY . A -V/04 - MYTH.NFO.mp3",
        "checksum": "D4FA9F57"
      },
      {
        "path": "output_dir/MASTER BOOT RECORD/2020.01 - FLOPPY DISK OVERDRIVE/07 - RAMDRIVE.SYS.mp3",
        "checksum": "C3149CF3"
      },
      {
        "path": "output_dir/MASTER BOOT RECORD/2016.03 - C-EDIT AUTOEXEC.BAT/05 - SET MIDI=SYNTH1 MAPG MODE1.mp3",
        "checksum": "C756AE33"
      },
      {
        "path": "output_dir/Nightwish/2019 - Decades Live in Buenos Aires/12 - Elvenpath (Live).mp3",
        "checksum": "781AABD2"
      },
      {
        "path": "output_dir/Nightwish/2004 - Once/03 - Nemo.mp3",
        "checksum": "820F92DA"
      },
      {
        "path": "output_dir/Nightwish/While Your Lips Are Still Red.mp3",
        "checksum": "DCBC7996"
      },
      {
        "path": "output_dir/Van Pletzen/Benaaihilism/101 - Van Pletzen - Benaaihilism.m4a",
        "checksum": "166D0587"
      },
      { "path": "config/trash/run_id/extra_cover.jpg_166003A3" },
      {
        "path": "config/trash/run_id/extra_description.txt_3958064D"
      }
    ]
  },
  "tests": {
    "initial": {
      "expectations": "initial-state"
    },
    "apply": {
      "command": "--simple --yes rename -t frontmatter_prefix -- output_dir/",
      "expectations": "apply",
      "previous-expectations": "initial-state"
    },
    "undo": {
      "command": "--simple --yes undo",
      "expectations": "initial-state",
      "previous-expectations": "apply"
    },
    "redo": {
      "command": "--simple --yes redo",
      "expectations": "apply",
      "previous-expectations": "initial-state"
    }
  }
}
```

- [ ] **Step 3: Create the missing-required-argument failure fixture case**

Create `tests/fixtures/cli/cases/frontmatter_prefix_missing_required.case.json`:

```json
{
  "description": "Rename with frontmatter_prefix.tfmt fails before touching files when the required 'prefix' argument is omitted",
  "expectations": {
    "initial-state": [
      { "path": "input/Amon Amarth - Under Siege.mp3", "checksum": "64E9A6E7" },
      {
        "path": "input/Damjan Mravunac - Welcome To Heaven.ogg",
        "checksum": "FFC58E16"
      },
      {
        "path": "input/Die Antwoord - Gucci Coochie (feat. Dita Von Teese).mp3",
        "checksum": "3490B59A"
      },
      {
        "path": "input/Lindemann - Ich Weiß Es Nicht.mp3",
        "checksum": "40C9CD73"
      },
      { "path": "input/MASTER BOOT RECORD - Dune.mp3", "checksum": "5F144F41" },
      {
        "path": "input/MASTER BOOT RECORD - MYTH.NFO.mp3",
        "checksum": "D4FA9F57"
      },
      {
        "path": "input/MASTER BOOT RECORD - RAMDRIVE.SYS.mp3",
        "checksum": "C3149CF3"
      },
      {
        "path": "input/MASTER BOOT RECORD - SET MIDI=SYNTH1 MAPG MODE1.mp3",
        "checksum": "C756AE33"
      },
      {
        "path": "input/Nightwish - Elvenpath (Live).mp3",
        "checksum": "781AABD2"
      },
      { "path": "input/Nightwish - Nemo.mp3", "checksum": "820F92DA" },
      {
        "path": "input/Nightwish - While Your Lips Are Still Red.mp3",
        "checksum": "DCBC7996"
      },
      {
        "path": "input/Van Pletzen - Benaaihilism.m4a",
        "checksum": "166D0587"
      },
      { "path": "input/extra/cover.jpg" },
      { "path": "input/extra/description.txt" }
    ]
  },
  "tests": {
    "initial": {
      "expectations": "initial-state"
    },
    "missing-required-argument": {
      "command": "--simple --yes rename -t frontmatter_prefix",
      "expected-exit-code": 1,
      "expectations": "initial-state"
    }
  }
}
```

- [ ] **Step 4: Run the CLI integration suite**

Run: `cargo test -p tfmttools --test integration -- frontmatter_prefix`
Expected: PASS — both new cases pass (`frontmatter_prefix` and `frontmatter_prefix_missing_required`)

- [ ] **Step 5: Run the full CLI integration suite to check for regressions**

Run: `cargo test -p tfmttools --test integration`
Expected: PASS — all existing cases (`typical_input`, `simple_input`, etc.) still pass unchanged

- [ ] **Step 6: Commit**

```bash
git add tests/fixtures/cli/template/frontmatter_prefix.tfmt tests/fixtures/cli/cases/frontmatter_prefix.case.json tests/fixtures/cli/cases/frontmatter_prefix_missing_required.case.json
git commit -m "Add CLI fixtures for frontmatter-declared scripts"
```

---

### Task 12: Full workspace verification

**Files:** none (verification only)

- [ ] **Step 1: Run the full test suite**

Run: `cargo test --workspace`
Expected: PASS, no failures

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, no warnings (fix any `clippy::pedantic` issues surfaced in the new code before proceeding)

- [ ] **Step 3: Confirm formatting**

Run: `cargo fmt --check`
Expected: PASS (run `cargo fmt` and re-commit if it reports diffs)

- [ ] **Step 4: Manually exercise the deprecation warnings**

Run: `RUST_LOG=warn cargo run -p tfmttools --bin tfmt -- list-templates -t tests/fixtures/cli/template/`
Expected: stderr includes a warning for `typical_input` (or `case_only_rename`/`staged_swap`, whichever still use `args[N]` without frontmatter) matching "uses positional `args[N]` without frontmatter", confirming the deprecation path fires end-to-end. No warning or error for `frontmatter_prefix` or `simple_input`.

This step is manual verification, not an automated test, per the decision in Task 6 to avoid adding a tracing-capture test dependency for two log lines.
