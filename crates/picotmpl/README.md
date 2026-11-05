# picotmpl

A small language for constructing paths from metadata. Templates contain
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
  {prefix} {$album_artist ?? $artist ?? "Unknown Artist"} /
  [$album? {$album} /]
  [$track_number? {$track_number | pad(2)} " - "]
  {$title}
)
```

Bare references name declared arguments; `$` references name metadata
fields. Adjacent expressions concatenate, and whitespace outside quoted
strings is ignored. `#` starts a comment outside a string. Strings preserve
all their text and support `\"` and `\\` escapes. Literal slashes remain component text. Use a bare `/` to end a directory component.

`[reference? contents]` includes contents when a value is present and
nonempty. `[!reference? contents]` handles missing or empty values. Integer
zero and text `"0"` are present. `??` chooses the first present alternative;
quoted strings can supply fallback text. Guards and fallbacks are lazy.
Missing interpolation values emit nothing. A final filename must be nonempty.

`| year` extracts a four digit year, prioritizing ISO style dates, then
day/month/year style dates, then other four digit year matches. A present
value with no matching year is an error. `| pad(width)` accepts widths from 0 through 1024 and left pads displayed
text with zeros to a minimum width; it never truncates text. Formatters
apply to the selected fallback value and can be chained. Path arguments
cannot be formatted.

## Compile, bind, and render

```rust
use std::convert::Infallible;

use tfmttools_picotmpl::{Scalar, Template};

let template = Template::compile(
    r#"
    name: "Artist and title"
    arg prefix: path(default: "")
    path: ({prefix} {$artist ?? "Unknown Artist"} / {$title})
    "#,
)?;

// Check the caller's schema before binding or reading any files.
for reference in template.tag_references() {
    assert!(["artist", "title"].contains(&reference.name.as_str()));
}
assert_eq!(template.metadata().name.as_deref(), Some("Artist and title"));

let bound = template.bind(&["Music".to_owned()])?;
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
required arguments. `Template` and `BoundTemplate` are owned, reusable values;
cloned templates share their compiled representation. Argument lookup is
case insensitive. Metadata reference names preserve their original spelling
when passed to the resolver. Metadata names may contain hyphens, as well
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

String arguments preserve their text. Path arguments split on `/` and `\`,
discard empty input segments, and preserve the remaining components.
Prohibited controls are rejected before coercion. Defaults are type-checked
at compilation; binding checks supplied values, including unused arguments,
before metadata resolution. Filename character replacement belongs to the caller
and should run on completed components after interpolation.

A path argument inserts complete directory components and must be used
at a component boundary. It supplies its own boundary, so write
`{prefix} {$title}` rather than `{prefix} / {$title}`. Path arguments are
relative prefixes; leading input separators are discarded when splitting.
A bare `/` as the first top-level expression sets the rendered root indicator.
An empty expression before `/` is an invalid empty component. Repeated
separators, trailing separators, and empty final filenames are errors.
`to_path_buf()` uses the host platform's separator. On Windows a leading
separator is rooted on the current drive; this language does not express
drive or UNC prefixes.

The library preserves component text without filename sanitization, including
slashes in literals and scalars. Callers must transform completed components
with `try_map_components()` before filesystem conversion, then validate paths
for reserved names, traversal, filename characters, and collisions.
`tfmt` applies its default or strict policy at this point. Controls, Unicode
line/paragraph separators, and bidi formatting controls are prohibited;
callers should also check raw metadata before numeric conversion or formatting.
The library does not append file extensions or create directories.

## Diagnostics and development

Inspect a template’s path AST as a compact tree:

```sh
cargo xtask dump-ast examples/stef.tfmt
# Or run the standalone Cargo example:
cargo run -p tfmttools-picotmpl --example dump_ast -- examples/stef.tfmt
```

The command reads one UTF-8 template file and writes one expression per line,
indenting guard contents and keeping fallbacks and formatters inline. It omits
source text, spans, and template metadata. For example:

```text
Path
  Interpolation prefix
  Interpolation $album_artist ?? $artist
  Separator
  Guard $album
    Interpolation $album
    Separator
  Interpolation $title
```

Rust callers can use `Template::dump_ast()` for the same output. Inspection
requires no argument binding or path rendering. File and compilation errors
are written to stderr with a nonzero exit status. Compilation errors include
the filename, line, and column. The debug output format may change.

`Diagnostic` contains a message and UTF-8 byte span. Compiled templates retain
their source text in the shared compiled representation. Use
`Template::format_diagnostic()` or `BoundTemplate::format_diagnostic()` to format
an error as `line:column: message`, with a one based character column. The
caller can add a filename or template label. Compilation errors can use
`Diagnostic::line_column(source)` before a compiled template exists.
Resolver errors keep the original error
and the failing reference's byte span. The full Stef layout and synthetic
metadata scenarios are in this package's `tests/` directory.

From the workspace root, run `cargo test -p tfmttools-picotmpl` or
`cargo clippy -p tfmttools-picotmpl --all-targets`. tfmt uses this crate through its
audio metadata adapter; the library remains independent of application code.
