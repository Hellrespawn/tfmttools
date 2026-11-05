# tfmt

Rename audio files according to their tags using small path templates.

## Installation

### Release Archives

Download a release archive from Forgejo, extract it, and place the
`tfmt` binary on your `PATH`.

Linux archives contain `tfmt`, shell completions, man pages, and the
`examples/` directory with starter templates.

Verify the installed binary:

```sh
tfmt --version
```

### From Source

1. Ensure `cargo` and Cargo's `bin` folder are on your `PATH`.
1. Ensure you have a version of Rust matching the MSRV described in
   `Cargo.toml`.
1. Clone the repository.
1. Run `cargo install --path crates/tfmt`.
1. Run `tfmt --version`.

## Workspace Map

`tfmt` is a CLI for renaming audio files from their tags using
path templates.

The workspace is split by responsibility:

- `crates/tfmt/` builds the `tfmt` binary and owns argument parsing, commands,
  terminal interaction, and integration test wiring.
- `crates/core/` contains the main rename logic, template rendering, and tag
  processing.
- `crates/fs/` applies rename plans to the filesystem and provides related file
  handling helpers.
- `crates/core/src/history/` contains the history data model and the concrete
  SQLite history storage and interruption reporting used by the CLI.
- [crates/picotmpl/](crates/picotmpl/README.md) implements the new
  path template language as an independent library. Its example is
  `examples/stef.tfmt`; the CLI uses this library for all templates.
- `crates/test-harness/` contains shared test utilities used by fixture-backed
  integration tests.

All workspace members inherit the Clippy policy from
`[workspace.lints.clippy]` in the root `Cargo.toml`. Keep targeted lint
exceptions next to the code they apply to.

Supporting directories:

- `examples/` contains example templates such as
  `examples/stef.tfmt`.
- `tests/fixtures/cli/` contains CLI integration fixtures, including
  `cases/`, `audio/`, `extra/`, `template/`, and `report/`.
- `docs/` contains project notes and refactor planning.
- `packaging/` contains packaging metadata such as the Arch Linux
  `PKGBUILD`.

To inspect a template's AST during development, run
`cargo xtask dump-ast examples/stef.tfmt`.

## Usage

Cleanup previews describe proposed deletions; dry runs say “Would delete”.
A writable rename reports completion only after filesystem execution and history
confirmation succeed. Nonempty directories may remain after cleanup.

Audio discovery errors abort rename and validation before applying file changes.
An incomplete directory scan is never treated as a complete selection.
Rename and validation accept `--recursion-depth` from 0 through 256; the default
is 4. This is an application limit, independent of filesystem path-length limits.
Operational errors include their underlying causes.

Commands return status 0 on success and 1 on validation findings or operational
failure. Validation findings print their report without an operational-error
banner. Normal returns allow history and logging guards to close before exit.


Write a path template and run `tfmt rename` against a directory of audio
files.

Always inspect a large rename plan with `--dry-run` first:

```sh
tfmt --dry-run rename -t examples/stef.tfmt
```

For non-interactive use, add `--yes` or `--no-confirm`:

```sh
tfmt --simple --yes rename -t examples/stef.tfmt
```

See also the "examples"-folder.

### History locking

Commands that change history hold an exclusive lock for the whole
history session. `show-history` and dry runs use read-only database access.
Their records and interruption reports describe one snapshot captured when
history is loaded.
If another command is using the same history, a writable command exits
with an error immediately. Validation fixes acquire the lock before changing
audio files.

The lock is stored in a persistent sibling file: for example, `tfmt.hist`
uses `tfmt.hist.lock`. Leave this file in place; its existence does not mean
that a command is running. The operating system releases the lock when the
session's file handle closes, including when the command exits with an error.
History symlinks use the referent's sibling lock file, so commands accessing
the same history through different links still exclude one another.

### Validate

Rename, validation fixes, undo, and redo report existing interrupted history
once when the command starts. Execution still checks unresolved attempts before
changing files; newly failed actions produce their own failure report.

`tfmt validate` checks audio files and tags without modifying files. It runs
all validation checks and exits with a non-zero status when issues are found.

Run a single check by validation type:

```sh
tfmt validate characters
tfmt validate id3-encoding
```

`tfmt validate characters` reports tag values containing characters that may
not work well in filenames. Fixing these issues is optional. To strip or
replace the same characters used by template sanitization, run:

```sh
tfmt validate characters --fix
```

`tfmt validate id3-encoding` reports MP3 ID3 text frames with non-ASCII text
stored as UTF-8. To rewrite matching frames as UTF-16 while preserving their
text values, run:

```sh
tfmt validate id3-encoding --fix
```

Both `--fix` commands change file tags and record undoable history.
They inspect the selected files before applying changes. If any required audio
inspection fails, they report the errors and apply no tag fixes. Dry runs use the
same inspection policy. An execution failure after inspection can still leave
a confirmed partial run, as described above.

### Templates

Templates render target paths without the file extension. `tfmt` appends the
source extension, including when the rendered filename already contains dots.
Use `.tfmt` files and select them with `--template` (`-t`), or pass the same
document syntax directly with `--script` (`-s`):

```sh
tfmt --dry-run rename --script 'template: ($artist / $title)'
```

Template discovery still includes `.jinja` and `.j2` files, but their contents must
also use the new syntax. Lookup uses the filename stem. `name` and
`description` control the metadata shown by `tfmt list-templates`. Renaming
compiles only the selected template, so invalid unrelated templates do not
block it. If multiple files share a lookup stem, specify an explicit file
path to resolve the ambiguity.

```text
name: "Artist and title"
description: "Place tracks under an optional directory prefix."
arg prefix: path(default: "", description: "Directory prefix.")
arg suffix: string(default: "")
template: (prefix $artist / $title suffix)
```

Arguments are bare names; tags start with `$`. Adjacent expressions concatenate,
with all literal text in double quotes. Whitespace outside strings is ignored.
The `template:` definition contains the output expression; `path` is an
argument type for directory prefixes.
`#` starts a comment. Strings support escaped quotes (`\"`) and backslashes
(`\\`); other escapes are rejected. Audio tag names use exact lowercase
snake_case, such as `album_artist` and
`track_number`. The explicit aliases are `album`, `artist`, `title`,
`album_sort`, and `disk_number`; `date` is a computed fallback. Argument names
remain case insensitive. Unknown tags are errors,
even in skipped guards. Missing recognized tags are allowed. Templates using
compact, uppercase, or hyphenated tag names must be migrated. New tag-fix
history records use canonical names. Old JSON histories are unsupported and
are never imported; move them aside explicitly to start a new SQLite history.

| Construct | Example | Meaning |
| --- | --- | --- |
| Tag interpolation | `$title` | Insert a prepared tag value. |
| Argument interpolation | `suffix` | Insert a declared argument. |
| Elvis fallback | `$album_artist ?: $artist ?: "Unknown"` | Select the first present value. |
| Positive guard | `$album ? ($album /)` | Include content when present. |
| Negated condition | `!$album ? ("Singles" /)` | Include content when missing or empty. |
| Alternative | `$album ? "Album" : "Single"` | Select one branch. |
| Year extraction | `$date \| year` | Extract a four-digit year. |
| Number padding | `$track_number \| pad(2)` | Pad displayed text to a minimum width. |

Missing or empty values are absent; numeric zero is present. Elvis evaluates
its left operand once, and conditionals evaluate only the selected branch.
Each conditional branch is one expression: group multiple output segments
with parentheses. A space before `?` is conventional, but optional.
Unary `!` produces a boolean for use in conditions;
booleans cannot be emitted, concatenated, or formatted, even in unreachable
branches. Boolean conditions use their truth value.

Binding from strongest to weakest is `!`, formatting `|`, Elvis `?:`,
conditional `?` with optional `:`, then concatenation. Parentheses override
this order. Thus `$a ?: $b | pad(2)` pads only `$b`, while
`($a ?: $b) | pad(2)` pads the selected value. A literal can keep a group
present: `("By " $artist) ?: "Unknown"` emits `"By "` when artist is absent.
Nested conditionals bind to the right, and `:` belongs to the nearest
unmatched `?` in its group. Elvis `?:` cannot contain whitespace.

`year` checks ISO-style dates first, then day/month/year-style dates, then
any four-digit year. Invalid present dates are errors. Padding accepts
widths from 0 through 1024 and never truncates text. Formatters cannot
accept an expression that could return structural path components.

A bare `/` separates components using the host platform's native separator.
Quoted `/` and `\` are component text, subject to the filename character policy.
Only a bare `/` as the first top-level expression requests the platform root;
empty initial directories, repeated separators, and empty final filenames
are errors. Windows drive and UNC prefixes are outside the initial grammar.

### Arguments

Declare arguments in CLI positional order with `arg name: type`. Types are
`string`, `int`, and `path`. Arguments without `default` are required;
`default: ""` makes a string or path optional. Supplied empty values override
defaults. Integer arguments must fit a signed 64-bit integer; empty integers
are invalid. Excess arguments are errors, including templates with no declarations.

String arguments preserve their text until interpolation. Path arguments split
into components first; all completed components then follow the same filename
character policy as literals and tags. Defaults are type-checked during
compilation. Prohibited controls are rejected in defaults and supplied values,
including unused arguments, before reading audio.

A path argument splits on both `/` and `\`, discards empty segments, and inserts
complete directory components. Leading separators do not make it absolute.
Thus `prefix $artist` needs no `/` after `prefix`. A path argument must
occur at a component boundary; inserting one after unfinished text or following
one immediately with `/` is an error.

See [the language reference](crates/picotmpl/README.md) and the full
[Stef layout](examples/stef.tfmt).

### Migrating existing templates

MiniJinja templates, TOML frontmatter, and the previous picotmpl delimiters
are no longer supported. Migration is manual; there is no converter or
compatibility mode.

| Old syntax | New syntax |
| --- | --- |
| `path: (...)` | `template: (...)` |
| `{{ artist }}/{{ title }}` | `template: ($artist / $title)` |
| `{{ albumartist or artist }}` | `$album_artist ?: $artist` |
| `{% if album %}...{% endif %}` | `$album ? (...)` |
| `{{ tracknumber \| zero_pad(2) }}` | `$track_number \| pad(2)` |
| `{$title}` or `{prefix}` | `$title` or `prefix` |
| `[$album? {$album} /]` | `$album ? ($album /)` |
| `[!$album? "Singles" /]` | `!$album ? ("Singles" /)` |
| `{$a ?? $b \| pad(2)}` | `($a ?: $b) \| pad(2)` |
| Frontmatter `name = "Layout"` | `name: "Layout"` |
| Positional `args[0]` | Declare an argument, then interpolate its bare name. |

Use first-class argument declarations instead of frontmatter `args` entries.
Remove `required` settings: omit `default` to require an argument, or specify
`default: ""` to make it optional. Move leading description
comments to `description: "..."`.

Guards and fallbacks now retain zero values. The migrated Stef example also
omits the filename artist prefix when the track artist is missing or empty:
the old `albumartist and artist ~ " - "` expression could produce a dangling
`" - "`; the new nested guards require both tags to be present. General Jinja
comparisons, loops, includes, and arbitrary functions are unsupported.

Saved filename references work after the referenced files are migrated.
Reusing a saved legacy inline template for a new rename fails with a migration
hint; select an explicit replacement using `--script` or `--template`.
Undo and redo replay actions stored in supported SQLite histories without
parsing templates. Old JSON histories are rejected without importing them.

### Safety

`tfmt` validates the full rename plan before it moves files. It rejects:

- target files also required as directories by another destination
- target collisions
- targets that differ only by case
- existing target files, except targets that are also sources in the same
  in-situ rename plan
- Windows reserved device names such as `CON`, `NUL`, `COM1`, and `LPT1`
- path components with leading or trailing spaces
- path components with trailing periods
- target paths that exceed the conservative cross-platform path length limit

In-situ renames are supported, including swaps, cycles, chains, and case-only
renames. `tfmt` uses temporary staging paths internally when direct moves would
be unsafe.

Rename operations are not fully transactional. If an unexpected filesystem
error occurs after some files have moved, use `tfmt undo` to revert completed
recorded actions where possible.

Audio read and write errors include the affected path and preserve the
underlying cause in diagnostic error chains.

### Filename Sanitization

After interpolation, the same rules apply to every completed component,
including literals, string arguments, path argument components, and tags.
By default, forbidden characters are replaced or removed. Use
`tfmt rename --strict` to reject components that would require a change,
with the original component shown in the error:

| Character | Replacement |
| --------- | ----------- |
| `<`       | removed     |
| `"`       | removed     |
| `>`       | removed     |
| `:`       | removed     |
| `|`       | removed     |
| `?`       | removed     |
| `*`       | removed     |
| `~`       | `-`         |
| `/`       | `-`         |
| `\`       | `-`         |

Leading spaces and trailing spaces or periods are removed from completed
components. Interior spaces and periods remain, including those originating
at the edges of interpolated values. Tag whitespace still produces a metadata
warning. `validate characters --fix` uses these same replacement rules on
whole tag values.

In both modes, Unicode control characters (including NUL, tabs, and newlines),
line/paragraph separators U+2028/U+2029, and bidirectional formatting controls
U+061C, U+200E/U+200F, U+202A–U+202E, and U+2066–U+2069 are always rejected.
Combining marks, joiners, and variation selectors remain allowed. Controls in
referenced tags are checked before numeric parsing or formatting can hide them.

Replacement must leave nonempty components. Reserved names, collisions,
existing targets, and path length are checked on the resulting paths before
files move. Slashes in text never create directories; only template separator
syntax and path arguments create component boundaries.

### History

Every applied run is recorded in the history file under the configuration
directory. Use these commands to inspect and manage history:

```sh
tfmt show-history
tfmt undo
tfmt redo
tfmt clear-history
```

History uses a versioned SQLite database at `<configuration directory>/tfmt.hist`.
Old JSON histories are rejected without importing or rewriting them. Move an old
history aside explicitly if you want to start a fresh database; it cannot supply
exact-byte undo/redo patches. Invalid databases, foreign application identifiers,
and unsupported schema versions stop commands before applying new actions.

Records preserve IDs, ordering, timestamps, template metadata, and replay states.
Record IDs define chronological order. Each record explicitly stores its applied
count, completeness, and redo eligibility; these values are also required in
serialized record data. Old JSON record defaults are no longer supported.
SQLite uses STRICT tables, enforced foreign keys, rollback journaling, and
`synchronous=EXTRA`. Intent and tag patches are saved before each action; its
successful completion is committed before the next action starts. Database
commits and filesystem changes remain separate operations.
Per-action validation uses the loaded history cache, and confirmations update
it only after commit, without rereading the growing action list for every file.

Tag fixes edit and verify a candidate beside the audio file. tfmt moves the
original to a temporary backup, installs the candidate, records completion, then
removes the backup. This uses no additional backup copy. The portable switch
briefly leaves the original pathname absent. Audio symlinks retain their links
while the target is edited; hard-linked audio files are rejected. Audio
replacement currently requires Unix hard-link checks, and preserves permissions
but does not promise to preserve ACLs or extended attributes.

Tag-edit undo/redo checks the expected complete file bytes and applies the
recorded patch to a candidate. It restores exact recorded bytes, including tag
layout and encodings. If the file has changed since the operation, replay fails
without overwriting it. New files and restored candidates are verified before
installation; raw tag serialization is never used for undo/redo.

Commands stop on the first error. Confirmed actions stay in history and can be
undone, including the applied prefix of a partially undone record. Partial runs
and records affected by failed or interrupted replay cannot be redone. Rename
and cleanup actions are recorded in order, including staging moves and
copy/remove steps. Cleanup confirmation happens before file changes, and the
configuration and bin directories are protected from cleanup.

Cleanup preserves relative paths under `trash/<run ID>/`, so files such as
`a/b.jpg` and `a_b.jpg` remain distinct. Cleanup destinations are validated
before any files move. Protected directory checks resolve relative paths and
symlink aliases.

Moves and removals check file metadata without reading file contents. Copies
and tag replacement still verify file bytes; their file hashes use a fixed-size
buffer rather than loading the whole file into memory. Tag editing and binary
patch creation still require whole-file buffers.

An error after an action starts, a crash, or power loss can leave that action's
completion unconfirmed. tfmt reports the last attempted action, affected paths,
and what history confirms. It never resumes, reverses, or infers the outcome of
an interrupted action. Check and repair the files manually, then explicitly
record whether the attempted action happened:

```sh
tfmt resolve-history --attempt 12 --outcome applied
# Or, after restoring the state before the attempted action:
tfmt resolve-history --attempt 12 --outcome not-applied
```

Use the same configuration directory as the interrupted command. Resolution
updates history only; it does not inspect or change files or remove artifacts.
For a partial copy/delete operation, finish the operation or restore its original
state manually before choosing an outcome. A surviving tag backup is retained
until completion is confirmed; inspect the reported candidate and backup paths
before removing anything. Interruption during backup cleanup can leave an orphan
backup after the action is already confirmed. Remove that backup manually;
tfmt does not scan for or automatically delete orphan artifacts.

Unresolved attempts block new file mutations, undo, redo, and clear-history.
`show-history` and dry runs can report unresolved history without changing it.
A writable invocation closes a marker-free interrupted run as partial and
reports its confirmed actions, without executing file operations. Invalid
databases remain errors; manual resolution does not repair database corruption.

File actions require regular files. Move destinations must be absent or a
verified case-only alias of the source; distinct hard links are rejected.
Copies are verified and synced before associated source deletion. The session
lock excludes cooperating writers but cannot prevent arbitrary external
programs from changing files. Dry runs write neither history nor audio.

The generated [version 1 SQL schema](docs/history/schema-v1.sql) describes the
SQLite contract. Regenerate it with `cargo xtask history-schema`; core tests
compare it against the checked-in snapshot. The initial SQLite schema is still
undeployed and has been simplified; databases from earlier development revisions
are rejected without rewriting them. Move those aside explicitly to start fresh.
Published database format changes
require a version bump, migration, updated schema, and compatibility/replay tests.
Stored template text remains unchanged; reuse restrictions still apply.

### Windows Notes

Windows support is best-effort. Windows binaries are not release artifacts for
this release, and Windows-specific filesystem behavior is not a compatibility
promise.

Template syntax is parsed with pest. Diagnostics identify the source location
and expected language constructs; template syntax is unchanged.

## Development checks

`cargo xtask test` runs the workspace tests, and `cargo xtask test-cli` runs
all CLI tests. Cargo discovers conventional integration targets automatically.
Use `cargo xtask test-integration` for the focused fixture harness.
