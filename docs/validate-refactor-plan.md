# Validate Command Refactor Plan

## Goal

Refactor `tfmt validate` from the old nested command shape:

```text
tfmt validate check
tfmt validate fix characters
tfmt validate fix id3-encoding
```

to a validation-type first shape:

```text
tfmt validate
tfmt validate characters
tfmt validate characters --fix
tfmt validate id3-encoding
tfmt validate id3-encoding --fix
```

This is a clean break. Do not keep compatibility aliases for
`validate check` or `validate fix <type>`.

## Target Behavior

### `tfmt validate`

Runs all validation checks in read-only mode.

- Runs the `characters` check.
- Runs the `id3-encoding` check.
- Does not modify files.
- Exits with a non-zero status if any check finds issues.
- Rejects `tfmt validate --fix`, because `--fix` requires an explicit
  validation type.

### `tfmt validate characters`

Checks tag values for characters that may not work well in filenames.

This should use friendlier wording than the current "forbidden
characters" / "validation failed" language. These characters are not
necessarily hard errors; fixing them is optional and strips or replaces
characters in tag values.

Suggested output direction:

```text
Some tag values contain characters that may not work well in filenames.
Run `tfmt validate characters --fix` to strip or replace them.
This changes file tags.
```

The command should remain read-only and exit with a non-zero status when
issues are found so it is still useful in automation.

### `tfmt validate characters --fix`

Applies the existing tag value cleanup behavior.

- Strips or replaces the same characters currently handled by
  `validate fix characters`.
- Modifies file tags.
- Stores undoable history.
- Uses history metadata matching the new command shape, for example
  `validate characters --fix`.

### `tfmt validate id3-encoding`

Checks MP3 ID3 text frames for non-ASCII text stored as not UTF-16.

Report frames where both are true:

- The text value contains at least one non-ASCII character, such as `ß`.
- The ID3 text frame is not encoded as UTF-16.

Suggested output direction:

```text
Some ID3 text frames contain non-ASCII characters but are not encoded as UTF-16.
UTF-16 is recommended for compatibility.
Run `tfmt validate id3-encoding --fix` to rewrite matching frames as UTF-16.
This changes file tags.
```

This check should not warn about ASCII-only fields, and it should
not warn about fields already encoded as UTF-16.

### `tfmt validate id3-encoding --fix`

Rewrites only the matching ID3 text frames to UTF-16.

- Uses the same predicate as the read-only check:
  non-ASCII text encoded as not UTF-16.
- Preserves the tag text value.
- Changes the frame encoding to UTF-16.
- Does not touch ASCII-only fields.
- Does not touch fields already encoded as UTF-16.
- Stores undoable history.
- Uses history metadata matching the new command shape, for example
  `validate id3-encoding --fix`.

Remove the `--encoding` option. The command should always rewrite
matching frames to UTF-16 and should no longer support alternate target
encodings.

## Implementation Steps

1. Update the clap model in
   `crates/tfmt/src/cli/args_definition.rs`.

   Replace `ValidateSubcommand` and `ValidateFixSubcommand` with a single
   validation type enum:

   ```rust
   pub enum ValidateType {
       Characters,
       #[command(name = "id3-encoding")]
       Id3Encoding,
   }
   ```

   Make the validation type optional on `ValidateArgs` so bare
   `tfmt validate` can run every check:

   ```rust
   pub command: Option<ValidateType>
   ```

   Add a global `--fix` flag on `ValidateArgs` so it works after the
   validation type:

   ```rust
   #[arg(long, global = true)]
   pub fix: bool
   ```

2. Update CLI exports in `crates/tfmt/src/cli/mod.rs`.

   Remove exports for the deleted fix args and subcommand types. Export
   the new validation type if command dispatch needs it.

3. Update command dispatch in `crates/tfmt/src/commands/validate.rs`.

   Dispatch by `(validate_args.command, validate_args.fix)`:

   - `(None, false)` runs all checks.
   - `(None, true)` returns a user-facing error because `--fix` needs a
     validation type.
   - `(Some(Characters), false)` runs only the character check.
   - `(Some(Characters), true)` applies character fixes.
   - `(Some(Id3Encoding), false)` runs only the ID3 encoding check.
   - `(Some(Id3Encoding), true)` applies ID3 encoding fixes.

4. Rename and soften the character validation reporting.

   The existing implementation can mostly stay in place, but rename
   internal types where helpful so the code does not keep treating
   optional cleanup as hard errors. For example:

   - `TagValueError` -> `TagValueIssue`
   - "Forbidden characters in tag values" -> friendlier user-facing text

5. Replace the current ID3 encoding fix model.

   Remove the generic target encoding path:

   - `FixId3EncodingArgs`
   - `TargetEncoding`
   - `encoding_problems_for`
   - `--encoding`
   - Latin1 mojibake repair behavior

   Add a shared predicate for both check and fix:

   ```rust
   fn should_rewrite_id3_text_as_utf16(
       value: &str,
       encoding: TextEncoding,
   ) -> bool {
       encoding == TextEncoding::UTF8 && !value.is_ascii()
   }
   ```

   The fix path should create tag value changes that preserve the text
   value and set the new encoding to `UTF16`.

6. Add a read-only ID3 encoding result/reporting path.

   It should report the file path, tag key or frame identifier, current
   encoding, and value for each matching frame.

7. Update CLI fixture cases.

   Existing commands should change as follows:

   ```text
   --simple validate -i input check
   ```

   becomes:

   ```text
   --simple validate -i input characters
   ```

   ```text
   --simple --yes validate -i input fix characters
   ```

   becomes:

   ```text
   --simple --yes validate -i input characters --fix
   ```

   ```text
   --simple --yes validate -i input fix id3-encoding
   ```

   becomes:

   ```text
   --simple --yes validate -i input id3-encoding --fix
   ```

   Add a read-only `id3-encoding` fixture step that expects exit code `1`
   before the fix. The existing Lindemann fixture is a good fit because
   `Ich Weiß Es Nicht` contains `ß` and starts with UTF-8 tag encoding.

8. Update documentation.

   Update at least:

   - `README.md`
   - `CHANGELOG.md`
   - relevant fixture docs under `tests/fixtures/cli/`
   - `docs/TODO.md` if the encoding TODO is now obsolete

   The README should clearly state that:

   - `tfmt validate` is read-only.
   - `tfmt validate characters --fix` changes file tags.
   - Fixing character issues is optional.
   - `tfmt validate id3-encoding --fix` rewrites matching non-ASCII
     ID3 text frames as UTF-16.

## Verification

Run the fast CLI check first:

```text
cargo xtask test-cli
```

Then run the broader test suite:

```text
cargo xtask test
```

Finish with the formatting and lint gate:

```text
cargo xtask lint
```
