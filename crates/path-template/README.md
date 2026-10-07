# path-template

A small language for constructing paths from metadata. Scripts contain
listing metadata, typed argument declarations, and a path rule. Compilation
owns its data; rendering uses a caller supplied resolver and performs no
filesystem operations.

This crate is implemented as an independent workspace member. Its API and
tests have no dependencies on tfmt, audio libraries, or sibling crate files.
It has not been published separately. Extracting it requires replacing
inherited Cargo metadata, dependency versions, and lint settings in its
manifest; the language API does not depend on the workspace.

## Example

```text
name: "Artist and title"
arg prefix: path(default: "", description: "Output directory.")

path: (
  {prefix} {$albumartist ?? $artist ?? "Unknown Artist"} /
  [$album? {$album} /]
  [$tracknumber? {$tracknumber | pad(2)} " - "]
  {$title}
)
```

Bare references name declared arguments; `$` references name metadata
fields. Adjacent expressions concatenate, and whitespace outside quoted
strings is ignored. `#` starts a comment outside a string. Strings preserve
all their text and support `\"` and `\\` escapes. Literal separator text is
rejected in the path rule: use a bare `/` to end a directory component.

`[reference? contents]` includes contents when a value is present and
nonempty. `[!reference? contents]` handles missing or empty values. Integer
zero and text `"0"` are present. `??` chooses the first present alternative;
quoted strings can supply fallback text. Guards and fallbacks are lazy.
Missing interpolation values emit nothing. A final filename must be nonempty.

`| year` extracts a four digit year, prioritizing ISO style dates, then
day/month/year style dates, then other four digit year matches. A present
value with no matching year is an error. `| pad(width)` left pads displayed
text with zeros to a minimum width; it never truncates text. Formatters
apply to the selected fallback value and can be chained. Path arguments
cannot be formatted.

## Compile, bind, and render

```rust
use std::convert::Infallible;

use path_template::{ArgumentPolicy, Scalar, Script};

let script = Script::compile(
    r#"
    name: "Artist and title"
    arg prefix: path(default: "")
    path: ({prefix} {$artist ?? "Unknown Artist"} / {$title})
    "#,
    ArgumentPolicy::new(&['?', '*', '~']),
)?;

// Check the caller's schema before binding or reading any files.
for reference in script.tag_references() {
    assert!(["artist", "title"].contains(&reference.name.as_str()));
}
assert_eq!(script.metadata().name.as_deref(), Some("Artist and title"));

let bound = script.bind(&["Music".to_owned()])?;
let path = bound.render(|name| {
    let value = match name {
        "artist" => Some(Scalar::Text("Example Artist".to_owned())),
        "title" => Some(Scalar::Text("Example Song".to_owned())),
        _ => None,
    };
    Ok::<_, Infallible>(value)
})?;

assert_eq!(path.components(), ["Music", "Example Artist", "Example Song"]);
let native_path = path.to_path_buf();
# let _ = native_path;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Metadata and argument declarations can be inspected before supplying
required arguments. `Script` and `BoundScript` are owned, reusable values;
cloned scripts share their compiled representation. Argument lookup is
case insensitive, and reference names passed to the metadata resolver are
normalized to ASCII lowercase. Metadata names may contain hyphens, as well
as letters, digits, and underscores. The library does not impose an audio
schema; `tag_references()` exposes every occurrence and source span,
including references inside skipped guards, for the caller to validate.

The resolver returns `None` for missing metadata or a text/integer value.
It may return its own error type, which `RenderError::Resolver` preserves
with the tag name and source span. A resolver can also collect application
warnings without introducing warning types into the library.

## Argument validation and path boundaries

Arguments bind positionally in declaration order. Supported types are
`string`, `int`, and `path`, with optional `default` and `description`
settings. An argument without a default is required. `default: ""` makes
a string or path argument optional and absent when omitted; it is invalid
for an integer argument. Defaults apply only to omitted values, and a
supplied empty string overrides a nonempty default. Extra values are errors.

The caller supplies forbidden characters through `ArgumentPolicy`. String
arguments reject those characters and path separators. Path arguments
split on `/` and `\`, discard empty input segments, and validate the
remaining components against the policy. Component text is preserved,
including whitespace and trailing periods; there is no trimming,
replacement, or removal. Defaults are validated during compilation even
when overridden later. Binding checks supplied values, including unused
arguments, before any metadata is resolved.

A path argument inserts complete directory components and must be used
at a component boundary. It supplies its own boundary, so write
`{prefix} {$title}` rather than `{prefix} / {$title}`. Path arguments are
relative prefixes; leading input separators are discarded when splitting.
A leading bare `/` in the rule sets the rendered root indicator. Repeated
separators, trailing separators, and empty final filenames are errors.
`to_path_buf()` uses the host platform's separator. On Windows a leading
separator is rooted on the current drive; this language does not express
drive or UNC prefixes.

Metadata preparation belongs to the caller. The library does not trim,
sanitize, or replace metadata text; separators in scalar values are
rejected when they would enter a path component. Callers should prepare
uncontrolled values before returning them, and validate complete paths
against their filesystem rules, including reserved names, traversal,
filename characters, and collisions. The library does not append file
extensions or create directories.

## Diagnostics and development

`Diagnostic` contains a message and UTF-8 byte span. Use
`line_column(source)` for a one based line and character column; the caller
can add a filename or script label. Resolver errors keep the original error
and the failing reference's byte span. The full Stef layout and synthetic
metadata scenarios are in this package's `tests/` directory.

From the workspace root, run `cargo test -p path-template` or
`cargo clippy -p path-template --all-targets`. CLI integration into tfmt is
a following milestone; this crate currently evaluates the new language
independently of the application's `MiniJinja` engine.
