# Simplify parsing, CLI testing, and confirmation prompts

Status: proposed. This document specifies a migration to pest, snapbox,
and dialoguer; it does not authorize or implement the migration.

## Purpose and scope

Reduce custom infrastructure while preserving tfmt's path language,
audio-specific verification, and filesystem/history guarantees. Adopt:

- `pest` and `pest_derive` inside `tfmttools-picotmpl` for syntax parsing.
- `snapbox` inside the CLI test infrastructure for command assertions and
  optional output snapshots.
- `dialoguer` inside `tfmt` for interactive confirmation.

Keep Rust 1.91.0 and edition 2024. Select compatible releases during
implementation, declare dependencies centrally in `Cargo.toml`, and update
`Cargo.lock`. Enable only necessary features. Parser and prompt libraries
are production dependencies; snapbox must not enter the production CLI
dependency graph through normal dependencies.

This is a maintenance change, not a new template language, test runner,
terminal application, or history format. No ORM, filesystem transaction
replacement, trycmd migration, or report viewer redesign is included.

## Current implementation and design choices

The template lexer and recursive-descent parser occupy approximately 500
lines in `crates/picotmpl/src/lexer.rs` and `parser.rs`. Rendering, argument
binding, and path construction already have distinct responsibilities.
A pest grammar replaces syntax machinery while retaining those boundaries.
Using logos alone would replace less code; replacing the language with a
general template engine would change behavior and undo the earlier move
away from MiniJinja.

`crates/test-cli/src/runner.rs` discovers JSON cases, runs sequential
commands, verifies files and audio tags, and records structured outcomes.
Snapbox supplies reusable assertions within this harness. Replacing the
whole harness with trycmd would require recreating audio verification and
report integration, so that approach is excluded.

`crates/tfmt/src/ui/prompt.rs` implements a line-based yes/no loop. All
confirmation call sites use this wrapper. Keep it as the policy boundary
and delegate terminal interaction to dialoguer.

The previous UI used a local `tfmttools-tui` crate with ratatui and
crossterm. Commit `5cd9cce` (2023-12-28) replaced rename TUI confirmation
with printed previews and the current prompt; `8081a1c` (2023-12-29)
removed the TUI. Neither commit explains a library defect. The migration
must retain the conventional CLI flow rather than restore a full TUI.

## Pest architecture and compatibility

Add a grammar at `crates/picotmpl/src/grammar.pest`. Keep pest's generated
parser, rules, and pairs private. Convert pairs into the existing AST and
compiled representation in `parser.rs`; remove `lexer.rs` and its module
declaration once all syntax parsing uses pest. Public types and signatures
in `template.rs`, `args.rs`, `diagnostic.rs`, and `lib.rs` remain stable.

Separate grammar recognition from semantic checks. The grammar handles
document definitions, argument settings, strings, references, sequences,
guards, alternatives, and formatter syntax. Rust code retains duplicate
checks, name resolution, argument policy/default validation, formatter
validation, and the padding limit of 1024. Collect every tag reference,
including those in unselected branches. Keep binding and rendering in
their existing modules.

The current implementation and tests are the compatibility baseline.
The earlier path-language design provides context but contains historical
tag spellings; it must not override the current canonical-name contract.
In particular:

- Consume the complete document; reject trailing garbage and missing or
  duplicate path definitions.
- Preserve definition and argument name normalization, case-sensitive
  tag names, explicit aliases, and tag-name hyphens.
- Preserve Unicode whitespace and line comments outside strings. Strings
  retain literal whitespace and punctuation and allow only escaped quote
  and escaped backslash. Grammar whitespace insertion must not permit
  splitting identifiers or escape sequences.
- Preserve declaration order, optional argument settings and trailing
  commas, arbitrary definition order, defaults, and duplicate errors.
- Preserve guards, negative guards, nested sequences, left-to-right
  fallback, formatter chains, and structural separators/root handling.
- Preserve numeric zero presence, lazy resolver calls, tag sanitization,
  path argument insertion, extension handling, and invalid-component
  checks by leaving evaluation and path construction unchanged.

Map pest syntax failures into the existing `Diagnostic` with UTF-8 byte
offsets. Translate internal rule names into actionable language terms.
Semantic errors retain their specific messages and relevant source spans.
Syntax wording may improve, but script identity, line/character column,
offending construct, expected syntax, and the legacy Jinja/frontmatter
migration hint remain available. EOF errors may use an empty span at EOF.
Never expose generated `Rule` names as the user-facing explanation.

Before deleting the old parser, compare both implementations on the
existing corpus and additional boundary cases. Compare accepted/rejected
inputs and public results, not private AST layout. Any disagreement must
be resolved explicitly; this migration must not silently expand syntax.

## Snapbox integration

Keep `libtest-mimic`, JSON case discovery, sorting/filtering, sequential
steps, skip-after-failure behavior, and work-directory retention on failure.
Keep fixture expectations for checksums, audio values, and raw tag
encodings. Keep report serialization and viewer contracts intact.

Use snapbox's command facilities in `run_command` to replace the current
assert_cmd command wrapper where supported. Capture the real process
output and arguments once, then use snapbox's non-panicking assertion
interface for comparison. Convert mismatches into structured failures so
the harness still writes its JSON and HTML report. Do not let a snapshot
panic bypass outcome collection. Preserve explicit numeric exit codes,
including expected failures, rather than reducing status to success/fail.

Extend `TestData` with optional `stdout` and `stderr` fields containing
paths relative to the case JSON's directory. Absence means that stream is
not compared, preserving existing cases. An empty snapshot file means
that the stream must be empty. Resolve references independently of the
child process working directory. Document the fields and keep expected
text assets beside the cases under `tests/fixtures/cli/`.

Use snapbox normalization for platform line endings and approved dynamic
paths. Redact only concrete run-specific values such as the temporary
work/config roots. Do not broadly remove numbers, filenames, errors, or
path components that carry behavior under test. Keep raw captured output
in reports and apply normalization only to comparisons. Include useful
stream mismatch details in the existing command-failure reporting path;
if that model needs additive fields, update its consumers and viewer tests
together. Missing/unreadable snapshot files are reported as test failures.

Snapshot updates require an explicit local opt-in using the selected
snapbox release's update mechanism. Default runs and CI must compare
without overwriting expectations. Add representative help, validation
error, and rename-preview snapshots; avoid snapshotting every existing
audio fixture merely to increase coverage.

Directory snapshots are optional for text-only fixtures. They must not
replace audio checksums or metadata/encoding verification. Remove
assert_cmd only from crates where no remaining code uses it; retain
assert_fs where it still provides fixture setup utilities.

## Dialoguer integration and input policy

Retain `ConfirmationPrompt::new(...).prompt() -> Result<bool>` so rename,
undo/redo, and clear-history commands share one policy. Keep their force
and dry-run decisions at existing call sites. A bypass must not read stdin
or initialize an interactive prompt.

When stdin and stdout are terminals, use `dialoguer::Confirm` with a false
default, newline confirmation, a plain theme, and an explicit stdout
terminal. Keep prompt questions and previews unchanged. Avoid an extra
selection report after confirmation. Minor punctuation differences from
dialoguer are acceptable and must be documented; no colors, menus, or
screen-clearing interaction are introduced.

When either stream is not a terminal, retain a small line-based adapter.
Dialoguer terminal interaction must not unexpectedly open a controlling
terminal or consume piped input as keystrokes. This intentional fallback
limits the amount of prompt code that can be deleted:

- Trim and case-fold input; `y`/`yes` accepts, `n`/`no` declines.
- An empty line or EOF declines, matching the current false default.
- Invalid input prints the existing retry guidance and reads another line.
- Write and flush the question to stdout before reading input.
- Propagate I/O failures; never treat an input error as consent.

Terminal mode must accept the same affirmative/negative spellings and
empty-line default. Preserve trimming/case handling through a narrow
adapter if the selected dialoguer release differs. Cancellation must
never confirm an operation. Preserve the application's interruption and
cursor-restoration behavior; do not install a competing signal handler.

## Delivery and verification

Deliver three independently reviewable changes: pest, snapbox, and
dialoguer. Each includes its dependency changes, focused verification,
and relevant documentation. The snapbox change should precede dialoguer
so captured-output tests can cover its nonterminal behavior. Parser and
prompt changes must not alter history schemas or execution semantics.

Pest acceptance requires all existing picotmpl, core, and path-template
CLI tests plus coverage for Unicode offsets, EOF, quoted punctuation,
comments, token boundaries, duplicate settings, trailing input, legacy
syntax, malformed escapes, padding bounds, and nested guards/fallbacks.
Use fixed expected rendered paths including `examples/stef.tfmt`.

Snapbox acceptance requires fixture runs with matching and mismatching
stdout/stderr, absent and empty snapshots, missing snapshot files,
nonzero expected status, normalization, explicit snapshot updates,
step skipping, and report creation after assertion failure. Existing
audio checks and report viewer tests must continue to pass.

Dialoguer acceptance requires piped yes/no, uppercase and surrounding
whitespace, empty input, EOF, invalid-then-valid input, and bypass cases.
Verify acceptance and refusal preserve filesystem/history outcomes.
Exercise real terminal interaction using a PTY test if an existing
facility is available; otherwise record a manual terminal smoke test for
newline behavior, cancellation, output placement, and cursor restoration.
Snapshots alone do not verify terminal interaction.

Run targeted tests for each affected crate, then `cargo test --workspace`,
`cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.
Record toolchain/environment limitations rather than claiming unrun checks.
Compare release binary size and clean build cost before/after, and record
net code removed including grammar and adapters; no arbitrary reduction
target justifies weakening behavior.

Update README.md and CHANGELOG.md for user-visible prompt/diagnostic
differences. Update `tests/fixtures/cli/README.md` for snapshot authoring
and updates. Template examples require no syntax migration.

## Post-implementation maintainability gate

After each migration is implemented and behavior checks pass, review its
final code against the pre-migration revision. Repeat the assessment for
the combined result before declaring the migration complete. Passing
tests or reducing line count alone does not satisfy this gate.

For pest, snapbox, and dialoguer separately, record concrete before/after
examples and answer:

- **Clarity:** Is the normal flow easier to read? Are grammar rules,
  assertions, and prompt policy easier to find and understand? Account
  for generated-code concepts and library knowledge a maintainer needs.
- **Complexity:** Did custom branching, state, duplication, and error
  plumbing actually shrink? Count grammar, conversion code, adapters,
  compatibility fallbacks, and configuration as part of the new solution.
  Identify complexity moved elsewhere rather than removed.
- **Maintainability:** Is a representative future change easier to make
  and verify? Trace adding a language construct for pest, a command-output
  expectation for snapbox, and changing confirmation policy for dialoguer.
  Compare the files and concepts involved and how failures are diagnosed.
- **Dependency cost:** Are feature selection, build/binary cost, API
  coupling, and future upgrades proportionate to the demonstrated benefit?

Save the assessment beside this spec as
`2026-10-10-dependency-simplification-review.md`, with the compared Git
revisions, supporting file references, remaining drawbacks, and an
explicit **keep**, **revise**, or **revert** recommendation for each crate.
Do not presume all three migrations deserve the same verdict.

A migration is accepted only when the review demonstrates clearer code,
lower overall complexity, and improved maintainability without weakening
the behavior contract. If it fails that standard, simplify the integration
and reassess, or recommend reverting that individual migration. Report
the finding explicitly rather than retaining a dependency solely because
its adoption was planned. The completion report must include these
verdicts and any unresolved tradeoffs.

## References

- [Pest grammar semantics](https://pest.rs/book/grammars/syntax.html)
- [Pest pairs and spans](https://pest.rs/book/parser_api.html)
- [Snapbox command and snapshot facilities](https://docs.rs/snapbox/latest/snapbox/)
- [Dialoguer confirmation options](https://docs.rs/dialoguer/latest/dialoguer/struct.Confirm.html)
- [Existing path-language design](2026-10-07-path-template-language-design.md)
- [Interruption contract](2026-10-10-manual-interruption-handling-design.md)
