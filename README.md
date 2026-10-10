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

## Usage

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
history session. `show-history` and dry runs use read-only database access. If another command is using the same history, tfmt exits
with an error immediately. Validation fixes acquire the lock before changing
audio files.

The lock is stored in a persistent sibling file: for example, `tfmt.hist`
uses `tfmt.hist.lock`. Leave this file in place; its existence does not mean
that a command is running. The operating system releases the lock when the
session's file handle closes, including when the command exits with an error.
History symlinks use the referent's sibling lock file, so commands accessing
the same history through different links still exclude one another.

### Validate

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

### Templates

Templates render target paths without the file extension. `tfmt` appends the
source extension, including when the rendered filename already contains dots.
Use `.tfmt` files and select them with `--template` (`-t`), or pass the same
document syntax directly with `--script` (`-s`):

```sh
tfmt --dry-run rename --script 'path: ({$artist} / {$title})'
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
path: ({prefix} {$artist} / {$title} {suffix})
```

Arguments are bare names; tags start with `$`. Adjacent expressions concatenate,
with all literal text in double quotes. Whitespace outside strings is ignored.
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
| Tag interpolation | `{$title}` | Insert a prepared tag value. |
| Argument interpolation | `{suffix}` | Insert a declared argument. |
| Fallback | `{$album_artist ?? $artist ?? "Unknown"}` | Select the first present value. |
| Positive guard | `[$album? {$album} /]` | Include content when present. |
| Negative guard | `[!$album? "Singles" /]` | Include content when missing or empty. |
| Year extraction | `{$date \| year}` | Extract a four-digit year. |
| Number padding | `{$track_number \| pad(2)}` | Pad displayed text to a minimum width. |

Missing or empty values are absent; numeric zero is present. Guards and
fallbacks evaluate only selected content. Nest guards for combined presence
conditions. `year` checks ISO-style dates first, then day/month/year-style
dates, then any four-digit year. Invalid present dates are errors. Padding
accepts widths from 0 through 1024 and never truncates text.

A bare `/` separates components using the host platform's native separator.
Quoted path literals cannot contain `/` or `\`; tag separators are sanitized.
Only a bare `/` as the first top-level expression requests the platform root;
empty initial directories, repeated separators, and empty final filenames
are errors. Windows drive and UNC prefixes are outside the initial grammar.

### Arguments

Declare arguments in CLI positional order with `arg name: type`. Types are
`string`, `int`, and `path`. Arguments without `default` are required;
`default: ""` makes a string or path optional. Supplied empty values override
defaults. Integer arguments must fit a signed 64-bit integer; empty integers
are invalid. Excess arguments are errors, including templates with no declarations.

String and path arguments are validated rather than sanitized. They reject
the characters in the sanitization table below; `/` and `\` are allowed as
structural separators in path arguments. Argument text is preserved without
trimming or replacement. All defaults are validated during compilation, and
all supplied values are checked before reading audio, including unused ones.
Accepted whitespace still undergoes final filename validation.

A path argument splits on both `/` and `\`, discards empty segments, and inserts
complete directory components. Leading separators do not make it absolute.
Thus `{prefix} {$artist}` needs no `/` after `{prefix}`. A path argument must
occur at a component boundary; inserting one after unfinished text or following
one immediately with `/` is an error.

See [the language reference](crates/picotmpl/README.md) and the full
[Stef layout](examples/stef.tfmt).

### Migrating existing templates

MiniJinja templates and TOML frontmatter are no longer supported. Migration
is manual; there is no converter or compatibility mode.

| Old syntax | New syntax |
| --- | --- |
| `{{ artist }}/{{ title }}` | `path: ({$artist} / {$title})` |
| `{{ albumartist or artist }}` | `{$album_artist ?? $artist}` |
| `{% if album %}...{% endif %}` | `[$album? ...]` |
| `{{ tracknumber \| zero_pad(2) }}` | `{$track_number \| pad(2)}` |
| Frontmatter `name = "Layout"` | `name: "Layout"` |
| Positional `args[0]` | Declare an argument, then interpolate its bare name. |

Use first-class argument declarations instead of frontmatter `args` entries.
Remove `required` settings: omit `default` to require an argument, or specify
`default: ""` to make it optional. Arguments previously sanitized may now be
rejected and must be corrected by the caller. Move leading description
comments to `description: "..."`.

Guards and fallbacks now retain zero values. The migrated Stef example also
omits the filename artist prefix when the track artist is missing or empty:
the old `albumartist and artist ~ " - "` expression could produce a dangling
`" - "`; the new nested guards require both tags to be present. General Jinja
comparisons, loops, includes, and arbitrary functions are unsupported.

Saved filename references work after the referenced files are migrated.
Reusing a saved Jinja inline template for a new rename fails with a migration
hint; select an explicit replacement using `--script` or `--template`.
Undo and redo replay actions stored in supported SQLite histories without
parsing templates. Old JSON histories are rejected without importing them.

### Safety

`tfmt` validates the full rename plan before it moves files. It rejects:

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

Interpolated tag values are sanitized before rendering final paths:

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

Trailing periods are also removed from interpolated tag values. Rendering
trims surrounding whitespace and reports a warning; `validate characters --fix`
preserves surrounding whitespace when applying the shared sanitization rules.

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
SQLite uses STRICT tables, enforced foreign keys, rollback journaling, and
`synchronous=EXTRA`. Intent and tag patches are saved before each action; its
successful completion is committed before the next action starts. Database
commits and filesystem changes remain separate operations.

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
reports its confirmed actions, without executing file operations. Completed
version-1 databases migrate on a writable invocation; pending version-1 work
must be resolved with the compatible tfmt version first. Read-only commands ask
for a writable migration when necessary. Invalid databases remain errors;
manual resolution does not repair database corruption.

File actions require regular files. Move destinations must be absent or a
verified case-only alias of the source; distinct hard links are rejected.
Copies are verified and synced before associated source deletion. The session
lock excludes cooperating writers but cannot prevent arbitrary external
programs from changing files. Dry runs write neither history nor audio.

The generated [version 2 SQL schema](docs/history/schema-v2.sql) describes the
SQLite contract. Regenerate it with `cargo xtask history-schema`; core tests
compare it against the checked-in snapshot. Published database format changes
require a version bump, migration, updated schema, and compatibility/replay tests.
Stored template text remains unchanged; reuse restrictions still apply.

### Windows Notes

Windows support is best-effort. Windows binaries are not release artifacts for
this release, and Windows-specific filesystem behavior is not a compatibility
promise.
