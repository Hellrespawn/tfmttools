# History simplification verification

Implementation branch: `simplify-sqlite-history`.
Base: `9178c1d`. Reviewed implementation: `8a45cd3`.

All seven stages of the [plan](2026-10-10-history-simplification.md) implement the
[approved spec](../specs/2026-10-10-history-simplification-design.md).

## Verification

- Baseline `cargo test --workspace`: passed.
- Focused `cargo xtask test-core`, `test-fs`, and `test-cli`: passed.
- `cargo xtask history-schema`: regenerated snapshot; schema tests passed.
- Final `cargo test --workspace`: passed.
- Final `cargo +nightly clippy --workspace --all-targets`: passed without warnings.
- Final `cargo xtask lint`: passed without warnings.
- `git diff --check`: passed.

Regression coverage includes required concrete JSON progress, eligibility/cursor
rules, old development schema rejection without rewriting, explicit ID ordering,
malformed patch input/storage, affected-record decoding, cache agreement with a
fresh reopen, first-action manual supersession, failed COMMIT, ambiguous committed
success, failed reload, and release of a duplicated session lock handle.

Independent read-only whole-branch review found no critical, important, minor,
or declined findings and judged the implementation ready to merge. The reviewer
inspected coverage and verification evidence without independently rerunning tests.

## Decisions recorded during implementation

- Ruling: Cache optimization removes unrelated-record decoding but retains active-record rereads, as the approved spec explicitly permits; append remains quadratic within one run.
- Task 3: Ruling: fix xtask stale history_recovery target to history_interruption — baseline cargo xtask test-cli failed on removed target; needed to run agreed verification, cost if wrong: test selection only.
- Task 3: Ruling: explicitly unlock on drop after closing SQLite — parallel CLI tests exposed inherited-handle lock retention, reproduced by duplicate handle RED; preserves existing release guarantee, cost if wrong: lock lifecycle regression.
- Task 6: Ruling: use a real deferred foreign-key failure to exercise failing COMMIT, plus a test-only after-commit error for ambiguity — tests cover both durable outcomes without mocking SQLite; cost if wrong: insufficient platform-specific commit error coverage.
- Task 6: Ruling: use ensure_usable directly rather than a duplicate require_usable forwarding helper — same guard at all fallible boundaries; cost if wrong: internal naming only.

## Practical limits

Affected-record refresh avoids decoding unrelated history. It still rereads the
active record; appending a long run can remain quadratic in that run's action
count. This matches the approved scope.

The initial SQLite schema is undeployed. Earlier development schema shapes are
rejected without rewriting; no migration or JSON import was added. Serialized
records now require explicit progress. Action and metadata payloads and manual
interruption handling retain their existing contracts.

The branch and isolated worktree remain available for integration. No merge,
push, or publication was performed.
