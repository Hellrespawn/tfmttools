# Dependency simplification review

Baseline: `9178c1dcb964f1f01cc303d3a12b824550e8483c`.
Tests run serially at the user's request; the baseline parallel history-lock
failure is being addressed in another worktree. Serial baseline passed.

## Pest: keep

Before: 152 lexer lines plus 348 parser lines manually tracked tokens,
indices, delimiter consumption, and lexical state. After: a 26-line grammar
and a 336-line conversion/validation parser. The final parsing machinery
drops 138 lines including the grammar, with unchanged evaluator and APIs.
The grammar makes nesting, alternatives, and token boundaries visible in
one place. Rust conversion still owns semantic validation and byte spans.

Adding a construct previously touched lexer token definitions, lexing,
recursive-descent consumption, AST and rendering. It now touches the
explicit grammar, conversion, AST and rendering; token bookkeeping is
eliminated. Diagnostic mapping is the principal retained complexity.
Atomic lexical rules need pest knowledge; keyword and optional-setting
span mistakes were caught by characterization and differential tests.

Old/new comparison passed on 320 generated documents, Stef and five
fixture templates, comparing acceptance and full compiled representations.
The temporary old implementation/comparison was removed after parity;
permanent public syntax/render tests remain. Pest resolved to 2.9.3
(MSRV 1.83), with its std/memchr defaults and derive; clean build/binary
cost measurements and final verification are recorded below.

## Snapbox: revert under the simplification contract

Implemented and verified in commit `2cf25c7`, then reverted after the
maintainability gate and a fresh reviewer reached the same conclusion.
The trial added optional stdout/stderr expectations, normalization,
redactions, fallible diffs, updates, and report/viewer failure details.
All 13 fixtures and focused harness tests passed, including missing/empty
snapshots, default/CI no-write updates, expected exit 2, retained work,
step skipping, and JSON/HTML reporting after mismatches.

The baseline harness has no custom snapshot engine to replace. Replacing
its small assert_cmd invocation with snapbox's command wrapper did not
simplify it; optional snapshots added a schema, adapter, update policy,
report plumbing, and viewer handling. The new functionality is useful,
but is not a reduction in overall complexity compared with the existing
status/audio/filesystem harness. A future maintainer would have more
concepts and behavior to understand. Fewer third-party assertions would
not remove that added application glue.

Therefore this migration fails the explicit acceptance criterion. The
snapshot implementation, fixture additions, docs, dependency and report
changes were reverted together; existing audio assertions and report
contracts remain intact. The trial commit can seed a separately scoped
output-snapshot feature if desired. Its minor env-sensitive unit-test
finding is moot after removal (intentional mismatches should use an
explicit Verify action if that feature is revived).

## Dialoguer: revert

Trialed dialoguer 0.12.0 with default features disabled, Confirm's plain
style, false default, newline mode, stdout terminal and report(false).
A tested line-input fallback retained piped/redirected behavior.
The integration added a terminal branch without removing the line loop.

Real PTY tests against disposable copies of Nemo proved y/yes accept,
n/empty decline, and uppercase/trimmed YES accepts. However `yikes` and
`notyes` both renamed the file, violating whole-line validation. The
newline mode tracks individual y/n keystrokes rather than validating the
whole response. Escape/newline did not exit in the smoke test and the
probe killed it after eight seconds. Confirm also clears/redraws the line.

The old prompt has one small, uniform input loop. Keeping dialoguer would
retain that fallback and add terminal state, cancellation/cursor behavior,
and another input layer to validate complete responses. A future policy
change would then need to update and verify both paths. This is more
complex and less maintainable for this application, despite the useful
library APIs. The trial production changes and dependency were reverted;
the existing CLI flow is retained. Regression tests pin rejection of
ambiguous affirmative text and ordinary piped answers.

Ruling: reject the migration rather than weaken input semantics or add a
second prompt implementation to accommodate Confirm. This is the spec's
explicit per-dependency acceptance gate, not an unimplemented adoption.

## Combined result and verification

The accepted result retains pest and the added parser/confirmation
regressions. Snapbox and dialoguer are absent from the final dependency
graph. The original test harness, report model/viewer, and prompt production
code are restored exactly. No history or filesystem behavior changed.

A fresh reviewer examined baseline-to-branch changes and the final cleanup,
independently recommended keep/revert/revert, and found no remaining
blocking correctness issues. Reducing boilerplate was not used to excuse
new compatibility layers or unrelated feature infrastructure.

Final checks on the accepted code:

- `cargo test --workspace -- --test-threads=1`: 200 passed, zero failed,
  including all 12 original CLI cases and the new confirmation regression.
- `cargo +nightly clippy --workspace --all-targets`: pass, no warnings.
- `cargo xtask lint`: pass (format and shared clippy gates).
- `cargo +1.91.1 check --workspace --locked` with a separate target: pass.
  Exact Rust 1.91.0 is not installed; the nearest installed patch release
  was checked. The manifest floor remains Rust 1.91.0.
- `git diff --check`: pass.

Clean release build measurements used `cargo build --release --locked`,
Rust 1.97.0 on the same host, sequential builds in separate empty target
directories, and the baseline source archived at the pinned revision.
Dependencies were cached; this is one observation, not a statistical
benchmark or an isolated CPU measurement.

| Measurement | Baseline | Accepted parser |
| --- | ---: | ---: |
| Clean release build | 44.47 s | 48.32 s |
| Stripped release binary | 5,510,728 bytes | 5,564,456 bytes |
| Lexer/parser/grammar production lines | 500 | 362 |

The binary grew 53,728 bytes (about 1%); observed clean build time grew
3.85 seconds. These costs are proportionate to eliminating the custom
lexer/token cursor and making syntax declarative. Grammar and conversion
code are included in the line comparison; tests and prose are separate
costs. New permanent regression coverage adds 22 parser-test lines and a
confirmation integration test; the temporary differential harness and
failed integrations are not retained in production.

The snapshot trial is preserved in Git commit `2cf25c7` for reference;
its removal is part of the final acceptance commit. The dialoguer trial
was evaluated locally and reverted without a production commit.
