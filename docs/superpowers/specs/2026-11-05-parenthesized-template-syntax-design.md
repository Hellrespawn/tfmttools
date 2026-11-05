# Parenthesized template expressions

Status: revised draft for approval. After implementation, supersedes the
expression syntax in the 2026-10-07 path template language design.

## Intent and scope

Simplify picotmpl's punctuation and give expressions consistent composition
rules. Quoted strings distinguish literals from references, so simple value
insertion needs no delimiters. Parentheses group expressions instead of
assigning separate delimiter shapes to insertion and conditions.

This replaces the old syntax. Migrate executable templates, examples,
tests, and current documentation together. Preserve existing layout output,
path safety, validation, and error handling. No compatibility dialect,
automatic migration command, new formatters, general boolean algebra,
audio tag preparation changes, or history format migration is needed.

## Approaches and decisions

Three approaches were considered:

1. Explicitly grouped operation kinds: a group contains a fallback,
   formatting, condition, or sequence, with nesting required to combine
   kinds. Initially selected, then rejected during review: the distinction
   between value and sequence groups was too strict and added special cases.
2. Expressions with precedence and ordinary grouping. Selected in the
   revised discussion: operators bind locally, and parentheses extend their
   operand to a longer expression. This allows formatting concatenated text
   without adding a separate kind of group.
3. Conditions consuming the remainder of their enclosing group. Rejected
   in favor of a guard applying only to the next expression; compound
   guarded output must be grouped explicitly.

The user agreed that formatting binds first, fallback next, guards next,
and concatenation last. The next expression guarded includes formatting
and fallback. Formatting any possible path-valued input is rejected.
The language's presence rules remain unchanged.

## Surface syntax and binding

```text
"literal"                       # Literal text
$title                          # Metadata reference
prefix                          # Declared argument reference
/                               # Directory boundary
$album_artist ?? $artist        # Fallback
$track_number | pad(2)           # Formatting
$album? $album                  # Positive guard
!$album? "Singles"              # Negative guard
("Part " $title)                # Grouping and concatenation
```

Adjacent expressions concatenate. Whitespace outside strings is ignored;
comments, escapes, identifier rules, and case handling stay unchanged.
Quoted delimiter characters remain literal text. Old `{...}` and `[...]`
syntax outside strings is rejected.

From strongest to weakest binding:

| Operation | Binding and evaluation |
| --- | --- |
| Parentheses | Form one operand from any enclosed expression |
| Formatting, `|` | Apply to the nearest operand; chains run left to right |
| Fallback, `??` | Select the first present operand, left to right |
| Guard, `?` or negative guard `!…?` | Test presence and guard the next expression; nesting binds right to left |
| Concatenation | Evaluate adjacent expressions in order |

A parenthesized group is an expression, not a distinct operation kind.
Operators can mix in a group according to this table. Parentheses are
optional around individual operations and required only to override binding
or make several adjacent expressions one operand.

### Formatting

```text
"." $album_sort | pad(2)       # "." followed by the padded number
("." $album_sort) | pad(2)     # Format the combined text
$date | year | pad(6)          # Extract year, then pad
```

Formatters use their current names, parameters, behavior, and width limits.
Formatting does not consume following adjacent expressions. Missing or empty
input stays absent without invoking the formatter.

### Fallback

```text
$a ?? $b | pad(2)             # Use $a, otherwise formatted $b
($a ?? $b) | pad(2)           # Format the selected value
"By " $a ?? $b               # Concatenate "By " and the selected value
("By " $a) ?? "Unknown"      # Test the combined text for presence
```

In the last example a missing `$a` leaves `"By "`, which is present, so
`"Unknown"` is not selected. This is intentional. Fallback tests the value
of its operand, not whether every reference within it was present.

### Guards

```text
$track_number? $track_number | pad(2) " - "
# Only the formatted number is conditional; the separator always emits.

$track_number? ($track_number | pad(2) " - ")
# Both the number and separator are conditional.

$album? $a ?? $b
# Guard the entire fallback: $album? ($a ?? $b).

$a? $b? $title
# Right-nested guards: $a? ($b? $title).
```

A guard's condition does not emit output. If it is present, evaluate the
next expression and return that result. Otherwise return absence without
evaluating the guarded expression. A negative guard reverses this test.
There is no implicit alternative branch.

## Grouping details proposed by the author

These details complete the agreed behavior and are subject to approval:

- Guard conditions may be expressions, using the same binding rules as
  elsewhere. For example, `($a ?? $b)? $title` tests a fallback and
  `("By " $artist)? $title` tests concatenated text. No new boolean value
  type is introduced. `!` is only the negative-guard marker, not a
  standalone operator that emits a boolean; `!$a` without `?` is invalid.
  `!$a ?? $b? $title` means `!($a ?? $b)? $title`.
- Empty groups emit nothing and count as absent. A group around a single
  value preserves that value, including integers and structural paths.
  Text concatenation displays integers as today; a concatenation containing
  only absent values is absent.
- A guard's next expression may itself be a guard or `/`. The latter
  conditionally emits a boundary; it never sets the root indicator.
- `path:` still requires an outer parenthesized expression. Thus
  `path: ($title)`, `path: ($a ?? $b)`, and
  `path: (($a ?? $b) | pad(2))` need no extra sequence wrapper.
- The root slash stays syntactic: only a bare slash that is the first
  direct segment of the outer path group sets the root indicator, as in
  `path: (/ $title)`. A slash inside a nested group or guard remains a
  directory boundary. Empty or skipped expressions cannot turn a later
  slash into a root.
- Metadata definitions stay quoted strings. Argument settings and formatter
  parameters keep their existing syntax; their parentheses are recognized
  in those declaration and operator contexts.

An equivalent grammar sketch is:

```text
path-body    := "(" sequence ")"
sequence     := guarded*
guarded      := "!" fallback "?" guarded
              | fallback "?" guarded
              | fallback
              | "/"
fallback     := formatted ("??" formatted)*
formatted    := atom ("|" formatter)*
atom         := reference | quoted | "(" sequence ")"
```

The negative-guard form requires `?`. This sketch omits lexical rules and
formatter parameters, which stay as today. It specifies binding rather
than prescribing a parser implementation. A bare ungrouped `/` is a
structural segment, not an implicit fallback or formatting operand.

## Values, paths, and validation

Missing and empty values are absent; integer zero and text `"0"` are
present. Concatenation preserves literal content, including spaces.
When grouping is used as an operand, evaluate its result before the outer
operator: a formatted or guarded result can itself be a fallback operand.
A selected operand's formatter or resolver failure is an error, not a
reason to try another fallback. Preserve resolver tag names and source spans.

Fallbacks and guards are lazy. Do not resolve or format skipped branches,
and do not append anything from a tested or discarded operand to the final
path. A condition may resolve a tag again in its body, as today; no caching
or resolver-purity guarantee is added.

Grouping and concatenation preserve directory boundaries and structural
path arguments rather than flattening them into separator-containing text.
A group may represent part of a path, including a trailing directory
boundary; only the completed outer path requires a nonempty final filename.
For presence testing, any nonempty text, inserted directory component, or
explicit boundary makes a structural fragment present. A fragment consisting
only of absent expressions is absent. Boundary and path-insertion checks
apply in the surrounding path when the selected result is emitted; grouping
must not introduce an artificial component boundary or bypass validation.

Reject formatting at compilation whenever the input's result could include
an explicit boundary or a path argument, even through nested groups,
fallbacks, or guarded output. Unreachable alternatives are still checked.
A path-valued guard condition does not itself make the guarded text output
path-valued. Quoted slashes remain text, as today, and the completed-component
character policy still applies to them.

Validate every argument reference and collect every metadata reference,
including those inside unselected branches and nested groups. Keep argument
binding, defaults, literal control-character checks, CLI schema validation,
component character handling, empty-component errors, and path-argument
insertion rules unchanged. Diagnostics must identify offending source spans
using the new syntax and explain replacement of old delimiters where
practical. No new behavior may silently stringify structural paths or
suppress data-loss protection.

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
  $album? (
    $date? (
      $date | year
      $album_sort? ("." $album_sort | pad(2))
      " - "
    )
    $album /
  )
  $disc_number
  $track_number? ($track_number | pad(2) " - ")
  $album_artist? $artist? ($artist " - ")
  $title
)
```

For the existing complete Stef test data and prefix `Music`, components
remain `Music`, `Example Artist`, `2024.02 - Example Album`, and
`103 - Example Artist - Example Song`. Missing optional tags retain existing
output, including punctuation placement. The rename command still appends
the source extension. Parentheses around the artist fallback are optional
but help distinguish the directory's segments.

## Implementation reach and acceptance

Cover picotmpl's grammar, parser, expression representation, evaluator,
recursive validation, reference collection, diagnostics, and AST dump.
Migrate library tests and templates, the standalone dump example tests,
CLI integration tests and inline templates, `examples/`, CLI fixture
configuration and templates, README files, and CHANGELOG. Historical design
documents and retained history records are not current executable templates
and must not be mechanically rewritten. Existing undo/redo behavior remains
covered by regression tests.

Acceptance checks must demonstrate:

1. The Stef layout and existing CLI scenarios keep their outputs after
   migration; listing, inline templates, and AST inspection still work.
2. The precedence examples above produce the stated output. Cover
   formatting concatenated text, formatting only the fallback's last
   alternative versus formatting its selected value, and concatenation
   that keeps a fallback operand present despite an absent reference.
3. Guards cover exactly one following expression, including fallback and
   formatting. Cover conditional separators, unconditional adjacent text,
   grouped compound bodies, right-nested guards, and negative guards.
4. Missing, empty, zero, invalid-date, and resolver-error cases behave as
   specified. Check laziness and compile-time validation of skipped branches.
5. Nested path groups and selected structural fallback values preserve
   boundaries. Reject formatting any potentially structural output. Check
   root handling, partial groups, path-argument boundaries, and empty paths.
6. Old delimiters, malformed groups, standalone `!`, missing guard bodies,
   and operators missing operands fail with useful source locations.
   Quoted delimiter characters remain valid literal content.
7. AST output shows operation nesting and binding without resolving values,
   and `cargo xtask dump-ast examples/stef.tfmt` works with the new example.

During implementation run focused picotmpl tests and example tests, the CLI
fixture suite, `cargo test --workspace`,
`cargo +nightly clippy --workspace --all-targets`, and the nightly format
check. This revision is a specification only; it does not change the parser.

## Left to the builder

Choose the smallest internal representation and parsing changes that
implement these expression and structural-value semantics. Exact debug
labels and diagnostic wording are implementation choices. No particular
new public API or dependency is required. Existing compile, bind, render,
reference-inspection, and diagnostic interfaces should keep serving their
callers. The syntax replacement is the intended user-facing break.
