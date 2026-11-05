# Parenthesized Template Expressions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace picotmpl's delimited insertion and guards with composable
expressions, unary negation, conditional alternatives, and Elvis fallback.

**Architecture:** Keep the existing compile, bind, and render interfaces.
Parse the approved precedence into a recursive expression tree, validate
possible boolean and structural results before rendering, and evaluate
selected values before emitting them through the existing path builder.
Migrate workspace callers after the library's focused checks pass.

**Tech Stack:** Rust 1.91.0, edition 2024, existing Pest parser, Cargo tests,
and the existing fixture-backed CLI integration harness. No new dependency.

**Spec:** [Approved expression design](../specs/2026-11-05-parenthesized-template-syntax-design.md).

## Global Constraints

- "This replaces the old syntax." Reject `{...}`, `[...]`, and `??` outside
  strings; retain them only in negative tests or historical records.
- "Numeric zero remains present for both conditionals and Elvis."
- "Boolean results may be used in conditions but must never be emitted
  into a filename."
- "Parse expressions uniformly, then validate their possible result types
  in the AST rather than encoding that restriction in separate grammar
  productions."
- "Formatting any possible path-valued input is rejected."
- "A group may represent part of a path, including a trailing directory
  boundary; only the completed outer path requires a nonempty final
  filename."
- "Do not rely on runtime resolver values or constant folding to allow a
  potentially boolean output."
- "No compatibility dialect, automatic migration command" or history
  format migration. Preserve argument validation and filename safety.

## Review Focus

- Nested `?`, `:`, and `?:` without spaces must preserve branch association
  and never reinterpret an empty conditional branch as Elvis.
- Multibyte text before an error must still yield valid UTF-8 spans and
  character-based columns.
- A discarded path-valued operand must never append to the selected path;
  grouping a selected fragment must not bypass path-argument boundaries.
- Boolean results hidden in unreachable alternatives must fail compilation
  without requiring a metadata resolver or touching audio files.
- Stored template strings in old history must remain readable and undoable
  even though the replacement parser rejects them for new renames.

## File map

Library implementation:

- Modify `crates/picotmpl/src/grammar.pest`: expression binding and tokens.
- Modify `crates/picotmpl/src/parser.rs`: recursive AST construction,
  source spans, root lowering, reference collection, and syntax diagnostics.
- Modify `crates/picotmpl/src/ast.rs`: recursive operations and debug dump.
- Modify `crates/picotmpl/src/value.rs`: internal booleans, absence, and
  structural sequence results; public `Scalar` stays text/integer.
- Create `crates/picotmpl/src/validation.rs`: argument checks and expression
  result checks, replacing the flat validator in `template.rs`.
- Modify `crates/picotmpl/src/lib.rs`: register the private validation module.
- Modify `crates/picotmpl/src/template.rs`: call validation during compile;
  retain public interfaces and default coercion.
- Modify `crates/picotmpl/src/render.rs`: expression evaluation and selected
  value emission through `path::Builder`.
- Reuse `crates/picotmpl/src/path.rs`, `args.rs`, `format.rs`, and
  `diagnostic.rs`; change them only if required to preserve existing checks.

Library verification and docs:

- Modify `crates/picotmpl/tests/{syntax,rendering,arguments,stef,dump_ast}.rs`.
- Modify `crates/picotmpl/tests/fixtures/stef.tfmt`.
- Modify `crates/picotmpl/examples/dump_ast.rs` tests and
  `crates/picotmpl/README.md`, including its compiled Rust example.

Workspace migration:

- Modify `examples/stef.tfmt` and the five templates in
  `tests/fixtures/cli/template/`: `typical_input.tfmt`,
  `declared_prefix.tfmt`, `simple_input.tfmt`, `staged_swap.tfmt`, and
  `case_only_rename.tfmt`.
- Modify executable snippets in `crates/core/src/audiofile.rs` and the
  migration hint in `crates/core/src/templates/template.rs`.
- Modify snippets in `crates/tfmt/src/cli/args_definition.rs` and
  `crates/tfmt/src/commands/list_templates.rs`.
- Modify `crates/tfmt/tests/{input_scope,discovery,history_interruption,
  history_compatibility,confirmation,path_templates}.rs` where snippets
  are executed by the current parser. Preserve intentionally stored legacy
  strings and rejection cases.
- Update `README.md`, `CHANGELOG.md`, and any executable snippets found by
  the targeted search in Task 2. Inspect fixture case JSON only where it
  contains template source or asserts syntax-dependent diagnostics.
- Keep `xtask`'s dump command and the CLI harness unchanged unless their
  verified behavior requires a change. Do not read generated report assets.

## Task 1: Replace picotmpl's expression engine

This is one coupled library change: the parser, evaluator, validator, and
dump must agree on the AST. Do not add a transitional parser or preserve
the old expression model solely to split this task. The library can be
verified independently; workspace CLI snippets are migrated in Task 2.

**Interfaces:**

- Keep `Template::compile(source: &str) -> Result<Template, Diagnostic>`,
  `Template::bind(&self, supplied: &[String]) -> Result<BoundTemplate,
  Diagnostic>`, and `Template::dump_ast(&self) -> String`.
- Keep `BoundTemplate::render<E>(&self, resolve: impl FnMut(&str) ->
  Result<Option<Scalar>, E>) -> Result<RenderedPath, RenderError<E>>`.
- Keep `Template::tag_references(&self) -> &[TagReference]`, metadata,
  arguments, and diagnostic APIs.
- Add private `validation::validate(path: &[Expression], arguments:
  &[ArgSpec]) -> Result<(), Diagnostic>`. Its recursive analysis returns
  private `Capabilities { boolean: bool, structural: bool }` for possible
  results. Scalar or absent results need neither flag.
- Private AST and runtime variant names are local to this task. They must
  represent references, literals, sequences, unary negation, formatting,
  Elvis, conditional branches, root, and separators with source spans.

- [x] **Step 1: Add binding tests in `tests/rendering.rs`.**

  Reuse `bind`, `text`, and the resolver helpers. With `a = "7"`,
  `b = "3"`, `n = "2"`, and absent `missing`, assert these components:

  | Source inside `path: (...)` | Expected filename |
  | --- | --- |
  | `"." $n \| pad(2)` | `.02` |
  | `("." $n) \| pad(3)` | `0.2` |
  | `$a ?: $b \| pad(2)` | `7` |
  | `($a ?: $b) \| pad(2)` | `07` |
  | `"By " $missing ?: "Unknown"` | `By Unknown` |
  | `("By " $missing) ?: "Unknown"` | `By ` |
  | `$missing ? $n \| pad(2) " - "` | ` - ` |
  | `$missing ? ($n \| pad(2) " - ") "Song"` | `Song` |
  | `$missing ? "yes" : "no" "!"` | `no!` |
  | `$a ? $missing ? "both" : "only a" : "no a"` | `only a` |

  Name the tests `formatting_and_concatenation_binding`,
  `elvis_binding_and_presence`, and `conditional_branch_scope`.

- [x] **Step 2: Add type and token tests in `tests/syntax.rs`.**

  `booleans_are_rejected_only_in_invalid_contexts` must compile
  `path: ((!$album) ? "missing" : $title)` and
  `path: ((!$a ?: !$b) ? "absent" : "present")`.
  It must reject each of the following at compilation with an in-bounds
  span identifying the offending operation:

  ```text
  path: (!$album)
  path: ($artist ? !$album)
  path: ("Album: " !$album)
  path: ((!$album ?: "Unknown"))
  path: ((!$album | pad(2)) ? "missing")
  path: ("always" ?: !$album)
  path: ("always" ? "ok" : !$album)
  ```

  `conditional_tokens_and_association` must accept `$a?$b?:$c:$d` and
  `$a?$b?"both":"a":"none"` inside the path body, and reject
  `$a ? : $b`, `$a ?? $b`, `$a ?`, `$a ? $b :`, a dangling `:`, and `!`
  without an operand. Add old-delimiter rejection cases while keeping
  quoted `{}`, `[]`, `??`, and `?:` valid literal text.

  `unicode_diagnostic_columns` must use `path: ("é" !)` and assert the
  missing-operand diagnostic points at `)` or its boundary with one-based
  character column 13, not a byte-based column. Retain source-slice checks
  for resolver and formatter spans after multibyte text.

- [x] **Step 3: Add behavioral edge tests in `tests/rendering.rs` and
  `tests/arguments.rs`.**

  - `negation_and_elvis_keep_zero_present`: use missing, empty text, integer
    `0`, and text `"0"`; `!$n ? "absent" : "present"` yields `absent`
    for the first two and `present` for the last two. `$n ?: "fallback"`
    yields `fallback`, `fallback`, `0`, `0`. Test double negation as a
    condition and a boolean-false Elvis operand.
  - `selected_operands_are_evaluated_once`: log resolver calls for
    `$a ?: $b ?: "Unknown"` and assert `["a"]` when `a` is present.
    Have skipped branches return an error if resolved. Cover both arms of
    `? :`, and a selected invalid date remaining an error.
  - `grouped_paths_keep_boundaries`: `("Artist" /) "Song"` yields
    `["Artist", "Song"]`; `"Artist" (/) "Song"` does too.
    A grouped absent guard `( $missing ? "Unused" ) "Song"` yields only
    `Song`. A selected group `(("Artist" /) ?: ("Unused" /)) "Song"`
    must not emit the unselected directory.
  - `nested_paths_cannot_be_formatted`: reject formatting grouped `/`,
    fallback to a declared path argument, and conditional structural output
    even when statically unselected. Permit formatting a text result whose
    condition merely tests a path argument.
  - `grouping_does_not_bypass_path_insertion`: inserting a grouped path
    argument after `"partial"` fails; inserting it before `"Song"` at a
    boundary succeeds. Grouping does not finish a partial path prematurely.
  - Retain tests for repeated/trailing boundaries, empty final filenames,
    skipped leading expressions not selecting root, and native paths.

- [x] **Step 4: Run the new tests and verify failure.**

  Run `cargo test -p tfmttools-picotmpl --test rendering`,
  `cargo test -p tfmttools-picotmpl --test syntax`, and
  `cargo test -p tfmttools-picotmpl --test arguments`. The new syntax must
  fail under the existing parser; do not mistake an unrelated build error
  for the expected failure.

- [x] **Step 5: Implement the recursive parser and AST in the mapped files.**

  Implement the approved grammar: unary `!`, formatter chains, Elvis,
  right-nested conditionals with optional alternatives, and adjacency.
  Treat `?:` as a contiguous token and attach `:` to the nearest unmatched
  `?` in its group. Preserve every reference occurrence and byte span.
  Lower only the first direct slash of the outer path group to root.
  Retain existing declaration, default, literal, and formatter validation.
  Update the AST dump to show nested operations and branches without
  resolution; exact debug labels remain the implementer's choice.

- [x] **Step 6: Implement compile-time result validation.**

  Register and call `validation::validate` from `Template::compile`.
  Analyze every branch, even unreachable ones, and validate all arguments.
  Negation returns boolean capabilities; conditional and Elvis results
  union their output alternatives. A condition's capabilities do not leak
  into the conditional's result. Single-expression groups preserve types;
  multi-expression concatenation rejects boolean members. Formatting rejects
  boolean or structural capabilities. Reject boolean capabilities at the
  outer output. Keep argument default coercion and public `Scalar` intact.

- [x] **Step 7: Implement value evaluation and selected output emission.**

  Extend private values for boolean, absence, and ordered structural
  fragments. A sequence must preserve text, slash, and path-insertion events
  and their spans, rather than joining paths into strings. A text-only
  concatenation can be formatted; a one-value group preserves its type.
  Tests use booleans by truth value, other values by the spec's presence
  rules, including integer zero. Elvis resolves a selected operand once;
  conditionals evaluate only their selected branch. Negation reverses this
  test. Emit selected fragments through existing `path::Builder` methods,
  preserving path-insertion boundaries and final-filename validation.

- [x] **Step 8: Migrate the library's existing tests, fixture, and README.**

  Use the approved Stef example. Preserve assertions about complete,
  missing, empty, and zero metadata. Update dump snapshots and standalone
  example tests to the new tree. Replace combined fallback-and-format
  cases with explicit `($a ?: $b) | pad(2)` when preserving their old
  meaning. Keep intentionally invalid examples as rejection tests.
  Update the README's Rust doctest and language reference, including the
  binding table, boolean errors, optional alternatives, and Elvis laziness.

- [x] **Step 9: Verify and commit the library change.**

  Run `cargo test -p tfmttools-picotmpl`,
  `cargo test -p tfmttools-picotmpl --example dump_ast`, and
  `cargo +nightly clippy -p tfmttools-picotmpl --all-targets`.
  Expected: all pass without warnings. Run the nightly workspace formatter
  on edited Rust code, inspect the diff, and commit as
  `Replace picotmpl delimiters with template expressions`.
  Workspace CLI tests still contain old syntax until Task 2; this is not
  the final verification or a complete feature yet.

## Task 2: Migrate and verify every workspace caller

**Interfaces:** Consume the unchanged public picotmpl interfaces from
Task 1 through existing `compile_audio_template(source: &str) ->
TFMTResult<Template>` and CLI template loading. Produce migrated sources,
current help and docs, and passing workspace and fixture checks. No new
command, option, history schema, or public API.

- [ ] **Step 1: Migrate the file-map examples, fixtures, and Rust snippets.**

  Search using `rg -n -F 'path: (' crates examples tests/fixtures/cli
  README.md` and targeted searches for `{\$`, guarded brackets, and `??`.
  Classify each match before editing: current source, rejection test,
  retained history metadata, or historical prose. Migrate current sources
  and the legacy-Jinja error hint; do not alter stored history strings.
  Search fixture case JSON for embedded template source and expected
  diagnostics. Preserve fixture filenames, output paths, and rename history
  expectations. Do not regenerate or inspect bulky report assets.

- [ ] **Step 2: Add CLI boundary checks to `tests/path_templates.rs`.**

  Reuse its temp-directory CLI helpers and the existing audio fixture.
  `boolean_output_is_rejected_before_rename` must run an inline template
  with `path: (!$album)`, assert failure with a located boolean diagnostic,
  and assert the original file bytes and name are unchanged and no target
  or applied history action exists. Repeat for a boolean hidden in an
  unreachable branch. `conditional_and_elvis_inline_template` must dry-run
  a template using `!`, `? :`, and `?:` against a known fixture and assert
  the expected target filename and unchanged input. Pin the expected name
  from the fixture's existing metadata assertions rather than synthesizing
  a new audio asset.

- [ ] **Step 3: Verify history remains independent of current syntax.**

  Preserve `historical_rename`'s stored `{{ artist }}/{{ title }}` in
  `tests/path_templates.rs` and other retained history compatibility data.
  Run `cargo test -p tfmt --test path_templates` and
  `cargo test -p tfmt --test history_compatibility`. Expect current-source
  syntax and boolean errors to fail before rename, while undo/redo and
  history display still work for old stored strings without recompilation.
  Fix any actual integration regression at its existing boundary.

- [ ] **Step 4: Finish current documentation and release notes.**

  Update README syntax tables, CLI examples, and migration guidance;
  update CLI help examples and CHANGELOG. Document replacement of braces,
  brackets, and `??`, with examples of branch grouping and the surprising
  but intentional `("By " $a) ?: "Unknown"` behavior. Keep approved
  historical specs and the stored history format intact.

- [ ] **Step 5: Run final checks and inspect the complete diff.**

  Run `cargo xtask test-integration`, `cargo test --workspace`,
  `cargo test -p tfmttools-picotmpl --example dump_ast`,
  `cargo +nightly clippy --workspace --all-targets`,
  `cargo +nightly fmt --all -- --check`, and
  `cargo xtask dump-ast examples/stef.tfmt`.
  Expect zero failing tests, clean Clippy and formatting, and a dump showing
  the new expression nesting. Run `git diff --check` and targeted searches
  to ensure old syntax remains only where intentionally retained. Commit as
  `Migrate CLI templates to expression syntax`.

## Self-review and execution

Spec coverage: Task 1 owns all syntax, presence, boolean, structural-value,
span, reference-inspection, and dump behavior. Task 2 owns executable caller
migration, CLI preflight safety, history compatibility, and final workspace
verification. Every Review Focus item has a named test or preserved
integration check above. Public signatures are unchanged across tasks;
result capabilities and AST changes stay internal to Task 1.

Native execution is recommended: parser, evaluator, and validator share one
AST, and migration depends on that finished library. Implement sequentially,
then obtain one independent whole-change review. Subagent-driven execution
remains an option if the user prefers separate implementation and review
contexts per task. Review this plan and choose the execution method before
implementation.
