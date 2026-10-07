# Path Template Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement the agreed language as an independently usable workspace
crate and evaluate it with the new Stef script before CLI integration.

**Architecture:** `crates/path-template/` compiles owned scripts, binds
validated arguments, and renders structural paths through a caller supplied
metadata resolver. tfmt owns its audio schema, character policy, tag
sanitization, warnings, filesystem loading, and rename operations. The
first milestone leaves the current CLI using MiniJinja; replacing that
adapter requires a subsequent integration plan.

**Tech Stack:** Rust 2024, MSRV 1.89.0, standard library, existing workspace
`regex` and `thiserror` dependencies, Cargo unit/integration/doc tests.

**Spec:** [Path template language design](../specs/2026-10-07-path-template-language-design.md).

## Global Constraints

- The package is provisionally `path-template`, at `crates/path-template/`.
- No dependencies on tfmt crates, `lofty`, warning/history types, or filesystem discovery.
- Compiled scripts own their data; no environment or source lifetime coupling.
- Definitions use `:`, guards `?`, fallback `??`, and formatting `|`.
- Bare references resolve to declared arguments; `$` references use a caller metadata schema.
- Whitespace outside quoted strings contributes nothing to the path.
- Zero is present. Missing and empty values are absent. No truthy/falsy functions.
- Arguments without defaults are required. There is no `required` option.
- Argument component text is preserved; forbidden characters cause errors.
- Path arguments split on `/` and `\`; these are separators, not component text.
- Defaults are validated at compile time, including defaults overridden later.
- Tags are supplied as already prepared values; sanitization and warnings belong to the caller.
- Source extensions, reserved names, collisions, and full rename validation belong to tfmt.
- Preserve the current application behavior during this standalone milestone.
- Manual migration and removal of MiniJinja occur during the following integration milestone.

## Review Focus

- An argument default containing forbidden text must fail even if the caller supplies an override: Task 2.
- Optional content must not look up skipped tags or create stray separators: Task 3.
- A resolver failure must retain its original error and the failing reference's span: Task 3.
- A path argument inserted after unfinished component text must fail without silently changing the layout: Task 5.
- The crate and its tests must be usable without files or dependencies from sibling tfmt crates: Task 6.

## File structure and public interfaces

Create these files under `crates/path-template/`:

- `Cargo.toml`, `README.md`: independent description, usage, and packaging.
- `src/lib.rs`: public exports and crate documentation.
- `src/diagnostic.rs`: source spans and diagnostics, including resolver errors.
- `src/lexer.rs`: tokens, quoted strings, comments, and byte spans.
- `src/ast.rs`: private owned expression representation.
- `src/parser.rs`: document, argument declaration, and path expression grammar.
- `src/script.rs`: compiled/bound script ownership, metadata, and reference inspection.
- `src/args.rs`: declarations, character policy, defaults, and binding.
- `src/value.rs`: caller supplied scalar values.
- `src/format.rs`: `year` and `pad(width)` implementations.
- `src/path.rs`: rendered components and native path conversion.
- `src/render.rs`: ordered evaluation, fallback, guards, and path builder.
- `tests/syntax.rs`, `tests/arguments.rs`, `tests/rendering.rs`,
  `tests/stef.rs`, `tests/fixtures/stef.tfmt`: public API behavior tests.

Modify the workspace root `Cargo.toml` to add the member. Use workspace
edition, MSRV, license, version, repository metadata, and lint settings;
set a language specific package description. Runtime dependencies are
`regex` and `thiserror`; neither production nor development dependencies
may point to another workspace crate. Generic tests use synthetic values.

The public API must provide these signatures and types:

```rust
pub struct Span { pub start: usize, pub end: usize }
pub struct Diagnostic { pub message: String, pub span: Span }
pub enum Scalar { Text(String), Integer(i64) }
pub struct TagReference { pub name: String, pub span: Span }
pub struct Metadata { pub name: Option<String>, pub description: Option<String> }
pub enum ArgKind { String, Int, Path }
pub struct ArgSpec {
    pub name: String,
    pub kind: ArgKind,
    pub default: Option<String>,
    pub description: Option<String>,
    pub span: Span,
}
pub enum RenderError<E> {
    Template(Diagnostic),
    Resolver { name: String, span: Span, source: E },
}

impl ArgumentPolicy {
    pub fn new(forbidden: &[char]) -> Self;
}
impl Script {
    pub fn compile(source: &str, policy: ArgumentPolicy) -> Result<Self, Diagnostic>;
    pub fn metadata(&self) -> &Metadata;
    pub fn arguments(&self) -> &[ArgSpec];
    pub fn tag_references(&self) -> &[TagReference];
    pub fn bind(&self, supplied: &[String]) -> Result<BoundScript, Diagnostic>;
}
impl BoundScript {
    pub fn render<E>(
        &self,
        resolve: impl FnMut(&str) -> Result<Option<Scalar>, E>,
    ) -> Result<RenderedPath, RenderError<E>>;
}
impl RenderedPath {
    pub fn is_rooted(&self) -> bool;
    pub fn components(&self) -> &[String];
    pub fn to_path_buf(&self) -> std::path::PathBuf;
}
```

Spans use UTF-8 byte offsets into the original source. Diagnostic display
and a `Diagnostic::line_column(&self, source: &str) -> (usize, usize)` helper
provide one based line and character column positions. The caller adds a
filename or script label; the library does not read files. `RenderError<E>`
keeps resolver errors typed, with standard error/display implementations
when the corresponding trait bounds hold. Metadata can be inspected before
binding required arguments.

Tag references include every occurrence, including skipped guards, so a
caller can perform schema validation before rendering and report its span.
Normalize reference lookup names case insensitively. Bare identifiers use
letters/underscores followed by letters, underscores, or digits; tag names
also permit hyphens for existing alias forms. No audio tag names are built
into this crate. Script binding produces an owned bound script, with no
source or environment lifetime parameters.

## Task 1: Compile documents with owned data and diagnostics

**Files:** Create the manifest, library exports, lexer, AST, parser,
diagnostics, script types, argument/value/path type definitions, and
`tests/syntax.rs`. Modify workspace `Cargo.toml` and resulting `Cargo.lock`.

**Interfaces:** Produce `Script::compile`, metadata/argument/tag reference
inspection, `Span`, `Diagnostic`, `ArgSpec`, and the type declarations above.
Argument policy validation is completed in Task 2; rendering in Tasks 3–5.

- [x] Write failing lexer/parser tests with these names and assertions:
  `definitions_and_forward_argument_references` compiles
  `path: ({prefix} {$title}) arg prefix: string(default: "")` and reports
  one argument and one `$title` reference;
  `quoted_whitespace_and_comments` preserves `" - # [{}] "` while ignoring
  whitespace and `#` comments outside strings;
  `nested_guards_and_negative_guards` accepts
  `path: ([$album? [$date? {$date | year}]] [!$album? "Singles" /] {$title})`;
  `unknown_argument_and_option` rejects `{undeclared}` and `required: false`;
  `duplicate_definitions` rejects duplicate name/path/argument/options,
  including argument case variants;
  `bad_formatter_syntax` rejects unknown formatters, `pad()`, `pad(-1)`,
  and `year(2)`;
  `diagnostic_unicode_positions` reports line 2 and the correct character
  column after a quoted Unicode string, with byte offsets matching source;
  `compiled_script_outlives_source` compiles from a temporary String and
  inspects metadata after dropping it.
- [x] Run `cargo test -p path-template --test syntax` and establish failures.
- [x] Implement the grammar and source spans with a lexer and recursive
  descent parser. Recognize `??` before `?`. Require one path definition,
  validate bare names after all declarations are collected, and preserve
  every tag reference. Quoted strings accept `\"` and `\\`; reject unknown
  escapes and separators in path text literals. Definitions can be adjacent
  without newline significance; parse each by its grammar and delimiter.
- [x] Run the syntax suite and `cargo check -p path-template` successfully.
- [x] Commit the compiling language crate and grammar tests.

## Task 2: Bind arguments and enforce caller character policy

**Files:** `src/args.rs`, `src/script.rs`, `src/parser.rs`, `src/diagnostic.rs`,
`tests/arguments.rs`.

**Interfaces:** Consume parsed `ArgSpec` declarations. Produce
`ArgumentPolicy::new`, `Script::bind`, and owned `BoundScript` values.
Compilation must validate every declared default against its type/policy.

- [x] Write failing tests:
  `no_default_requires_argument` rejects omitted `arg edition: int`;
  `empty_default_makes_prefix_optional` binds omitted
  `arg prefix: path(default: "")` successfully;
  `defaults_do_not_replace_supplied_empty_values` accepts an explicitly
  supplied empty string for a string argument with a nonempty default;
  `integer_validation` accepts `"0"` and `"-2"`, rejects `""`, `"no"`, and
  values outside i64 range;
  `declaration_order_and_excess_arguments` binds in declaration order and
  rejects extras, including when no arguments are declared;
  `forbidden_string_and_path_text` supplies the policy
  `<">:|?*~/\` as individual characters and rejects each in string values,
  while path values accept `/` and `\` as separators but reject the others
  inside components;
  `invalid_defaults_even_when_overridden` fails compilation of forbidden
  string/path defaults and empty integer defaults;
  `invalid_unused_argument` rejects a forbidden supplied value even if the
  path rule never references it.
- [x] Run `cargo test -p path-template --test arguments` and confirm failures.
- [x] Implement binding and compile-time defaults validation. Preserve text
  without trimming/replacement. Split path arguments on both separators,
  discard empty segments, and validate remaining text. Character errors
  identify the argument, offending character, and path component as needed.
- [x] Run syntax and argument suites successfully.
- [x] Commit argument binding and policy enforcement.

## Task 3: Evaluate scalar values, fallbacks, and guards

**Files:** `src/render.rs`, `src/script.rs`, `src/value.rs`,
`src/diagnostic.rs`, `src/path.rs`, `tests/rendering.rs`.

**Interfaces:** Consume bound arguments and owned expressions. Produce
`BoundScript::render`, typed resolver errors, and path component accessors.
Formatter evaluation is completed in Task 4; path argument insertion in Task 5.

- [x] Write failing tests:
  `quoted_literals_and_adjacent_values` renders `path: ("A" {$title} " B")`
  with title `"T"` to component `"AT B"`;
  `zero_is_present` renders `[$track? {$track}]` with Integer(0) to `"0"`;
  `missing_and_empty_are_absent` uses None/Text("") to select a negative guard;
  `fallback_is_lazy` renders `{$albumartist ?? $artist ?? "Unknown"}`,
  asserts the first present value wins, and verifies later lookups do not occur;
  `skipped_guard_does_not_lookup_contents` records resolver calls for
  `[$album? {$date | year} /] {$title}` with missing album and asserts only
  album/title are looked up and the result has one filename component;
  `resolver_error_has_reference_span` returns a custom error from `$title`
  and asserts its value and reference byte span survive in RenderError;
  `all_references_are_visible_before_rendering` inspects references inside
  skipped guards, including duplicates;
  `whitespace_only_argument_is_preserved` renders a string argument containing
  a space inside `"A" {suffix} "B"` to `"A B"`.
- [x] Run `cargo test -p path-template --test rendering` and confirm failures.
- [x] Implement ordered/lazy evaluation. Absence emits no text; only empty
  text/paths or missing values are absent. Resolve tag names on demand using
  the caller closure. Keep original typed errors and source spans.
- [x] Run all existing crate tests successfully.
- [x] Commit scalar rendering, guards, and fallback behavior.

## Task 4: Implement year extraction and zero padding

**Files:** `src/format.rs`, `src/render.rs`, `tests/rendering.rs`.

**Interfaces:** Consume parser-validated formatter expressions. Implement
private `year(text: &str) -> Result<String, String>` and
`pad(text: &str, width: usize) -> String`; rendering wraps failures in a
Diagnostic at the formatter span.

- [x] Write failing tests: ISO `2024-03-10`, `10-03-2024`, and `2024` all
  render year `"2024"`; a present `"unknown"` errors; a missing date and
  a skipped date guard do not error. Padding Integer(3) with width 2 yields
  `"03"`, Text("123") stays `"123"`, width 0 preserves the value, and
  Integer(0) with width 2 yields `"00"`. A formatter following fallback
  operates on the selected value. `year | pad(6)` on `2024` yields `"002024"`.
- [x] Run the formatter rendering tests and confirm failures.
- [x] Implement the existing three date matching patterns with regex,
  preserving matching precedence. Pad displayed text without truncation.
  Absent formatter input emits nothing. Reject formatting path arguments
  during compilation, including path alternatives in a fallback expression.
- [x] Run all crate tests successfully and commit formatter support.

## Task 5: Construct structural paths and insert directory arguments

**Files:** `src/path.rs`, `src/render.rs`, `tests/rendering.rs`.

**Interfaces:** Produce `RenderedPath::is_rooted`, `components`, and
`to_path_buf`; complete path argument rendering and separator validation.

- [x] Write failing tests:
  `native_separators` renders `"Artist" / "Song"` to two components and
  compares conversion with `PathBuf::from("Artist").join("Song")`;
  `prefix_components` binds `Music/Artists` and `Music\\Artists` to
  `{prefix} "Artist" / "Song"` and gets the same four components;
  `empty_prefix_has_no_boundary` omits a prefix with `default: ""` and
  gets only Artist/Song;
  `argument_after_component_text_is_error` rejects `"A" {prefix} "B"`
  when prefix is nonempty;
  `explicit_separator_after_path_argument_is_error` rejects an extra `/`
  after a nonempty path argument;
  `invalid_boundaries` rejects repeated separators, a trailing separator,
  an empty path, and an empty final filename;
  `root_separator` checks a leading bare `/` sets the root indicator and
  native conversion joins the platform separator with its components;
  `prepared_tag_cannot_inject_boundary` rejects unsanitized resolver text
  containing `/` or `\`, rather than allowing it to create directories.
- [x] Run the structural path tests and confirm failures.
- [x] Implement a component builder without parsing a joined string.
  A path argument inserts completed directory components at a component
  boundary; scalar values append text. Reject separator characters in scalar
  component text, but perform no tfmt character replacement. Apply the
  spec's root and empty component rules after conditional evaluation.
- [x] Run all crate tests successfully on the available host. Include
  platform-independent expectations and conditional native-root tests;
  record whether Windows execution was available.
- [x] Commit structural path rendering.

## Task 6: Evaluate the Stef script and verify independent use

**Files:** `tests/stef.rs`, `tests/fixtures/stef.tfmt`, `README.md`,
`src/lib.rs`, and the crate manifest if documentation packaging needs it.
The fixture is a local copy of `examples/stef-next.tfmt` so crate tests do
not depend on files outside the package.

**Interfaces:** Use only the public API, standard library, local fixtures,
and synthetic metadata. No audio files, workspace test harness, or tfmt dependencies.

- [x] Write end-to-end tests with the exact example source. Compile using
  the tfmt character table supplied as a test policy, bind prefix `Music`,
  and render synthetic tags to components:
  `["Music", "Example Artist", "2024.02 - Example Album",
  "103 - Example Artist - Example Song"]`.
  Add cases without album, without date, without album sort, without album
  artist, without artist, without disc number, and without track number;
  assert exact components and delimiter absence.
  Assert zero track/album sort values render as present. Assert unchanged
  output when outside-string whitespace and comments are added.
- [x] Run `cargo test -p path-template --test stef` and establish any failures.
- [x] Complete any missing behavior through the owning module and rerun
  its targeted tests. Keep the language grammar unchanged unless the
  results expose a concrete requirement requiring a design revision.
- [x] Document compile → inspect references → bind → resolve → render,
  with a doctest that compiles and renders a small script. Explain caller
  character policy, prepared metadata, schema validation, typed resolver
  errors, standalone installation assumptions, and root/path limitations.
- [x] Verify fixture consistency with
  `cmp examples/stef-next.tfmt crates/path-template/tests/fixtures/stef.tfmt`.
  Run `cargo test -p path-template` including doc tests, and inspect
  `cargo package --list --allow-dirty --offline -p path-template` for local
  sources/fixtures/README. Its tests must not reference root example paths.
- [x] Run `cargo test --workspace`,
  `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.
  Fix actionable findings introduced by this change and report any
  pre-existing or environment failures. Check MSRV with
  `cargo +1.89.0 check -p path-template` when cached dependencies permit it.
- [x] Commit the evaluated crate and documentation. Report synthetic
  Stef output, verification results, and the remaining CLI integration
  milestone. Do not claim the application has migrated to the new language.

## Final review and verification

Independent whole-branch review found one Important issue: empty initial
output could accidentally select the platform root. Root selection now
requires `/` as the first top-level expression. Regression tests failed
before the fix and pass afterward, covering missing tags, empty literals,
empty path arguments, and separators inside guards. Explicit roots remain
supported.

The Minor padding allocation concern was also addressed: compilation rejects
widths above 1024. Boundary tests failed before the fix and pass afterward.
Both findings were handled in one final fix pass without a second review.

Workspace tests, nightly lint/format, Rust 1.89 checking, and independent
Cargo package verification pass. Existing core redundant-else and nix
future-incompatibility warnings remain. Windows execution was unavailable.
CLI migration is the next milestone.
