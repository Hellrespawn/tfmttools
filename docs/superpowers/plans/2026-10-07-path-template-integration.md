# Path Template Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Replace MiniJinja throughout tfmt with the standalone path template language, preserving rename and history behavior.

**Architecture:** Core owns an audio metadata adapter and an owned compiled Template with separate binding. Filesystem helpers discover and compile scripts; CLI lists declarations and binds once before planning. Rendering produces native path components, then core appends the source extension and resolves against the working directory.

**Tech Stack:** Rust 2024, path-template, lofty, camino, existing fixture harness.

**Spec:** docs/superpowers/specs/2026-10-07-path-template-language-design.md

**Execution:** Native, preserving the user's selection. One independent whole-branch review after implementation.

## Global Constraints

- MSRV Rust 1.89.0; keep the language crate independent of tfmt and lofty.
- Manual migration; every discovered script uses the new syntax. Retain discovery extensions tfmt, jinja, j2; document tfmt.
- Missing or empty values are absent; integer zero is present. No truthy/falsy or equality features.
- Arguments are required without default. Validate all supplied arguments and defaults against FORBIDDEN_CHARACTERS; preserve accepted text without trimming or replacement.
- Tags retain aliases, date fallback, number/total parsing, forbidden-character replacement, and whitespace warnings.
- Unknown tags fail before rendering, including inside skipped guards. Missing recognized tags are allowed.
- Source extension retention and history serialization stay intact. Old stored inline scripts require explicit replacement for a new rename; undo/redo use stored actions.
- Diagnostics include script, line/column, and migration guidance for legacy syntax; render errors identify the input audio file.
- Explicit first top-level slash alone selects root. Padding width remains bounded at 1024.

## Review Focus

1. Unknown aliases inside skipped guards must fail during compilation, before filesystem changes.
2. Invalid defaults and unused arguments must fail even when output does not reference them.
3. Numeric zero, malformed number/total tags, and integers outside i64 must avoid panics or silent truncation.
4. Empty initial directories must never become rooted; Windows-style path arguments preserve component boundaries.
5. Historical Jinja scripts must not block undo/redo, and reuse for rename must give actionable migration guidance.

## File Responsibilities

- `crates/core/src/templates/context.rs`: lazy audio value resolution, sanitization, warnings.
- `crates/core/src/templates/template.rs`: owned script/source identity, schema checking, diagnostics, binding, render adapter.
- `crates/core/src/templates/mod.rs`, `error.rs`, `audiofile.rs`: exports, errors, native destination construction.
- `crates/fs/src/template.rs`: owned compiled scripts and discovery; remove environment and frontmatter side table.
- `crates/tfmt/src/commands/list_templates.rs`, `commands/rename/{template_resolution,discovery,preview}.rs`: listing, binding, history reuse, warnings.
- `crates/test-harness/src/data.rs`, `crates/test-cli/src/{case,runner}.rs`: optional initial fixture source remapping for the swap scenario.
- `examples/`, `tests/fixtures/cli/`, README, CHANGELOG, CLI help: manual syntax migration.
- Workspace/core/fs manifests and Cargo.lock: remove unused legacy dependencies.

### Task 1: Audio metadata adapter and compiled template

**Files:** Modify core template modules and error.rs; remove frontmatter.rs/source.rs after consumers move.

**Interfaces:** `Template::compile(lookup_name: &str, source: String) -> TFMTResult<Template>`; metadata methods `name() -> &str`, `description() -> Option<&str>`, `declared_args() -> &[path_template::ArgSpec]`; `Template::bind(&self, arguments: &[String]) -> TFMTResult<BoundTemplate>`; `BoundTemplate::render(&self, audio_file: &AudioFile) -> TFMTResult<(path_template::RenderedPath, Vec<Warning>)>`. Re-export ArgSpec/ArgKind. Template retains source for diagnostics, compiled Script, and lookup name. BoundTemplate owns the bound script and diagnostic context.

- [x] Write core tests constructing synthetic lofty tags: alias lookup, date fallback precedence, current/total parsing, whitespace and forbidden-character replacement, zero presence, and missing values. Assert oversized numeric text is preserved or treated as absent for invalid number fields without narrowing overflow. Compile an unknown tag inside a skipped guard and assert script name and location.
- [x] Run targeted core tests and observe the new behavior fail.
- [x] Replace MiniJinja Object with resolver returning optional Scalar. Preserve existing usize coercion semantics where representable; use checked conversions and text fallback for scalar values outside i64. Compile with a policy derived from FORBIDDEN_CHARACTERS and check every referenced alias. Translate compile/bind/render diagnostics to named line/column errors; include legacy migration hints for Jinja/frontmatter input. Bind independently of metadata listing.
- [x] Run core tests; commit the adapter and compiler change. Keep transitional code only where needed for later consumers to compile.

### Task 2: Owned filesystem loader and CLI consumers

**Files:** Modify fs/template.rs, core/audiofile.rs, CLI listing and rename modules.

**Interfaces:** Lifetime-free `TemplateLoader`; `read_directory`, `read_filename`, `read_script` return `FsResult<TemplateLoader>`; `get_template(&self, name: &str) -> Option<&Template>`; `get_all_templates(&self) -> Vec<&Template>`. Discovery binds selected Template once, then passes `&BoundTemplate` to audio target construction. `AudioFile::construct_target_path(&self, template: &BoundTemplate, relative_path: &Utf8Directory) -> TFMTResult<(Utf8File, Vec<Warning>)>`.

- [x] Add loader tests for metadata without required arguments, display-name fallback, valid syntax under each retained extension, invalid defaults, invalid unused/excess supplied arguments, and legacy syntax diagnostics. Add listing tests for required/default descriptions. Add native destination tests for explicit roots, missing initial directory errors, backslash-separated prefix args, and extension retention.
- [x] Run targeted core/fs/CLI tests and observe failures.
- [x] Store owned Templates in the loader, preserving discovery order and existing lookup behavior. Remove environment/frontmatter setup, update listing to public ArgSpec fields and explicit kind labels, and bind before scanning actions. Convert RenderedPath to Utf8PathBuf and append the original extension to the filename without replacing dots already present. Remove obsolete template deprecation warning variants and presentation code; retain tag whitespace warnings. Preserve history serialization; legacy stored inline script reuse reports migration guidance while explicit new scripts override history normally.
- [x] Run targeted suites; commit the working consumers.

### Task 3: Preserve staging and case-only integration coverage

**Files:** Modify test-harness/data.rs, test-cli/case.rs and runner.rs, swap/case-only case JSON and templates. Read applicable crate AGENTS before edits.

**Interfaces:** Optional fixture-only `initial-sources` map in TestCaseData, mapping input destination filenames to existing audio fixture filenames; absent map preserves setup. Expose `initial_sources() -> &IndexMap<String, String>`. Apply copies from immutable fixture audio directory after normal population and before initial verification; never source a remapped copy from an already modified input file.

- [x] Add a setup test mapping two fixture files to one another; assert contents are swapped and default setup stays compatible.
- [x] Run setup test and observe failure.
- [x] Implement initial source remapping. In staged_swap initial expectations exchange checksums of the two filenames; use `path: ("input" / {$artist} " - " {$title})` so one rename restores the files to their natural titles through a staged swap. Keep expected apply/undo/redo states consistent. Case-only template uses `path: ("input" / "nightwish - nemo")` for its selected input. No language comparisons added.
- [x] Run both integration scenarios, verify staging, final-target cleanup, undo, redo, and checksums; commit.

### Task 4: Migrate scripts, examples, and history regressions

**Files:** examples/*.tfmt, remaining fixture templates/cases, inline Rust test scripts, relevant fixture docs; add focused CLI history tests in existing test structure.

- [x] Add regressions for old stored inline Jinja reuse returning a migration hint, explicit replacement succeeding, and undo/redo succeeding without parsing the legacy script. Exercise missing and empty metadata, zero, and argument validation before any rename occurs.
- [x] Run regressions and observe failures where behavior is missing.
- [x] Manually migrate every script. Replace examples/stef.tfmt with the approved stef-next layout; retain the separate example. Migrate frontmatter-prefix fixture with required prefix declaration and update its metadata labels. Preserve all scenario intentions; update expected output only for deliberate zero/missing-artist differences. Fix any missing history diagnostic behavior uncovered by tests.
- [x] Run core/CLI/integration suites; compare Stef expected paths with standalone fixture tests; commit.

### Task 5: Remove legacy dependencies and document the replacement

**Files:** Cargo.toml, Cargo.lock, core/fs manifests, unused legacy modules/errors, README.md, CHANGELOG.md, cli/args_definition.rs, fixture documentation.

- [x] Verify no live MiniJinja/frontmatter consumers remain using rg. Remove MiniJinja dependencies and obsolete TOML parsing; retain TOML/regex only if other live consumers need them. Update workspace package description.
- [x] Replace README syntax guide with first-class declarations, quoted strings, $tags/bare args, guards/fallbacks/formatters, structural slash, and manual migration examples. Explain missing/empty versus zero, preserved argument whitespace, rejected argument characters, and missing-artist filename-prefix difference. Update --script help to show `path: ({$artist} / {$title})`; document old history reuse and supported discovery extensions. Add breaking-change changelog entry.
- [x] Run `cargo test --workspace --offline`, `cargo +nightly clippy --workspace --all-targets --offline`, `cargo xtask lint`, Rust 1.89 crate/workspace check where supported, and `git diff --check`. Record actual results and environment limitations, including Windows execution availability.
- [x] Commit documentation and dependency cleanup. Request one fresh independent whole-branch review with the five Review Focus items. Address findings in one regression-tested fix pass, rerun the full suite, then present integration choices.

## Plan Self-Review

The adapter, loader/CLI, fixture migration, historical reuse, diagnostics, docs,
and dependency removal cover the integration sections of the approved spec.
Task interfaces separate compiled metadata from bound rendering; no lifetime
coupling or environment remains. Every review focus input has an owning test
step. Source remapping is limited to test setup and preserves binary assets.

## Completion and final review

All five tasks are implemented. Workspace tests, nightly lint/format, and
Rust 1.89 workspace checks pass. Existing item_keys redundant-else and nix
future-incompatibility warnings remain; Windows execution was unavailable.

One independent whole-branch review found a cleanup deletion with parent
components in the input path, dot filename component loss/panics, missing
separate total fields, and empty date sources blocking fallback. Each has
regression tests observed failing before its fix and passing afterward;
the full workspace suite passes after the single final fix pass. The README
quote escape typo was corrected as part of the required syntax documentation.

Execution adjustments: a temporary CompiledTemplate kept old consumers
compilable during Task 1 and was removed in Task 2; all shared fixture scripts
migrated in Task 3 because directory loading compiles every discovered file.
Case-only coverage uses a dedicated copied input directory. Relative rename
inputs are resolved against cwd; cleanup canonicalizes both protected and
scanned paths, skips cleanup on protected-path resolution failures, and
preserves action history. This can leave extra files for subsequent cleanup.

Existing duplicate-stem lookup and all-discovered-script compilation remain.
Drive/UNC grammar remains excluded, native Windows checks are unverified,
and extreme parser nesting is deferred to independent library hardening.
