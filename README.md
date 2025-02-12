# tfmttools

Use `minijinja` to rename audio files according to their tags.

## Requirements

- Rust (MSRV: 1.83)

## Installation

1. Ensure `cargo` and Cargo's `bin` folder are on your `PATH`.
1. Clone the repository.
1. Run `cargo install --path tfmttools`.

## Usage

Write a `minijinja` template.

See also the "examples"-folder.

## TODO

- Handle extra long names
- Do proper check for trailing period in folder / file name
- Add year to singles?
- Handle UTF-16 odd length error manually?
- Remember last used template, use it in the future when no template specified.

Check:

- .\The Witcher\2016 - The Witcher 3 Wild Hunt - Blood and Wine\09 - Percival Schuttenbach - The Musty Scent of Fresh Pâté.mp3
