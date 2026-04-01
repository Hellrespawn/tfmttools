# tfmttools

Use `minijinja` to rename audio files according to their tags.

## Installation

1. Ensure `cargo` and Cargo's `bin` folder are on your `PATH`.
1. Ensure you have a version of Rust matching the MSRV described in Cargo.toml.
1. Clone the repository.
1. Run `cargo install --path crates/cli`.

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
- `crates/history-core/` contains the history data model and the concrete
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

Write a `minijinja` template.

See also the "examples"-folder.

<!--
## Miscellaneous

Don't remember what this was about, probably related to the file in question:

> Handle UTF-16 odd length error manually?
>
> Check:
>
> .\The Witcher\2016 - The Witcher 3 Wild Hunt - Blood and Wine\09 - Percival Schuttenbach - The Musty Scent of Fresh Pâté.mp3
-->
