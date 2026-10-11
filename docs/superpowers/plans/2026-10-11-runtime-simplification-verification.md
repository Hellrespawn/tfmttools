# Runtime simplification verification

Date: 2026-10-11
Branch: `runtime-simplification`
Worktree: `/home/stef/.codex/worktrees/7570/tfmttools`
Base: `9f14865c776b94540b34276a1316acb3b88deda5`
Pre-review implementation through: `3e78d982447d7058ce8ce402cb67637bb7963e91`

## Checks

- Baseline `cargo test --workspace`: 226 passed, zero failures.
- Final `cargo test --workspace`: 243 passed, zero failures.
- `cargo test -p tfmt --features debug`: 68 passed, zero failures.
- Focused core, filesystem, CLI, native-ID3, and fixture suites passed during delivery.
- `cargo +nightly clippy --workspace --all-targets`: passed without warnings.
  Initial collapsible-conditional and semicolon findings were fixed; the direct
  Clippy command and `cargo xtask lint` were rerun after the review fixes.
- Final `cargo xtask lint`: passed without warnings.
- `git diff --check`: passed.
- No schema/snapshot, stored action model, binary patch format, or independent
  raw fixture-reader changes. Existing exact-byte replay and failure tests pass.
- Production source search finds encoding-name parsing only at the stored-data
  conversion boundary. No `FsHandler`, dead option verbosity, whole-scan native
  tag map, or `RenameCycleDetector` remains. Verbose formatter use by logging is
  preserved.

Verification was performed on Linux. Other platforms were not executed.

## Regression evidence

- Input normalization test first failed with a relative validation directory
  versus the expected absolute directory. It now passes; process tests cover
  relative/absolute paths, spaces, and a symlink without changing display paths.
- Typed encoding tests first failed to compile against string builders/accessors;
  all four encodings now round-trip with unchanged canonical stored names.
- Native-reader/matcher and all-frame candidate tests first failed against
  absent interfaces. Native MPEG/WAV/AIFF reads, absent tags, malformed input,
  aliases, ordering, and mismatching duplicate frames now pass.
- Malformed-audio fix test first returned success and changed a valid input.
  Character/ID3 fixes and dry runs now fail inspection with no tag/history effects.
- Dry-run rename first printed its interrupted-run startup report twice. It now
  prints once; multi-record undo, unresolved attempts, writable closure, and
  no-effects guards pass alongside existing artifact-retention tests.
- Mechanical deletions and behavior-preserving planner changes used existing
  coverage rather than implementation-mirroring tests.

## Implementation decisions

1. Mechanical deletions/refactors use existing regression coverage. Risk: a
   missed mechanical regression; affected suites and workspace checks passed.
2. Inspection reads generic `TaggedFile` directly so ID3-only validation can
   accept a successfully parsed MP3 without ID3v2. Character validation retains
   its missing-primary-tag error. Risk: incorrect no-primary policy; covered by
   the explicit no-native regression.
3. The quiet execution guard returns the optional closed record so admission can
   report the transition without repeating closure queries or mutation logic.
   Risk: a private interface difference from the plan; no new persisted state.

## Independent review

A fresh independent reviewer examined `9f14865..3e78d98` against the approved
specification, implementation plan, and review focus. No Critical or Important
issues were found; the declined-to-judge list was empty. Two Minor findings
concerned explicit acceptance requirements and were addressed in one fix pass:

- `./input` and missing-directory diagnostic spelling regressed. New path tests
  failed against the reviewed implementation, then passed after validation
  walked the verified scope through its original spelling. This also removes
  the post-discovery prefix-rewriting pipeline.
- The malformed-file test ordered the failure before the valid input. It now
  covers both orderings for both fix types and dry runs. A temporary-directory
  probe against the original checkout's baseline binary confirmed that a later
  malformed input incorrectly allowed earlier fixes for both types; the new
  implementation passes the strengthened test without changes or records.

The complete workspace/debug suites, direct nightly Clippy, and xtask lint were
rerun after the fix pass. No findings are deferred. No second review was requested.

Additional decisions from the fix pass:

4. Addressed the review's two Minor findings because they are explicit approved
   acceptance requirements, rather than optional polish. Risk: additional
   focused verification work; all checks passed.
5. Discovery uses the verified scope's supplied lexical spelling to preserve
   `./` and traversal-error paths without rewriting error chains or creating a
   display-policy framework. Risk: relative discovery depends on command cwd
   remaining stable, as before; relative/absolute/symlink tests pass.

All seven implementation tasks and all eight specification findings are complete.
Branch and managed worktree are preserved for integration; nothing was merged
or pushed.
