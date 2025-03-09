# tfmttools

Use `minijinja` to rename audio files according to their tags.

## Installation

1. Ensure `cargo` and Cargo's `bin` folder are on your `PATH`.
1. Ensure you have a version of Rust matching the MSRV described in Cargo.toml.
1. Clone the repository.
1. Run `cargo install --path tfmttools`.

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

## TODO

- TODO? Explicitly handle exit codes?
- TODO? Do test with interactive app?
- TODO? Actually check the moved files is the same file in tests?
- TODO? Some sort of transactions mechanism?
- TODO? Fix/warn tag with leading/trailing whitespace?
- TODO? Add year to singles?
- TODO? Replace custom history format with SQLite?
    ~~This doesn't want to work, because the current rust implementation requires getting references to Records, whereas you can't return a reference to data returned from SQLite (it's owned).~~

    This is now fixed
- ~~TODO Add more description when running `rename` under `--yes`~~
- ~~TODO Handle undone records superseded by new actions.~~
