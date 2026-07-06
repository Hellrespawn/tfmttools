# Script Frontmatter Design

Date: 2026-07-06

## Problem

Script (`.tfmt`/`.jinja`/`.j2`) files currently have no way to describe
themselves beyond a freeform leading `{# ... #}` minijinja comment, which
`TemplateLoader::description` scrapes as plain text. Template arguments
(`args[0]`, `args[1]`, ...) are entirely undocumented, untyped, and
unvalidated: any number of positional strings can be passed on the CLI, and
the template body accesses them purely by index. There is no way to know
what a script expects, whether a required argument was omitted, or whether
an argument's value is well-formed, short of reading the script source.

This design adds an optional TOML frontmatter block to script files that
declares basic metadata (display name, description) and typed argument
specs (name, type, required/default, description). It also tightens the
existing untyped/unsanitized argument behavior to close a filename-safety
gap, and introduces two deprecation warnings to steer scripts toward the
new model over time.

## Format and location

A script may begin with a TOML block fenced by `+++` on its own line,
recognized only at the very start of the file:

```
+++
name = "Stef's layout"
description = "Group by artist and album, with a directory prefix."

args = [
    { name = "prefix", type = "path", required = true, description = "Directory prefix to place output under." },
    { name = "extra", type = "string", required = false, default = "", description = "Optional suffix tag." },
]
+++
{{- args[0] -}}
{{- albumartist or artist -}}
...
```

- `TemplateLoader` strips the `+++...+++` block from the raw file content
  before registering the remaining body with the minijinja `Environment`.
  The jinja body is unaffected and remains valid minijinja on its own.
- An unterminated frontmatter block (starts with `+++` but no closing
  `+++`) is a load-time error.
- Files with no frontmatter block are entirely unaffected by this feature
  (see "Backward compatibility and deprecation warnings" below).
- `name`, if present, overrides only the *display* name shown in
  `tfmt list-templates`. The lookup name used by `--template <name>` and
  stored in rename history remains the filename stem, to avoid breaking
  existing invocations when a script gains frontmatter.
- `description`, if present, is the only source of the template's
  description — no fallback to a leading comment when frontmatter exists
  (see deprecation section).

## Data model

New module `crates/core/src/templates/frontmatter.rs`:

```rust
#[derive(Deserialize)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    args: Vec<ArgSpec>,
}

#[derive(Deserialize)]
struct ArgSpec {
    name: String,
    #[serde(rename = "type", default)]
    kind: ArgKind,       // String | Int | Path, default = String
    #[serde(default)]
    required: bool,
    default: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize, Default)]
enum ArgKind {
    #[default]
    String,
    Int,
    Path,
}
```

`toml` is added as a dependency of `crates/core` for parsing. No new
dependency is needed in `crates/fs` or `crates/tfmt`.

A frontmatter block whose `args` list contains two entries with the same
`name` is a load-time error (ambiguous for both positional matching and
named lookup).

## Argument resolution

Given a script's declared `args` (in declaration order) and the CLI's
positional `Vec<String>`:

1. Positional CLI values are matched to `ArgSpec`s by index (declaration
   order == positional order, same as today's implicit convention).
2. A declared arg with no corresponding CLI value:
   - uses `default` if present,
   - otherwise, if `required = true`, is a hard error before rendering,
     naming the argument and its `description`.
   - otherwise resolves to undefined (same as an omitted optional value
     today).
3. More positional CLI values are supplied than declared args: hard error
   before rendering (catches ordering/typo mistakes) — **unless the script
   has no frontmatter/no declared args at all**, in which case behavior is
   completely unchanged (unlimited raw positional args, as today).
4. Each resolved value is coerced/sanitized per `kind`:
   - `Int`: parsed as a number; a parse failure is a hard error naming the
     argument and its description.
   - `String`: sanitized as a single unit using the same
     forbidden-character table and trailing-period trim already applied to
     tag values (`AudioFileContext::remove_forbidden_characters`).
   - `Path`: split on `/` and `\`, each segment sanitized individually
     with the same rules as `String`, then rejoined with `/` and
     normalized to end with exactly one trailing `/` (regardless of
     whether the CLI value had zero, one, or several trailing separators,
     and regardless of which separator style was used).

Scripts with no frontmatter (or an empty `args` list) keep today's exact
behavior for `args[N]`: raw, unsanitized, positional-only, unlimited
count.

## Template access

Declared arguments become accessible in the template body both by name
(`{{ prefix }}`) and via `args[N]` (both yield the same resolved,
typed, sanitized value). `AudioFileContext` gains a name→`Value` map
populated from resolved args; `get_value` checks this map before falling
through to tag-key lookup, so a declared arg name can deliberately shadow
a tag lookup (documented behavior, not guarded against — script authors
are trusted).

## Backward compatibility and deprecation warnings

The goal is for every script to end up either small and argless with no
frontmatter, or fully declared with frontmatter and named-only access.
Two independent, `tracing::warn!`-based deprecation paths and one hard
error enforce migration without breaking existing scripts immediately:

| Frontmatter present? | Body uses `args[N]`? | Leading `{# #}` comment used as description? | Result |
|---|---|---|---|
| No | No | No | Unaffected (today's simplest case) |
| No | Yes | — | Warn: "uses positional `args[N]` without frontmatter; declare arguments to migrate" |
| No | — | Yes | Warn: "uses a leading comment as its description; move it to frontmatter's `description` field" |
| Yes | Yes | — | **Hard error** at load time: frontmatter presence opts a script into the named-args model, so raw indexing is disallowed even if `args` is empty |
| Yes | No | (ignored) | Fine — description comes only from `frontmatter.description`, which may be absent |

`args[N]` and comment-as-description detection are both simple regex
scans over the post-frontmatter-stripped body (`args\s*\[` for indexing;
existing leading-`{# #}` detection reused for the comment case) — not
full AST introspection, consistent with "basic" scope.

## Error handling

New `TFMTError` variants in `crates/core/src/error.rs`:

- Frontmatter TOML parse failure (wraps `toml::de::Error`, includes path).
- Unterminated frontmatter block.
- Missing required argument (name + description).
- Too many positional arguments supplied for a declared script.
- Argument type-coercion failure (name + description + offending value).
- Indexed `args[N]` access used in a script that has a frontmatter block.

All are hard errors raised before rendering begins (no partial renders).

## Affected files

- `crates/core/src/templates/frontmatter.rs` (new): `Frontmatter`,
  `ArgSpec`, `ArgKind`, and the resolution/coercion/sanitization logic.
- `crates/core/src/templates/template.rs`: `Template` gains resolved
  named argument values; `render` builds `AudioFileContext` with both the
  raw positional list (back-compat) and the name→`Value` map.
- `crates/core/src/templates/context.rs`: `AudioFileContext::get_value`
  checks declared-arg names before falling through to tag lookup.
- `crates/core/src/error.rs`: new variants listed above.
- `crates/core/Cargo.toml`: add `toml` dependency.
- `crates/fs/src/template.rs` (`TemplateLoader`): frontmatter-block
  stripping/parsing, the two deprecation-warning scans, the
  frontmatter-present hard-error scan, side-table of parsed
  `Frontmatter` per template name, and updated `description()`/display
  name resolution as described above.
- `crates/tfmt/src/commands/list_templates.rs`: `format_template` prints
  each declared arg's name, type, required/default, and description.
- `README.md`: replace the `## TODO` entry ("Add frontmatter to script
  with basic types.") with documentation of the frontmatter format,
  alongside the existing Templates/Safety/Sanitization sections.
- `examples/stef.tfmt`: updated to demonstrate frontmatter with a
  realistic `path`-typed prefix argument, replacing the current freeform
  comment.

## Testing plan

- `crates/core`: unit tests for `Frontmatter`/`ArgSpec` parsing (valid
  TOML, missing fields, unknown `type`, duplicate arg names) and for arg
  resolution (required-missing, extra-positional, int parse failure, path
  segment sanitization + trailing-separator normalization, string
  sanitization).
- `crates/fs`: unit tests in `template.rs` for frontmatter-block
  stripping (present/absent/malformed-unterminated block), both
  deprecation-warning paths, and the frontmatter-present hard-error path
  for `args[N]`.
- CLI integration fixtures (`tests/fixtures/cli/cases/`): a new fixture
  case using a frontmatter-declared script (rename success path) and one
  exercising a validation failure (missing required argument), following
  existing fixture conventions.
- `list-templates` output test/fixture update to cover the new per-arg
  display lines.
