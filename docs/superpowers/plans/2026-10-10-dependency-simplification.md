# Dependency Simplification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or, if the user selects delegation, superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Adopt pest, snapbox, and dialoguer only where the completed
migrations demonstrate clearer, less complex, more maintainable code.

**Architecture:** Pest produces the existing template AST behind the
existing public API. Snapbox supplies comparisons inside the current JSON
fixture/report harness. The prompt wrapper delegates terminal interaction
to dialoguer while retaining a small adapter for redirected streams.

**Tech Stack:** Rust workspace, pest/pest_derive, snapbox, dialoguer,
libtest-mimic, existing audio verification and report viewer.

**Spec:** [Dependency simplification design](../specs/2026-10-10-dependency-simplification-design.md).

## Global constraints

- Keep Rust 1.91.0 and edition 2024.
- Declare compatible dependency releases centrally; update Cargo.lock and
  enable only necessary features. Verify releases against official docs
  during implementation rather than assuming latest APIs/MSRV fit.
- Snapbox must not enter the production CLI dependency graph through
  normal dependencies; use it in the existing dev-only harness path.
- Preserve public template APIs, accepted language, canonical tag names,
  audio checks, history format, and filesystem/interruption guarantees.
- Preserve sequential fixture execution and report creation on failure.
- Snapshot updates require explicit local opt-in; default runs and CI
  must not overwrite expectations.
- Passing tests and smaller line counts alone do not pass acceptance.
- Leave unrelated work, including untracked history documents, untouched.

## Review focus

1. Implicit grammar whitespace must not split identifiers/escapes or eat
   quoted whitespace; characterize and compare these in Task 1.
2. Unicode and EOF diagnostics must identify the actual source position,
   without exposing generated rule names; verify in Task 1.
3. A snapshot mismatch or unreadable reference must still write a failed
   report and skip dependent steps; exercise the whole path in Task 2.
4. Normalization must preserve meaningful filename/error differences and
   raw output, including leading/trailing whitespace; verify in Task 2.
5. Redirected input, cancellation, and bypass must never accidentally
   approve effects or hang awaiting terminal input; verify in Task 3.

## Delivery and file map

Keep the three migrations independently reviewable. This umbrella plan
provides separate subsystem tasks because they share dependency policy and
the final acceptance gate. Execution order: pest, snapbox, dialoguer, then
combined review. Snapbox must precede dialoguer.

- Parser: new `crates/picotmpl/src/grammar.pest`, private parser conversion
  in `parser.rs`, removal of `lexer.rs`, existing integration tests.
- Test infrastructure: fixture schema in `crates/test-harness/src/data.rs`,
  report outcomes in `outcome.rs`, command comparison helper in
  `crates/test-cli/src/snapshot.rs`, runner wiring in `runner.rs`.
- Prompts: `crates/tfmt/src/ui/prompt.rs`, focused CLI integration tests.
- Evidence: `docs/superpowers/specs/2026-10-10-dependency-simplification-review.md`.
- Each subsystem owns its Cargo manifests, lockfile changes, documentation,
  and the corresponding section of the evidence document.

## Task 1: Replace template syntax machinery with pest

**Files:**
Create `crates/picotmpl/src/grammar.pest` and the review document above.
Modify root Cargo.toml/Cargo.lock, `crates/picotmpl/Cargo.toml`,
`crates/picotmpl/src/parser.rs`, `lib.rs`, tests `syntax.rs`,
`arguments.rs`, `rendering.rs`, and `README.md`/`CHANGELOG.md` where
diagnostic wording changes. Remove `crates/picotmpl/src/lexer.rs` last.

**Interfaces:** Preserve
`parser::parse(source: &str, policy: ArgumentPolicy) -> Result<Compiled, Diagnostic>`
and every public template signature. Generated rules and pairs stay private.
Consume the existing AST, Compiled, Diagnostic, Span, and ArgumentPolicy;
produce the same owned compiled representation and tag references.

- [ ] Record the starting Git revision and worktree state in the review
  document. Measure the baseline release binary size and clean release
  build duration in a separate temporary Cargo target directory. Record
  command, toolchain, profile, and timing conditions for later comparison.
- [ ] Add characterization tests before replacement: full-input rejection,
  duplicate/case-variant definitions and settings, arbitrary declaration
  order, trailing commas, comments, Unicode whitespace, quoted punctuation,
  permitted escapes and rejected other escapes, tag hyphens/case, split
  identifiers/escapes, nested guards, fallback/formatter chains, widths
  0/1024/1025, EOF, and legacy migration hints. Assert exact accepted
  values, error categories, byte offsets, and character columns as
  appropriate; avoid freezing incidental old syntax-error prose.
- [ ] Run `cargo test -p tfmttools-picotmpl` against the old implementation.
  These characterization tests should pass; resolve incorrect assumptions
  against current behavior rather than changing behavior to fit the plan.
- [ ] Add compatible pest/pest_derive dependencies and the private grammar.
  Use explicit full-document boundaries, atomic lexical rules, and string
  handling that preserve the characterized whitespace/escape behavior.
  Keep the old parser temporarily under a test-only module for comparison.
- [ ] Replace production `parse` with pair-to-AST conversion. Retain semantic
  checks, padding limit 1024, all tag references, and source spans. Preserve
  argument validation in template/args and leave render/path evaluation
  unchanged. Map syntax errors to actionable Diagnostic messages and keep
  the legacy-syntax hint without leaking Rule names.
- [ ] Compare old and new compilation on example/fixture sources and the
  boundary corpus. Compare success/failure, metadata, argument definitions,
  references, and public binding/render results with fixed resolver input.
  Compare error locations/categories where promised, not private AST
  equality. Record and resolve every disagreement.
- [ ] Run `cargo test -p tfmttools-picotmpl`,
  `cargo xtask test-core`, and `cargo xtask test-cli`; require passing
  existing Stef outputs, zero/presence, lazy resolver, extension, and
  invalid-path tests. Remove temporary comparison code and lexer only
  after parity; rerun the picotmpl suite after removal.
- [ ] Document any user-visible diagnostic changes. Complete the pest
  maintainability gate: compare grammar plus conversion/error code against
  old lexer/parser, trace adding one language construct, and record file
  evidence, costs, and keep/revise/revert. Revise and reassess if needed.
- [ ] Commit only this migration and its evidence with subject
  `Replace template parser with pest` once the acceptance gate passes.

## Task 2: Integrate snapbox without replacing the fixture harness

**Files:**
Modify root Cargo.toml/Cargo.lock, `crates/test-cli/Cargo.toml`,
`crates/test-cli/src/lib.rs`, `runner.rs`,
`crates/test-harness/src/data.rs`, `outcome.rs`,
`crates/test-harness/assets/report/viewer.js`,
`crates/test-harness/tests/report_viewer.rs`,
`tests/fixtures/cli/README.md`, and the review document.
Create `crates/test-cli/src/snapshot.rs` and representative `.stdout`/
`.stderr` files beside CLI case JSON files. Add unit tests inside the
schema, comparison, and runner modules to access private helpers.

**Interfaces:** Add optional `stdout`/`stderr: Option<Utf8PathBuf>` to
TestData with getters returning `Option<&Utf8Path>`.
Add `snapshot_failures: Vec<String>` to CommandOutcome, omitted from JSON
when empty, and `with_snapshot_failures(self, failures: Vec<String>) -> Self`
which marks nonempty failures as Failed without overwriting captured
streams or exit codes. Initialize it in all constructors.
The snapshot helper exposes privately within test-cli:
`compare_stream(actual: &[u8], expected: &Utf8Path, context: &TestContext) -> Result<(), String>`.
Thread the case JSON parent directory through run_test_case/run_test_step
to resolve references independently of the process cwd. Keep external
test_runner and report-writing interfaces stable.

- [ ] Add schema tests showing old JSON still loads, optional paths resolve
  from the case directory, omitted streams are unchecked, and empty files
  require empty output. Add outcome tests showing a matching exit code
  plus snapshot failure fails the command/step/case while retaining status
  codes and captured streams. Run affected crate tests and confirm the new
  fields/behavior fail before implementation.
- [ ] Implement schema and outcome extensions. Preserve raw decoded
  stdout/stderr without trimming in CommandOutcome; comparisons consume
  original Output bytes, never its display strings. Keep existing JSON
  fields and viewer compatibility, adding optional failure details.
- [ ] Add snapbox in test-cli with required command/comparison features.
  Implement compare_stream with the selected release's fallible comparison
  API. No assertion panic or catch_unwind adapter. Normalize line endings
  and redact only exact work/config root values. Use an explicit snapshot
  update environment opt-in and force read-only comparison in CI.
- [ ] Test compare_stream: exact match, stdout/stderr differences, CRLF,
  dynamic roots, meaningful filename/error/number differences, leading and
  trailing whitespace, absent/empty expectations, missing files, and local
  update opt-in versus CI/default no-write behavior. Keep environment
  mutation out of parallel tests by passing update policy internally or
  testing environment behavior in isolated child processes.
- [ ] Replace run_command's assert_cmd wrapper with snapbox command
  facilities. Keep tfmt binary discovery, explicit argv/config/run-id,
  current directory, numeric status checks, and a single process capture.
  Collect comparison failures into CommandOutcome rather than returning
  early from the case. Preserve checksums/audio checks and step skipping.
- [ ] Add runner-level tests using temporary, deliberately failing cases:
  matching nonzero exit status passes; mismatch and unreadable snapshot
  fail; the next step skips; failed work directory survives; JSON and HTML
  reports are written with raw streams and useful comparison details.
  Keep these negative cases outside the normal passing fixture discovery.
- [ ] Update viewer rendering/tests to show snapshot failures without
  assuming every failed command has a wrong exit code. Ensure old reports
  with no new field remain readable. Add help, validation-error, and
  rename-preview snapshots to normal passing fixtures; document authoring,
  normalization, and exact update command selected during implementation.
- [ ] Run `cargo test -p tfmttools-test-harness`,
  `cargo test -p tfmttools-test-cli`, and `cargo xtask test-integration`.
  Require all existing metadata/encoding checks and viewer tests to pass.
  Check Cargo dependency trees to confirm snapbox stays outside the normal
  tfmt dependency graph. Remove assert_cmd only where unused.
- [ ] Complete snapbox's maintainability gate, including the added schema,
  report plumbing, and adapter costs. Trace adding an output expectation
  and diagnosing a failure. Record keep/revise/revert, revise if necessary,
  then commit `Use snapbox for CLI output verification`.

## Task 3: Delegate terminal confirmations to dialoguer

**Files:**
Modify root Cargo.toml/Cargo.lock, `crates/tfmt/Cargo.toml`,
`crates/tfmt/src/ui/prompt.rs`, README.md, CHANGELOG.md, and review document.
Create `crates/tfmt/tests/confirmation.rs`; reuse existing history and
filesystem test setup patterns. Update prompt snapshots only for intended
presentation changes.

**Interfaces:** Preserve ConfirmationPrompt::new and
`ConfirmationPrompt::prompt(&self) -> color_eyre::Result<bool>`.
Add a private generic line-input helper accepting BufRead and Write so
nonterminal behavior and I/O errors are testable without global stdin.
No changes to command force/dry-run policy or signal handlers.

- [ ] Add table-driven line-adapter tests for y/yes/n/no, uppercase,
  surrounding whitespace, empty line, immediate EOF, invalid then valid,
  invalid then EOF, and read/write failures. Assert false on empty/EOF,
  retry guidance on invalid input, and errors rather than consent on I/O
  failure. Add a flush-observing writer to verify flushing before reading.
- [ ] Run prompt unit tests to observe missing-helper failures, then extract
  the existing line loop into the helper with the specified EOF/default
  behavior. Keep output on stdout and preserve existing retry wording.
- [ ] Add dialoguer with minimal features. Choose terminal interaction only
  when both stdin and stdout report IsTerminal. Configure Confirm with
  default false, wait_for_newline(true), plain theme, explicit stdout Term,
  and report(false). Check selected release source/docs for trimming and
  case handling; implement the narrowest compatible adapter if required.
  Do not silently relax the input contract to fit library defaults.
- [ ] Add child-process tests for piped acceptance/refusal, whitespace/case,
  empty/EOF, retry, and redirected stdout. Verify filesystem and history
  outcomes for rename, undo/redo, and clear-history confirmations. Add
  bypass tests with stdin held open and a bounded timeout to show existing
  force/dry-run paths do not wait for input; assert their existing effects.
- [ ] Run `cargo xtask test-cli`. Exercise terminal y/yes/n/no, uppercase
  and trimmed input, Enter/default, invalid retry, cancellation, cursor
  restoration, and absence of extra selection output using an existing
  PTY facility or a recorded manual terminal smoke test. Verify redirected
  streams never cause a controlling-terminal read.
- [ ] Update README/CHANGELOG for actual presentation changes. Complete
  dialoguer's maintainability gate accounting for retained fallback and
  any terminal adapter. Trace a future confirmation policy change. If
  two implementations plus compatibility code are harder to maintain,
  record revise/revert rather than claiming adoption is a success.
- [ ] Commit accepted migration as `Use dialoguer for terminal confirmation`.

## Task 4: Verify combined behavior and make acceptance decisions

**Files:** Review document; revise affected implementation/docs only if
new findings require changes. No new behavior or unrelated refactoring.

**Interfaces:** Consume all three verified migrations and their evidence;
produce separate per-crate verdicts plus a combined acceptance assessment.

- [ ] Run `cargo test --workspace`,
  `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.
  Require successful exits; fix actionable findings or record exact
  environment limitations. Recheck the relevant targeted suite after fixes.
- [ ] Re-measure clean release build duration and binary size with baseline
  conditions. Record net code change including grammar, snapshots,
  adapters, report plumbing, and tests separately; identify moved complexity.
  Verify the Rust 1.91.0 constraint using an available MSRV toolchain,
  documenting inability to run it rather than assuming compatibility.
- [ ] Compare final code with the baseline revision for clarity, complexity,
  maintainability, and dependency cost. Record concrete file examples and
  representative future-change traces for each crate; repeat the analysis
  for interactions among parser diagnostics, snapshots, and prompts.
- [ ] Resolve each verdict: keep only with demonstrated improvements in
  all three quality criteria and intact behavior; revise then reassess when
  integration can improve; recommend reverting the individual migration
  when the evidence does not support keeping it. Do not revert unrelated
  work or automatically revert a shared dependency needed by another task.
- [ ] Check spec coverage, README/CHANGELOG/fixture guide consistency, report
  compatibility, and dependency removal. Commit the final review evidence.
  Completion report links the review and states each keep/revise/revert
  verdict, verification results, and unresolved tradeoffs. Do not declare
  the migration accepted while a required gate remains unresolved.
