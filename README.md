# tfmttools

Use `minijinja` to rename audio files according to their tags.

## Requirements

- Rust (MSRV: 1.80.1)

## Installation

1. Ensure `cargo` and Cargo's `bin` folder are on your `PATH`.
1. Clone the repository.
1. Run `cargo install --path tfmttools`.

## Usage

Write a `minijinja` template.

See also the "examples"-folder.

## TODO

- TODO Handle exit codes properly
- TODO Check if leftovers are images and offer to delete.

- TODO? Add year to singles?
- TODO? Update tag with leading/trailing whitespace?

- TODO? Add explicit version to history format?

- TODO? Testcase inheritance?

- Handle UTF-16 odd length error manually?

Check:

- .\The Witcher\2016 - The Witcher 3 Wild Hunt - Blood and Wine\09 - Percival Schuttenbach - The Musty Scent of Fresh Pâté.mp3
