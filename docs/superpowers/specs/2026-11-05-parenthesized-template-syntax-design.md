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
automatic migration command, new formatters, boolean literals or operators
beyond unary negation, audio tag preparation changes, or history format
migration is needed.

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

The user agreed that formatting binds before fallback, conditional guards,
and concatenation. The next expression guarded includes formatting and
fallback. Formatting any possible path-valued input is rejected.

Subsequent decisions add unary `!` as an expression operator, an optional
`: alternative` on conditionals, and replace `??` with Elvis `?:`. Numeric
zero remains present for both conditionals and Elvis. Boolean results may
be used in conditions but must never be emitted into a filename. Parse
expressions uniformly, then validate their possible result types in the AST
rather than encoding that restriction in separate grammar productions.

For booleans, the alternatives considered were grammar-restricted conditions
or automatic text output. AST validation was selected to keep expressions
composable and reject accidental boolean output at compilation.

## Surface syntax and binding

```text
"literal"                       # Literal text
$title                          # Metadata reference
prefix                          # Declared argument reference
/                               # Directory boundary
$album_artist ?: $artist        # Elvis fallback
$track_number | pad(2)           # Formatting
$album ? $album                 # Conditional without an alternative
$album ? "yes" : "no"          # Conditional with an alternative
!$album ? "Singles"             # Unary negation as a condition
("Part " $title)                # Grouping and concatenation
```

Adjacent expressions concatenate. Whitespace outside strings is ignored;
comments, escapes, identifier rules, and case handling stay unchanged.
Quoted delimiter characters remain literal text. Old `{...}` and `[...]`
syntax outside strings is rejected, as is the old `??` operator.

From strongest to weakest binding:

| Operation | Binding and evaluation |
| --- | --- |
| Parentheses | Form one operand from any enclosed expression |
| Unary negation, `!` | Negate the following operand; repeated negations nest right to left |
| Formatting, `|` | Apply to the nearest operand; chains run left to right |
| Elvis, `?:` | Select the first operand whose presence test succeeds, left to right |
| Conditional, `?` with optional `:` | Select one following expression per branch; nesting binds right to left |
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

### Elvis fallback

```text
$a ?: $b | pad(2)             # Use $a, otherwise formatted $b
($a ?: $b) | pad(2)           # Format the selected value
"By " $a ?: $b               # Concatenate "By " and the selected value
("By " $a) ?: "Unknown"      # Test the combined text for presence
```

In the last example a missing `$a` leaves `"By "`, which is present, so
`"Unknown"` is not selected. This is intentional. Fallback tests the value
of its operand, not whether every reference within it was present. Elvis
evaluates each operand at most once; it does not expand into a conditional
that resolves the selected value a second time. The right operand remains
lazy. A chain selects the first successful operand, or returns the final
operand if none succeeds. For ordinary absent values that emits nothing;
a final boolean remains a boolean and is subject to output validation.

### Conditionals and guards

```text
$track_number? $track_number | pad(2) " - "
# Only the formatted number is conditional; the separator always emits.

$track_number? ($track_number | pad(2) " - ")
# Both the number and separator are conditional.

$album? $a ?: $b
# Guard the entire fallback: $album? ($a ?: $b).

$a? $b? $title
# Right-nested guards: $a? ($b? $title).

$album ? "yes" : "no" "!"
# The exclamation mark follows either branch, unconditionally.

$a ? $b ? "both" : "only a" : "no a"
# $a ? ($b ? "both" : "only a") : "no a".
```

A conditional's condition does not emit output. If its test succeeds,
evaluate the next expression and return that result. Otherwise evaluate
the alternative after `:`, or return absence if no alternative exists.
Evaluate only the selected branch. Each branch is one expression, including
formatting, Elvis, or a nested conditional; use parentheses for multiple
adjacent expressions. An unparenthesized `:` attaches to the nearest
unmatched `?` in the same group. A dangling `:` is an error.

`?:` is one contiguous operator token. Whitespace is allowed around it but
not inside it: `$a ? : $b` is an invalid conditional with a missing branch.
Token recognition must distinguish Elvis from `?` followed by a branch.

### Unary negation and booleans

`!` reverses the same test used by conditionals and Elvis:

| Input | Test succeeds | Negation |
| --- | --- | --- |
| Missing or empty value | No | Boolean true |
| Nonempty text or any integer, including zero | Yes | Boolean false |
| Boolean false | No | Boolean true |
| Boolean true | Yes | Boolean false |

Structural values use the presence rule below. Booleans are expression
results, not new resolver values or argument types. `true` and `false`
are not reserved literals; unprefixed names still refer to arguments.

```text
!$album ? "no album" : $album
!($album_artist ?: $artist) ? "no artist"
!!$album ? "has album"
```

Boolean values may flow through grouping, Elvis, and conditional branches
when the enclosing expression consumes them as a test. A boolean false is
not treated as present merely because it has a value. Elvis uses the same
test as `?`, including for booleans; numeric zero still passes that test.

Booleans cannot be emitted, concatenated with other segments, or passed to
a text formatter. Reject these possibilities at compilation, even in a
branch that appears unreachable:

```text
path: (!$album)                         # Boolean output
path: ($artist? !$album)                # Possible boolean output
path: ("Album: " !$album)               # Boolean concatenation
path: ((!$album ?: "Unknown"))          # Could select boolean true
path: ((!$album | pad(2)) ? "missing")   # Boolean formatting
```

In contrast, `path: ((!$album) ? "missing" : $title)` is valid. A group
containing a single boolean preserves it; grouping does not stringify it.
The parser accepts boolean expressions uniformly; these failures come from
AST validation with a diagnostic on the offending expression.

## Grouping details proposed by the author

These details complete the agreed behavior and are subject to approval:

- Unary negation binds more tightly than formatting. Consequently
  `!$date | year` attempts to format a boolean and is a compile error;
  write `!($date | year)` to negate the formatted result. Likewise
  `!$a ?: $b` means `(!$a) ?: $b`; write `!($a ?: $b)` to negate a fallback.
  These binding details extend the agreed operator order.
- Conditional inputs may be any expressions. For example,
  `($a ?: $b)? $title` tests a fallback and
  `("By " $artist)? $title` tests concatenated text.
- Empty groups emit nothing and count as absent. A group around a single
  value preserves that value, including booleans, integers, and structural
  paths. Text concatenation displays integers as today; a concatenation
  containing only absent values is absent.
- A conditional branch may itself be a conditional or `/`. The latter
  conditionally emits a boundary; it never sets the root indicator.
- `path:` still requires an outer parenthesized expression. Thus
  `path: ($title)`, `path: ($a ?: $b)`, and
  `path: (($a ?: $b) | pad(2))` need no extra sequence wrapper.
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
sequence     := conditional*
conditional  := fallback ("?" conditional (":" conditional)?)?
              | "/"
fallback     := formatted ("?:" formatted)*
formatted    := unary ("|" formatter)*
unary        := "!" unary | atom
atom         := reference | quoted | "(" sequence ")"
```

This sketch omits lexical rules and formatter parameters, which stay as
today. It specifies binding rather than prescribing a parser implementation;
the optional alternative is consumed before returning to an outer
conditional. A bare ungrouped `/` is a structural segment, not an implicit
fallback or formatting operand.

## Values, paths, and validation

Missing and empty values are absent; integer zero and text `"0"` are
present. Boolean tests use their truth value. Concatenation preserves
literal content, including spaces, without coercing booleans to text.
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
a boolean, explicit boundary, or path argument, even through nested groups,
fallbacks, or conditional output. Unreachable alternatives are still checked.
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

AST validation must conservatively track possible results through every
group, fallback, and conditional. A conditional's result types come from
its branches, not its condition; a negation produces a boolean regardless
of its operand type. One-member groups preserve result types, while
multi-member concatenation requires every member to be nonboolean.
Do not rely on runtime resolver values or constant folding to allow a
potentially boolean output. A boolean confined to a condition is valid.
All output paths must be checked before binding or reading audio files.

## Complete example

```text
name: "Stef's layout"
description: "Group by artist and album, with a directory prefix."

arg prefix: path(
  default: "",
  description: "Directory prefix to place output under."
)

path: (
  prefix ($album_artist ?: $artist) /
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
3. Conditionals cover one expression per branch, including fallback and
   formatting. Cover separators, unconditional adjacent text, grouped bodies,
   absent alternatives, and nested alternatives attaching to the nearest
   unmatched `?`. Both `?:` and conditional branches must stay lazy, and
   Elvis must evaluate a selected operand only once.
4. Missing, empty, zero, invalid-date, and resolver-error cases behave as
   specified. Check laziness and compile-time validation of skipped branches.
5. Nested path groups and selected structural fallback values preserve
   boundaries. Reject formatting any potentially structural output. Check
   root handling, partial groups, path-argument boundaries, and empty paths.
6. Negation and boolean conditions work for missing, empty, zero, and
   nonempty values, including double negation. Compilation rejects boolean
   output, concatenation, and formatting through nested branches and
   fallbacks, including unreachable branches. Boolean tests never treat
   false as present merely because it is a boolean value.
7. Old delimiters and `??`, malformed groups, missing negation operands,
   missing conditional branches, dangling colons, split Elvis tokens, and
   other missing operands fail with useful source locations. Quoted
   delimiter characters remain valid literal content.
8. AST output shows operation nesting and binding without resolving values,
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
