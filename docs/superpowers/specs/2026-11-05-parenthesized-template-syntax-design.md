# Parenthesized template syntax

Status: draft for approval. Supersedes the expression syntax in the
2026-10-07 path template language design after implementation.

## Intent and scope

Simplify picotmpl's punctuation while keeping separate template segments
explicit. Quoted strings already distinguish literals from references, so
simple value insertion needs no delimiters. Parentheses replace both braces
and square brackets for compound expressions and grouping.

This is a breaking syntax replacement, not an additional dialect. Migrate
the repository's executable templates, examples, tests, and current docs.
No compatibility parser, automatic migration command, new formatters,
general boolean expressions, or changes to audio tag preparation are needed.
Existing path safety, validation, and error handling remain requirements.

## Approaches and decisions

Two approaches were discussed:

1. Bare references with explicitly grouped compound expressions. This
   keeps template segments visible and avoids precedence between different
   operations. The user selected this approach.
2. Bare fallback and formatting operators governed by precedence, with
   parentheses only where needed. This saves punctuation but obscures
   segment boundaries and was not selected.

The user also decided to reject the old delimiters and require explicit
nesting when combining fallback and formatting. Neither the examples nor
the CLI end-to-end fixtures currently combine these operators; library
tests do and must be migrated.

## Syntax

Simple segments are quoted literals, references, or `/`:

```text
"literal"
$title
prefix
/
```

`$name` resolves metadata; an unprefixed name resolves a declared argument.
Adjacent segments concatenate. Whitespace outside strings is ignored;
comments, escapes, identifier rules, and case handling remain unchanged.

Compound segments use parentheses:

| Kind | Example | Meaning |
| --- | --- | --- |
| Fallback | `($album_artist ?? $artist ?? "Unknown")` | First present value |
| Formatting | `($track_number \| pad(2))` | Format one value |
| Positive condition | `($album? $album /)` | Emit contents when present |
| Negative condition | `(!$album? "Singles" /)` | Emit contents when absent |
| Sequence | `("Part " $title)` | Emit adjacent segments in order |

Each group contains one kind of operation. The following are errors:

```text
$a ?? $b
$track_number | pad(2)
($a ?? $b | pad(2))
($album_sort? "." $album_sort | pad(2))
($track_number? $track_number | pad(2) " - ")
```

Write explicit nesting instead:

```text
(($a ?? $b) | pad(2))
($album_sort? "." ($album_sort | pad(2)))
($track_number? ($track_number | pad(2)) " - ")
```

Repeating the same operation is allowed: fallback alternatives are checked
left to right, and a formatter chain such as `($date | year | pad(6))`
applies left to right. This introduces no precedence between operations.
`?` checks only a reference, optionally prefixed with `!`, as today.
Combined conditions still use nested conditional groups.

### Grouping details proposed by the author

These details complete the chosen syntax and are subject to approval:

- A fallback operand or formatter input may be a reference, quoted literal,
  or nested value group. A value group is a fallback, formatting operation,
  or parentheses around a single value. This permits
  `(($a | year) ?? "Unknown")` as well as formatting a selected fallback.
- A sequence containing multiple segments, a conditional, or `/` is not a
  value operand. For example, `(("A" $title) | pad(2))` is rejected rather
  than introducing formatting of concatenated output. Conditional groups
  emit segments; they do not manufacture a scalar or boolean value.
- Sequence groups can nest in emitting positions and are transparent to
  concatenation and path boundaries. Parentheses around a single reference
  or literal are accepted, although unnecessary. Empty sequence groups
  emit nothing; existing empty-component and empty-filename checks apply.
- `path:` still requires an outer parenthesized body. That group may itself
  be a sequence, fallback, formatting operation, or conditional. Thus
  `path: ($title)`, `path: ($a ?? $b)`, and
  `path: (($a ?? $b) | pad(2))` are valid without an extra wrapper.
- A root slash remains syntactic: it must be the first direct segment of
  the outer path sequence, as in `path: (/ $title)`. A slash inside a
  conditional or nested sequence is a boundary, never a root indicator.
  Empty or skipped segments cannot turn a later slash into a root.
- Metadata definitions remain quoted strings. Argument settings and
  formatter parameters keep their current parentheses and syntax; these
  are recognized in their respective declaration or operator contexts.

In grammar terms, emitting positions accept simple segments and groups.
A group is either a reference condition followed by emitting segments,
a value fallback, a value with one or more formatters, or a sequence of
emitting segments. Bare `?`, `??`, `!`, and `|` are never emitting segments.
Nested value groups delimit operands; a group cannot mix a value operation
with additional emitting segments. No expression precedence table is needed.

## Evaluation and validation

Preserve existing presence rules: missing and empty values are absent;
integer zero and text `"0"` are present. Missing formatted input remains
absent and does not invoke a formatter. A formatter's resulting text is
checked for presence if used as a fallback operand.

Fallbacks select values lazily, before emitting anything. Resolve and
format only operands needed to find the first present value. A present
operand that fails formatting is an error, not a reason to try the next
fallback. Conditions resolve their reference and evaluate their contents
only when selected. Resolver errors retain their tag name and source span.

Path arguments retain their structural value through fallback and grouping;
they are not flattened to text. They must still be inserted at a component
boundary. Reject formatting of any expression that could select a path
argument at compilation, including deeply nested and unselected fallback
alternatives. Validate all argument references and collect all metadata
references, even in skipped conditions or alternatives.

Keep formatter names, width validation, literal control-character checks,
argument binding and defaults, path construction, final component character
policy, and CLI schema validation unchanged. Diagnostics must identify the
offending source span using the new syntax. Reject old `{...}` and `[...]`
outside quoted strings, and explain the replacement where practical.

## Complete example

```text
name: "Stef's layout"
description: "Group by artist and album, with a directory prefix."

arg prefix: path(
  default: "",
  description: "Directory prefix to place output under."
)

path: (
  prefix ($album_artist ?? $artist) /
  ($album?
    ($date?
      ($date | year)
      ($album_sort? "." ($album_sort | pad(2)))
      " - "
    )
    $album /
  )
  $disc_number
  ($track_number? ($track_number | pad(2)) " - ")
  ($album_artist? ($artist? $artist " - "))
  $title
)
```

For the existing complete Stef test data and prefix `Music`, components
remain `Music`, `Example Artist`, `2024.02 - Example Album`, and
`103 - Example Artist - Example Song`. Missing optional tags retain their
existing output, including punctuation placement. The rename command still
appends the source extension.

## Implementation reach and acceptance

The implementation must cover:

- picotmpl grammar, parser, expression representation, evaluator, recursive
  validation, reference collection, source diagnostics, and AST dump.
- Library syntax, rendering, argument, Stef, and dump tests, including the
  standalone dump example. Debug output may change, but nested operations
  must remain visible; dumping must not resolve or render values.
- `examples/`, picotmpl test templates, CLI fixture templates, inline
  templates in Rust tests, fixture configuration, and CLI integration tests.
- README files, CHANGELOG, current usage examples, and developer commands
  containing template snippets. Historical design documents and retained
  history data are not rewritten as current templates.

Acceptance checks must demonstrate:

1. The complete Stef layout and existing CLI fixture scenarios keep their
   outputs after migration; listing and inline-template flows still work.
2. Bare references, grouped sequences, positive and negative conditions,
   nested guards, fallbacks, formatter chains, and explicitly nested mixed
   operations parse and render as specified.
3. Ungrouped operators, mixed operations in one group, malformed nesting,
   sequence or conditional value operands, and old delimiters fail with
   useful locations. Quoted delimiter characters remain literal text.
4. Missing, empty, zero, invalid date, and resolver-error cases preserve
   their semantics. Skipped conditions and fallback operands remain lazy,
   while compile-time checks still reach every reference.
5. Nested path-valued fallbacks preserve boundaries; formatting a possible
   path value fails at compilation. Root and empty-component rules hold.
6. AST inspection represents operation nesting and works through
   `cargo xtask dump-ast examples/stef.tfmt` with the new example.

Run the focused picotmpl tests and example tests, CLI fixture suite,
`cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`,
and the nightly formatting check during implementation.

## Left to the builder

Choose the smallest internal AST and parser changes that express these
rules, the exact debug tree labels, and diagnostic wording. No particular
new public API or dependency is required. Public compile, bind, render,
reference-inspection, and diagnostic interfaces should continue to serve
their existing callers. Syntax replacement is the intended user-facing
break; no filesystem or history format migration is part of this work.
