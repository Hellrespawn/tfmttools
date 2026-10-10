# Dependency simplification review

Baseline: `9178c1dcb964f1f01cc303d3a12b824550e8483c`.
Tests run serially at the user's request; the baseline parallel history-lock
failure is being addressed in another worktree. Serial baseline passed.

## Pest: keep (subject to final combined review)

Before: 152 lexer lines plus 348 parser lines manually tracked tokens,
indices, delimiter consumption, and lexical state. After: a 26-line grammar
and 342-line conversion/validation parser. The production machinery drops
132 lines including the grammar, with unchanged evaluator and public APIs.
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
cost measurements and final verification are recorded below when complete.

## Snapbox: provisional, reassess in combined review

The old harness only checked numeric status and audio/filesystem results.
Snapbox adds optional stdout/stderr expectations, newline/path normalization,
exact temporary-root redactions, fallible diffs and explicit update support.
The command path remains a standard command builder passed to snapbox once;
all output is captured once. Assertions feed structured report outcomes.
A maintainer adds an expectation by naming a neighboring text snapshot;
there is no custom diff/normalization/snapshot updater to maintain.

This adds functionality and application glue rather than reducing the
existing code's size: schema, report model/viewer and stream adapters must
be counted. The combined gate must distinguish this benefit from genuine
simplification of the original status-only harness. Snapbox 1.2.2 requires
Rust 1.85; defaults disabled, cmd and diff enabled. assert_cmd is removed
from test-cli and the resolved graph where unused; assert_fs remains.

Verified failure reporting, step skipping, retained work directories,
missing/empty/absent snapshots, meaningful whitespace/path mismatches,
normalization, expected exit 2, update opt-in and CI no-write policy.
The existing report schema remains version 1 with optional failure details
and old report compatibility.

## Dialoguer

Pending implementation and assessment.
