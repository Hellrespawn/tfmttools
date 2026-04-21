# tfmttools

Use `minijinja` to rename audio files according to their tags.

## Installation

### Release Archives

Download a release archive from GitHub or Forgejo, extract it, and place the
`tfmt` binary on your `PATH`.

Windows archives contain `tfmt.exe`. Linux archives contain `tfmt`.
Release archives include the `examples/` directory with starter templates.

Verify the installed binary:

```sh
tfmt --version
```

### From Source

1. Ensure `cargo` and Cargo's `bin` folder are on your `PATH`.
1. Ensure you have a version of Rust matching the MSRV described in
   `Cargo.toml`.
1. Clone the repository.
1. Run `cargo install --path crates/cli`.
1. Run `tfmt --version`.

## Workspace Map

`tfmt` is a CLI for renaming audio files from their tags using
`minijinja` templates.

The workspace is split by responsibility:

- `crates/cli/` builds the `tfmt` binary and owns argument parsing, commands,
  terminal interaction, and integration test wiring.
- `crates/core/` contains the main rename logic, template rendering, and tag
  processing.
- `crates/fs/` applies rename plans to the filesystem and provides related file
  handling helpers.
- `crates/history/` contains the history data model and the concrete
  serde-backed history storage used by the CLI.
- `crates/test-harness/` contains shared test utilities used by fixture-backed
  integration tests.

Supporting directories:

- `examples/` contains example templates such as
  `examples/stef.tfmt`.
- `tests/fixtures/cli/` contains CLI integration fixtures, including
  `cases/`, `audio/`, `extra/`, `template/`, and `report/`.
- `docs/` contains project notes and refactor planning.
- `packaging/` contains packaging metadata such as the Arch Linux
  `PKGBUILD`.

## Usage

Write a `minijinja` template and run `tfmt rename` against a directory of audio
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

### Templates

Templates render target paths without the file extension. `tfmt` keeps the
source file extension.

For example, a template can group files by artist and title:

```jinja
{{ artist }}/{{ title }}
```

Literal `/` or `\` characters in the template create directories. Separators
coming from interpolated tag values are sanitized so tags cannot accidentally
create extra directories.

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

Trailing periods are also removed from interpolated tag values.

### History

Every applied run is recorded in the history file under the configuration
directory. Use these commands to inspect and manage history:

```sh
tfmt show-history
tfmt undo
tfmt redo
tfmt clear-history
```

### Windows Notes

Windows support targets normal NTFS usage. Long-path mode, network shares,
unusual Unicode normalization, and cross-volume edge cases are not full
compatibility promises for this release.

<!--
## Miscellaneous

Don't remember what this was about, probably related to the file in question:

> Handle UTF-16 odd length error manually?
>
> Check:
>
> .\The Witcher\2016 - The Witcher 3 Wild Hunt - Blood and Wine\09 - Percival Schuttenbach - The Musty Scent of Fresh Pâté.mp3
-->
