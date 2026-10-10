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

## Snapbox

Pending implementation and assessment.

## Dialoguer

Pending implementation and assessment.
