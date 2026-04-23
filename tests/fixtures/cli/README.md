# CLI Fixture Harness

These fixtures drive the `tfmttools-cli` integration test harness.

## Directory Roles

- `cases/`: JSON test cases discovered by the integration harness.
- `template/`: templates copied into each test config directory.
- `audio/`: audio files copied into each test input directory.
- `extra/`: non-audio files copied into `input/extra/`.
- Reports are generated under `tests/reports/`.

## Case File Schema

Case files live in `cases/` and are discovered when their file name ends
with `.case.json`. Unknown top-level fields and unknown test step fields
are rejected by the harness.

Each case has this top-level shape:

```json
{
  "description": "Single apply, undo and redo of simple_input.tfmt",
  "expectations": {
    "initial-state": [
      { "path": "input/Nightwish - Nemo.mp3", "checksum": "820F92DA" }
    ],
    "apply": [
      { "path": "Nightwish/Nemo.mp3", "checksum": "820F92DA" }
    ]
  },
  "tests": {
    "initial": {
      "expectations": "initial-state"
    },
    "apply": {
      "command": "--simple --yes rename -t simple_input",
      "expectations": "apply",
      "previous-expectations": "initial-state"
    }
  }
}
```

- `description`: human-readable case summary.
- `expectations`: named sets of paths and optional checksums.
- `tests`: ordered test steps.

Expectation entries use paths relative to the temporary work directory.
If `checksum` is present, it must match the file checksum calculated by
the harness. If `checksum` is omitted, only file presence is checked.

Use `options: ["no-previous"]` on an expectation entry when a later
`previous-expectations` check should allow that path to remain:

```json
{ "path": "input/extra/cover.jpg", "options": ["no-previous"] }
```

Test steps support these fields:

- `command`: CLI command appended after harness-provided global options.
- `expectations`: expectation set that should exist after the step.
- `previous-expectations`: expectation set whose paths should no longer
  exist unless individual entries use `options: ["no-previous"]`.

All test step fields are optional. A step without `command` can verify
the initial fixture state. Commands are split on whitespace before they
are passed to the CLI.

## Harness Behavior

Every case starts from a fresh temporary work directory. Before each
case, the harness copies:

- `template/*` into `config/`.
- `audio/*` into `input/`.
- `extra/*` into `input/extra/`.

When a command runs, the harness invokes the `tfmt` binary from the
temporary work directory and injects:

```text
--config-directory <temp-work-dir>/config --run-id run_id
```

The command from the test step is appended after those global options.

Test steps run in the order written in the JSON object. The harness
stops a case after the first failing step. Failing work directories are
preserved for inspection; passing work directories are temporary.

After the integration run, the harness generates runner-specific
timestamped report files under `tests/reports/`, for example
`cli-2026-04-23T14-03-44.123Z.html` and
`cli-2026-04-23T14-03-44.123Z.json`. The HTML viewer is owned by the
harness crate and loads its adjacent JSON report.

## Adding A Case

- Name the file after the behavior, such as
  `case_only_rename.case.json`.
- Put matching templates in `template/` when the case needs them.
- Prefer the smallest existing audio or extra fixture that proves the
  behavior.
- Include checksums when file identity matters, such as rename, copy,
  undo, or redo behavior.
- Use `previous-expectations` when a step should prove that paths from a
  prior state were removed.
- Use `options: ["no-previous"]` for paths that may legitimately remain
  between states.

Run the fixture-backed suite with:

```sh
cargo test -p tfmttools-cli --test integration -- --nocapture
```

## Audio-file explanation

All audio-files are of silence, with tags copied to them for testing.

### Justification

- "Amon Amarth - Under Siege": Uses disc-number
- "Damjan Mravunac - Welcome To Heaven.ogg": Non-MP3 file
- "Die Antwoord - Gucci Coochie (feat. Dita Von Teese).mp3": Bug with periods in tag values
- "MASTER BOOT RECORD - Dune.mp3": Has no album or year
- "MASTER BOOT RECORD - MYTH.NFO.mp3": Complex album name, many forbidden characters.
- "MASTER BOOT RECORD - RAMDRIVE.SYS.mp3": Checking folder structure with same artist
- "MASTER BOOT RECORD - SET MIDI=SYNTH1 MAPG MODE1.mp3": Complex song and album name, many forbidden characters
- "Nightwish - Elvenpath (Live).mp3": Random initial pick
- "Nightwish - Nemo.mp3": Random initial pick
- "Nightwish - While Your Lips Are Still Red.mp3": Random initial pick
