# Picotmpl Expression Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add comparisons, integer literals, text transformations, and explicit boolean operators to picotmpl.

**Architecture:** Extend the existing Pest grammar, owned AST, recursive capability validation, and lazy evaluator. Keep the public compile/bind/render API and caller-owned path sanitization. Use existing Rust string operations; add no dependencies or general expression framework.

**Tech Stack:** Rust 1.91.0, edition 2024, Pest, existing Cargo workspace test harness.

**Spec:** The proposed language contract below extends `../specs/2026-11-05-parenthesized-template-syntax-design.md`. Status: approved by the user and implemented on `picotmpl-expression-features` on 2026-11-05.

## Global Constraints

- Preserve existing expression binding, presence rules, path boundaries, diagnostics, and boolean output restrictions.
- Rust 1.91.0, edition 2024; no new dependencies.
- No arithmetic, boolean literals, regex replacement, loops, includes, functions, or new argument types.
- Do not change existing sample layouts merely to demonstrate new syntax.
- No external API consumers require compatibility, but existing executable templates must retain their behavior.

## Approved language contract

### Operators and binding

Strongest to weakest: parentheses, unary `!`, formatting `|`, Elvis `?:`, comparisons, `&&`, `||`, conditional `? :`, adjacency.

Keeping Elvis above comparisons allows `$artist ?: "Unknown" == "Unknown"` to compare the selected text. It also preserves every existing precedence relationship. Document parentheses for unfamiliar combinations. Comparison chains such as `1 < 2 < 3` are rejected; write `1 < 2 && 2 < 3`. Recognize `!=` as a comparison token, and `||` before the single formatting pipe.

```text
($artist != $album_artist) ? ($artist " - ")
$disc_total > 1 ? ($disc_number "-")
$album && $date ? ($date | year " - " $album /)
($album_artist || $artist) ? "Known" : "Unknown"
$title | trim | lower | replace(" ", "_")
```

### Integer literals

Signed decimal `i64` literals are expression atoms: `0`, `42`, `-1`. No leading plus, floats, exponents, or arithmetic. Leading zeros are accepted as decimal and render canonically (`007` emits `7`); quote text when zeros must be retained. Overflow fails compilation at the literal span. A minus must touch its digits. Reject numeric identifier suffixes such as `12abc` rather than interpreting them as accidental concatenation; whitespace may separate a number and a reference.

Integer zero remains present. A numeric literal emits its decimal representation and works with existing formatters and fallback. Keep padding width parsing separate, with its current 0..=1024 bound.

### Comparisons

Support `==`, `!=`, `<`, `<=`, `>`, `>=`. Results are booleans and cannot appear in output or text formatting, even through unreachable branches.

- Equality compares integers numerically, text exactly and case sensitively, and booleans by truth value. No Unicode normalization or implicit text-to-integer conversion. Two present values of different scalar types are unequal.
- Missing values and empty text are one absent category: both absent means equal; exactly one absent means unequal. This preserves the language's existing missing/empty equivalence. Absent is unequal to integer zero.
- Ordering requires two integers. If either operand is absent, return false. Otherwise a text or boolean operand is a diagnostic, not a lexical comparison or conversion.
- Evaluate left then right once each. Comparison does not short circuit; resolver failures propagate.
- Reject every potentially structural operand at compilation, including path arguments, separator-bearing groups, and structural conditional branches. Also reject statically known noninteger ordering operands. Resolver-dependent types are checked at render time.
- A nonstructural sequence compares as its concatenated text; a single grouped value retains its scalar type. Thus `(1 2) == "12"` is true and `(1) == "1"` is false. A conditional or fallback preserves its selected value's type.

### Boolean operators

`&&` binds above `||`; both associate left to right and return booleans, never an operand. Use the same presence test as `?`, `?:`, and `!`, including zero being present. Evaluate the right operand only when necessary. Existing structural presence tests remain legal as conditions, including operands of these operators. All references remain discoverable and all branches remain compile-validated, even when runtime evaluation skips them.

### Text transformations

- `| lower` and `| upper`: Rust Unicode case conversion, without locale-specific casing.
- `| trim`: Rust `str::trim`, including Unicode whitespace at both ends.
- `| replace("from", "to")`: replace all nonoverlapping literal matches, case sensitively. Reject an empty search string at compilation; allow an empty replacement.
- All accept scalar/grouped text through the same conversion as `year` and `pad`; integers stringify. Booleans and potentially structural inputs are compilation errors.
- Missing or empty input skips the formatter chain as today. Within an already running chain, continue applying all formatters even if an earlier transformation produces empty text. The final empty text is absent for enclosing fallback and conditions.
- Formatter parameters are compile-time literals, not arbitrary expressions. Only `replace` accepts two strings; `pad` still accepts one width; other formatters take no parentheses or parameters. Preserve formatter name case handling.
- Reuse quoted-string escape and prohibited-character validation for replacement parameters. Replacement may produce slash text, but must not introduce structural boundaries or bypass caller sanitization.

## Review Focus

- Mixed text/integer resolver values: equality must not silently coerce; ordering errors must identify the operator span (Task 2).
- `||`, `|`, `!=`, and `!` token boundaries and nested ternaries must parse predictably (Task 3).
- Empty transformed strings and Unicode expansion must retain fallback and formatting behavior (Task 4).
- Skipped operands must avoid resolver calls while still exposing references and compile-time errors (Tasks 2–3).
- Transformation-generated separators must pass through existing sanitization and strict validation before filesystem actions (Task 5).

## File map

- `crates/picotmpl/src/grammar.pest`: literals, precedence levels, formatter parameters.
- `crates/picotmpl/src/parser.rs`: AST construction, parameter checks, spans, diagnostics, reference collection.
- `crates/picotmpl/src/ast.rs`: integer/comparison/logical variants, formatter variants, AST dump traversal.
- `crates/picotmpl/src/validation.rs`: result capabilities and scalar/integer operand checks.
- `crates/picotmpl/src/value.rs`: shared scalar normalization and comparison semantics.
- `crates/picotmpl/src/render.rs`: comparison evaluation and lazy logical evaluation.
- `crates/picotmpl/src/format.rs`: text operations if helpers improve clarity; direct standard-library calls are sufficient otherwise.
- `crates/picotmpl/tests/{syntax,rendering,arguments,dump_ast,stef}.rs`: focused coverage and regressions.
- `crates/tfmt/tests/path_templates.rs`: caller integration, sanitization, CLI diagnostics.
- `crates/picotmpl/README.md`, `README.md`, `CHANGELOG.md`: language contract and user-visible additions.
- `examples/stef.tfmt`, existing CLI fixtures, and core adapter: verify behavior; modify only if an actual integration issue requires it. No exports or configuration changes are expected.

## Task 1: Integer expression literals

**Files:** grammar, parser, AST, validation, renderer; syntax/rendering/dump_ast tests from the file map.

**Interfaces:** Add `ExpressionKind::Integer(i64)`; evaluate it as existing `Value::Integer`. Public APIs remain unchanged.

- [x] Add tests `integer_expression_literals` and `integer_literal_diagnostics`: `0`, `007`, `-1`, both i64 bounds, overflow on either side, lone minus, `- 1`, `12abc`, numeric output, `0 ?: 9` yielding `0`, and `7 | pad(2)` yielding `07`. Check overflow spans and numeric AST output.
- [x] Run `cargo test -p tfmttools-picotmpl`; confirm new acceptance tests fail before implementation.
- [x] Add the numeric atom with token boundaries; parse directly as `i64`. Update AST rendering, evaluation, and validation without changing pad width behavior.
- [x] Run `cargo test -p tfmttools-picotmpl`; require all tests and doctests to pass.
- [x] Commit as `Add integer literals to template expressions`.

## Task 2: Scalar comparisons

**Files:** grammar, parser, AST, validation, value, renderer; syntax/rendering/dump_ast tests.

**Interfaces:** Add `ComparisonOperator::{Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual}` and `ExpressionKind::Compare { left: Box<Expression>, operator: ComparisonOperator, right: Box<Expression>, operator_span: Span }`. Add `Value::compare(self, other: Self, operator: ComparisonOperator, span: Span) -> Result<bool, Diagnostic>` to centralize normalization and comparison. Extend capability analysis to distinguish potential text/integer/boolean/absent/structural results; metadata remains dynamically text or integer or absent. Do not build a general type system.

- [x] Add `comparison_values_and_types`: each operator's true/false boundary cases; `1 == "1"` false; equal/unequal text and booleans; missing versus empty, zero, and another missing value; `(1 2) == "12"`; `(1) == "1"` false; comparisons of selected conditional/fallback values. Assert rendered branch text, not emitted boolean output.
- [x] Add `comparison_errors_and_validation`: overflow remains a literal error; chained comparisons, missing operands, boolean output/formatting, structural alternatives, known text ordering, and resolver-returned text ordering fail with useful spans. Absent ordering returns false. Invalid expressions in skipped branches still fail compilation.
- [x] Add resolver-call assertions proving both operands are evaluated once, left to right, and resolver errors retain their source and reference span. Check AST dump children and reference collection on both sides.
- [x] Run the picotmpl tests and confirm the new tests fail.
- [x] Implement the comparison precedence level, AST traversal, capability checks, and shared value comparison. Normalize a multi-child nonstructural sequence using existing text conversion; preserve single grouped values and selected branch types. Use operator spans for dynamic type errors.
- [x] Run `cargo test -p tfmttools-picotmpl`; require all tests to pass, including Stef regressions.
- [x] Commit as `Add scalar comparisons to templates`.

## Task 3: Lazy boolean conjunction and disjunction

**Files:** grammar, parser, AST, validation, renderer; syntax/rendering/dump_ast tests.

**Interfaces:** Add `LogicalOperator::{And, Or}` and `ExpressionKind::Logical { left: Box<Expression>, operator: LogicalOperator, right: Box<Expression> }`. Evaluation returns existing `Value::Boolean` and calls `Value::is_present`; reference inspection walks both children.

- [x] Add `logical_presence_and_precedence`: missing, empty, zero, text `"0"`, negated booleans, all truth-table combinations, structural presence, and left associativity. `1 == 1 || 1 == 2 && 1 == 2` must select true; grouped disjunction followed by conjunction must select false. Test existing Elvis/format/conditional binding alongside the new levels.
- [x] Add `logical_short_circuit`: resolver calls and errors are skipped for an absent left operand of `&&` and a present left operand of `||`; required operands run once. All references still appear in `tag_references()`.
- [x] Add syntax and validation cases for `||` versus `|`, `!=` versus `!`, lone `&`, missing operands, boolean output/concatenation/formatting, skipped invalid branches, nested ternaries, and comparison chains. Assert AST nesting for precedence.
- [x] Run picotmpl tests and confirm the new acceptance tests fail.
- [x] Implement precedence and short-circuit evaluation, updating capability analysis and dump traversal. Preserve unary negation's existing binding above formatting.
- [x] Run `cargo test -p tfmttools-picotmpl`; require all tests to pass.
- [x] Commit as `Add lazy boolean operators to templates`.

## Task 4: Text formatters and literal string parameters

**Files:** grammar, parser, AST, renderer, optional format helpers; syntax/rendering/arguments/dump_ast tests.

**Interfaces:** Extend `Formatter` with `Lower`, `Upper`, `Trim`, and `Replace { from: String, to: String }`. Preserve existing `Year(Span)` and `Pad(usize)`. Parse generic literal formatter parameter lists locally, then validate name-specific arity and types; do not expose a formatter registration API.

- [x] Add `text_formatter_results`: `" Ab C " | trim | lower | replace(" ", "_")` yields `ab_c`; `"Straße" | upper` yields `STRASSE`; Unicode whitespace trims; repeated replacement uses nonoverlapping matches; empty replacement deletes matches; integer input stringifies. An absent input remains absent. Trimmed-to-empty text selects an enclosing fallback. Existing `year` and `pad` chains remain unchanged.
- [x] Add `text_formatter_parameter_errors`: empty search, wrong count/type, unknown formatter, unsupported escapes, prohibited controls, parameter expressions, missing commas/closing parentheses, and arguments supplied to zero-argument formatters. Preserve pad width diagnostics and limits. Check escaped quotes/backslashes and replacement parameter spans.
- [x] Test boolean and structural formatter rejection through branches, replacement-generated slash remaining component text, and an empty intermediate result continuing through `pad(2)` to yield `00`.
- [x] Run picotmpl tests and confirm the new tests fail.
- [x] Extend formatter parsing and AST dump labels; implement transformations with `to_lowercase`, `to_uppercase`, `trim`, and `replace`. Reuse literal validation and existing formatter input conversion. Do not duplicate sanitization here.
- [x] Run `cargo test -p tfmttools-picotmpl`; require all tests to pass.
- [x] Commit as `Add text transformation formatters`.

## Task 5: CLI integration, documentation, and complete verification

**Files:** `crates/tfmt/tests/path_templates.rs`, both README files, CHANGELOG; core adapter or CLI fixtures only if verification reveals a required change.

**Interfaces:** Use unchanged `Template::compile`, `bind`, `render`, and `tag_references` through existing audio-template adapters. Confirm which audio tags arrive as integers in `crates/core/src/templates/context.rs` before selecting numeric CLI fixtures.

- [x] Add CLI tests using existing test helpers for equal artist suppression, a multi-disc numeric condition, combined presence tests, and title normalization. Assert exact target paths and preserved source extension. Use integer arguments if fixture metadata does not provide a suitable numeric tag.
- [x] Add CLI cases proving unknown metadata in a skipped logical operand is still rejected, dynamic ordering failures identify the template location, and replacement-generated separators follow default sanitization and strict-mode rejection without unintended directories or file changes.
- [x] Run `cargo xtask test-cli`; fix only integration failures caused by the additions. Retain existing case-only rename and staging coverage; do not rewrite fixture scenarios unnecessarily.
- [x] Update README syntax tables and migration/unsupported-feature statements, including the exact precedence table, missing/type rules, Unicode casing, lazy evaluation, formatter signatures, examples, and diagnostics. Add a CHANGELOG entry. Keep the old approved syntax design as history and link this extension from current docs as appropriate once accepted.
- [x] Run `cargo +nightly fmt --all`, `cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`. All must pass; record exact blockers if the environment prevents a check. Do not claim an unrun check passed.
- [x] Review the final diff for exhaustive AST matches, tag reference traversal in skipped branches, unchanged public API, no new dependencies, and preserved fixture output. Check `git diff --check`.
- [x] Commit as `Document and verify template expression features`.

## Completion criteria

All four requested feature groups have executable library coverage and documented semantics. CLI tests demonstrate real audio-template integration and caller-owned path safety. Existing picotmpl, Stef, CLI, history, and staging tests retain their behavior. Implementation is complete. Workspace tests and the nightly format/Clippy gate pass without warnings.


## Implementation notes

- Feature tests live in `crates/picotmpl/tests/expressions.rs` instead of expanding the existing syntax/rendering modules.
- Result capabilities track possible text/integer/boolean/structural output; absence is implicit and needs no extra flag. A localized Clippy allowance explains why these independent flags are intentional.
- The final independent review found a compatibility regression in grouped text after `year`. A failing regression test reproduced it; the grammar now preserves existing `"2024" | year (" suffix")` adjacency while validating the new formatter signatures.
- Verification: `cargo xtask test-cli`, `cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`, `cargo xtask lint`, and `git diff --check` passed. The final workspace and lint runs followed the compatibility fix; the final workspace suite includes all CLI tests.
- No new dependencies, public API changes, existing fixture changes, or changes to the example layout were needed.
