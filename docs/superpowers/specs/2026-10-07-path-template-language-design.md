# Path template language

Status: approved. The standalone `path-template` crate implements this
syntax. The CLI integration milestone is pending; the current binary
continues to use MiniJinja.

## Purpose and agreed scope

Replace MiniJinja with a small language for generating audio file paths.
Metadata, argument declarations, and the path rule belong to the same
language. The language must express the layout in `examples/stef.tfmt`,
including its nested optional pieces, artist fallback, year extraction,
and number padding.

The agreed syntax uses `:` for definitions, `?` for guards, `??` for
fallbacks, and `|` for formatting. Arguments have bare names; audio tags
have a `$` prefix. Literals are quoted. A bare `/` creates a path boundary.
Whitespace outside strings contributes nothing to the output.

Missing and empty values count as absent. Numeric zero counts as present.
There are no `truthy()` or `falsy()` functions. This deliberately changes
the zero behavior of guards and fallbacks from Jinja.

Migration is manual. The replacement does not evaluate Jinja templates or
TOML frontmatter. Existing tag sanitization, rename validation, retained
source extensions, and undo/redo remain requirements of the rename flow.
String and path arguments are validated instead of sanitized: forbidden
characters are errors, while uncontrolled tag values retain replacement
behavior. The same argument validation applies to declared defaults.

The following sections turn those decisions into a proposed specification.
Details not previously discussed are proposals for this review.

## Complete example

The separate example is [stef-next.tfmt](../../../examples/stef-next.tfmt):

```text
name: "Stef's layout"
description: "Group by artist and album, with a directory prefix."

arg prefix: path(
  default: "",
  description: "Directory prefix to place output under."
)

path: (
  {prefix} {$albumartist ?? $artist} /
  [$album?
    [$date?
      {$date | year}
      [$albumsort? "." {$albumsort | pad(2)}]
      " - "
    ]
    {$album} /
  ]
  [$discnumber? {$discnumber | pad(1)}]
  [$tracknumber? {$tracknumber | pad(2)} " - "]
  [$albumartist? [$artist? {$artist} " - "]]
  {$title}
)
```

For prefix `Music`, artist and album artist `Example Artist`, album
`Example Album`, date `2024-03-10`, album sort `2`, disc number `1`,
track number `3`, and title `Example Song`, the output components are:

```text
Music / Example Artist / 2024.02 - Example Album / 103 - Example Artist - Example Song
```

The spaces around `/` in that display separate components for readability;
they are not filename characters. The source extension is appended by the
rename flow. Without an album, the album directory is omitted. Without an
album artist, the artist fallback supplies the artist directory and the
artist prefix in the filename is omitted. The nested artist guard also
omits that prefix when an album artist exists but the track artist is
absent, avoiding an otherwise dangling `" - "` literal.

## Definitions and arguments

A script has exactly one `path` definition. Optional `name` and
`description` definitions provide listing metadata. The default display
name is the filename stem, or the existing inline script label.

Argument declarations use a type with optional named settings:

```text
arg prefix: path(default: "", description: "Output directory.")
arg suffix: string(default: "")
arg edition: int
```

Supported types are `string`, `int`, and `path`. The parentheses can be
omitted when there are no settings. Settings are separated by commas;
a trailing comma is allowed. Supported settings are `default` (a quoted
CLI value) and `description` (a string). There is no `required` setting:
an argument without a default is required. A default makes an argument
optional; `default: ""` supplies an absent string or path value when omitted.

CLI arguments remain positional and bind in declaration order. Defaults
apply only when the CLI argument is omitted. Omitting an argument without
a default is an error. A supplied empty string or path
is still a supplied argument, but evaluates as absent. Empty integer
input is invalid, including `default: ""` on an integer argument. Invalid
integer values and excess CLI arguments are errors, including when there
are no declarations.

String arguments reject every character in `FORBIDDEN_CHARACTERS`,
including `/`, `\`, and `~`. Path arguments accept `/` and `\` as structural
separators, then reject forbidden characters within each component. Neither
argument type replaces, removes, or trims characters from component text.
Accepted values are preserved; the completed target still undergoes the
existing path validation for component boundaries and reserved names.

Defaults undergo the same type and character checks during compilation,
even if a CLI value would override them. Supplied CLI arguments are checked
when binding arguments, before rendering any files, including arguments
not used by the path rule. A character validation error names the argument
and offending character; path errors also identify the component. Default
errors point to the declaration's source span.

Bare names must resolve to declared arguments; `$` references must resolve
to recognized tag names or aliases. An argument and a tag may share a name
because their references differ. Argument identifiers use ASCII letters,
digits, and underscores, beginning with a letter or underscore. Lookup is
case insensitive, matching existing tag lookup conventions. Duplicate
definitions, arguments, or settings are errors, including case variants.
Declarations may appear in any order and are resolved after parsing.

## Path expressions

A path is an ordered sequence of literals, interpolations, guards, and
separators. Adjacent expressions concatenate; spaces, tabs, and line
breaks outside strings are ignored. `#` introduces a line comment outside
strings.

Strings use double quotes. `\"` represents a quote and `\\` a backslash.
Other escape sequences are rejected. Braces, brackets, `$`, `?`, and `#`
are ordinary characters within a string. In path expression text literals,
literal `/` or `\` is rejected with guidance to use a bare `/`: separators
have a structural meaning, and quoted strings supply text within a
component. This restriction does not apply to metadata or argument default
strings; a path argument default may contain separators.

Interpolation selects a value and can apply formatters:

```text
{$title}
{prefix}
{$albumartist ?? $artist}
{$tracknumber | pad(2)}
```

Fallbacks select the first present value, evaluating left to right. More
than two alternatives are allowed. A quoted string can be the final
fallback, for example `{$artist ?? "Unknown Artist"}`. Empty string
fallbacks count as absent. Formatters apply to the selected result, so
`{$albumartist ?? $artist | pad(2)}` formats the chosen value. No arbitrary
arithmetic, boolean expressions, loops, includes, or user functions are
part of this language.

## Presence and guards

```text
[$album? {$album} /]
[!$album? "Singles" /]
[prefix? {prefix}]
```

A positive guard includes its contents if its reference is present; a
negative guard includes them if it is absent. Guards accept one argument
or tag reference, optionally preceded by `!`. Nested guards express
combined conditions. Guard contents are evaluated only when selected.

Tag presence is determined after the existing tag sanitization and
trimming. Argument presence is determined from the validated value, with
no character replacement or trimming. A missing value, an empty string,
or an empty path argument is absent. Whitespace only tags are absent;
whitespace only string arguments are nonempty and count as present, with
the completed target subject to existing path validation. Integer zero
and text `"0"` are present.

Direct interpolation of an absent value emits nothing. A formatter on an
absent value also emits nothing. Errors in unselected fallback branches
or unselected guard contents are not triggered by rendering. Unknown
names are always checked when compiling the entire script, including
references inside guards.

## Formatters

The initial built-ins are `year` and `pad(width)`.

`year` retains the existing extraction behavior: look for an ISO style
date first, then a day/month/year style date, then a four digit year.
A present value containing no matching year is a rendering error.

`pad(width)` left pads the value's displayed text with zeros to a minimum
width, matching the existing `zero_pad` filter. It never truncates text.
Width must be a nonnegative integer literal. Unknown formatters or invalid
argument counts are compile errors. Applying a formatter to a path
argument is a compile error. Multiple formatters may be chained and
execute left to right.

Retain the existing tag aliases, date fallback, track/disc/movement
current and total handling, and numeric coercion when reading tag values.
For example, a raw track value of `3/12` supplies current value `3` to
`$tracknumber` and total value `12` to `$tracktotal`.

## Path construction

Bare `/` ends the current component. The renderer constructs a path from
components and uses the platform's path representation, rather than
normalizing separator characters in a rendered template string.

Tag values and string arguments contribute text to a single component.
Separators in tags use the existing forbidden character replacement
handling; separators in string arguments are rejected. Neither can
introduce directories. Literals are validated by the existing filename
rules once a complete target has been constructed.

A `path` argument is split on both `/` and `\`, with empty input segments
discarded and each remaining segment checked for `FORBIDDEN_CHARACTERS`.
Component text is preserved without replacement or trimming.
The resolved value retains those segments instead of joining them into a
string with a trailing slash. Empty input resolves to an absent path.
This retains the current relative prefix interpretation of path arguments.

Interpolation of a path argument inserts complete directory components.
It is permitted only at a component boundary, including the start of the
path. A prefix such as `Music/Artists` inserts two directories, and the
next scalar interpolation starts the filename or next directory component.
Inserting a path argument after unfinished component text is an error.
An explicit `/` immediately after a nonempty path argument would request
an extra empty component and is an error; the example needs no such `/`.

A leading bare `/` requests the platform root. Other empty components,
repeated separators, a trailing separator, and an empty final filename
are errors. Conditional contents do not introduce components when skipped.
Paths remain subject to existing full plan validation, including
collisions, reserved names, and forbidden leading or trailing characters.
Windows drive and UNC prefix syntax is outside this initial language;
cross platform relative directory prefixes are supported.

## Parser, renderer, and crate boundaries

Implement the language in a separate workspace crate at
`crates/path-template/`, with the provisional package name `path-template`.
It must have no dependencies on other tfmt crates, `lofty`, or application
warning/history types. Give it its own package description and README.
Workspace package metadata and lints may be inherited initially; extraction
should require ordinary manifest edits, not a redesign of its API.

The language crate owns a small lexer, recursive descent parser, source
spans and diagnostics, metadata, argument declarations and binding,
guards, fallbacks, `year`/`pad` formatters, and structural path rendering.
Compiled scripts own their data. No environment, template registration,
filesystem loading, or source lifetime coupling is needed. Argument binding
is separate from compilation so metadata can be inspected without supplying
required arguments.

The caller supplies an argument validation policy containing forbidden
characters. For tfmt this comes from `FORBIDDEN_CHARACTERS`; the language
crate must not duplicate tfmt's table. Its path argument validation handles
structural `/` and `\` separators before checking component text. Defaults
are validated when compiling with that policy; CLI values are validated
when binding. Tags are supplied on demand by a caller resolver as optional
text or integer values. The resolver can fail, and its failure is reported
with the reference's source span. The language crate does not sanitize tags.

Expose all referenced tag names and their source spans so the tfmt adapter
can reject unrecognized aliases before rendering, including references in
skipped guards. The generic language does not contain an audio tag schema;
an absent value returned by a resolver means missing metadata.

Rendering returns owned path components and a root indicator, with a
method to construct a standard `PathBuf` using the host platform's
separator. tfmt converts that path to its UTF-8 path type. Tag lookup,
aliases, date fallback, number/total parsing, sanitization, and whitespace
warnings remain in `crates/core/src/templates/` as the adapter to audio
files. The audio file rename flow appends the source extension and resolves
the target against the working directory. The caller retains warnings;
they are not part of the language crate's return types.

Filesystem discovery stays in `crates/fs/`. Preserve current discovery
extensions during the transition, but every discovered file must use the
new syntax. `.tfmt` is the documented extension. CLI resolution, listing,
history metadata, and rename planning consume the compiled script API.
Remove MiniJinja dependencies from core, filesystem helpers, errors, and
the workspace when all consumers have moved. Frontmatter TOML parsing is
also removed; any TOML dependency removal must account for other users.

Implement and evaluate the standalone language crate first, using the new
Stef example as a real script and synthetic metadata values in its tests.
During this milestone the existing CLI and MiniJinja templates continue to
work. Integrating the crate and removing MiniJinja are the following
milestone, with a separate implementation plan. Equality-based staging and
case-only fixture scripts require explicit migration work in that plan:
their general Jinja comparisons are outside the agreed language grammar.

## Errors and migration

Errors identify the script, line and column, offending construct, and
expected syntax. Rendering errors also identify the audio file. Unsupported
legacy syntax receives a manual migration hint. A missing recognized tag
is not an error; an unknown tag name or undeclared argument is.

Migrate `examples/stef.tfmt`, all template fixtures, inline scripts in
tests, README examples, and CLI help in the implementation change. Add a
changelog entry covering the breaking syntax change, zero presence, and
argument validation replacing argument sanitization.
Use the new example as the reference migration; no converter is provided.

Inline `--script` accepts the same document syntax, for example
`path: ({$artist} / {$title})`. Existing historical file references work
after their files have been migrated. Historical stored Jinja scripts
cannot be reused for a new rename and require an explicit new script;
report this with a migration hint. Undo and redo continue using stored
actions and do not require parsing old scripts. The history serialization
format need not change.

## Verification criteria

Exercise parsing and diagnostics for every syntax form, nested guards,
whitespace independence, comments, quoted punctuation, invalid names,
duplicate definitions, malformed strings, and formatter argument errors.

Exercise argument binding for declaration order, defaults, missing
required arguments, invalid integers, excess values, and preservation of
accepted argument text. Verify rejection of every `FORBIDDEN_CHARACTERS`
entry in string arguments and path components, while accepting `/` and
`\` as path argument separators. Check invalid defaults even when
overridden, invalid unused CLI arguments, and unchanged empty defaults.
Exercise rendering for missing/empty values, zero, short circuited
fallback, negative guards, and skipped invalid dates. Verify that tag
values still use their existing character replacement behavior.

Compare old and new Stef layouts using fixed expected outputs covering
album/date/album sort combinations, artist fallback, missing artist,
disc and track numbers, and multi component prefixes. Document zero as
an intentional difference. The new example's nested artist guard must
omit the filename artist prefix when the artist is missing or empty.
Include a comparison with the old expression and document any additional
behavior difference exposed by that case in the migration guide.

Verify structural separators on Unix and Windows, tag separator
sanitization, component boundaries, invalid empty components, and source
extension retention. Existing fixture scenarios must still exercise the
same rename, undo/redo, collision, and staging behavior after migration.

Run the core and CLI targeted suites during implementation, followed by
`cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`,
and the repository format/lint gate. Document any environment limitation
or remaining failure rather than claiming a passing gate.
