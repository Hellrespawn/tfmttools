# Warning Collection and Reporting

**Date:** 2026-07-08

## Problem

Deprecation warnings (in `tfmttools-fs`) and whitespace-in-tag warnings (in `tfmttools-core`) are currently emitted via `eprintln!` directly inside library crates. This violates the separation of concerns — library crates should not own user-facing output. It also makes aggregation impossible: processing 500 files can produce 500 identical warnings with no way to collapse or summarise them.

## Goal

- Library crates collect typed warning values; they never print.
- The CLI (`tfmt`) owns all formatting and output decisions.
- Warnings are surfaced at natural checkpoints (the rename preview step, the template list) so users can react before committing to an operation.
- Identical warnings across many files are collapsed into a single counted line.

## Warning Type

A typed `Warning` enum lives in `tfmttools-core`, since both `core` and `fs` need to emit warnings and `fs` already depends on `core`.

```rust
pub enum Warning {
    WhitespaceInTag { file: String, tag: ItemKey },
    DeprecatedPositionalArgs { template: String },
    DeprecatedLeadingComment { template: String },
}
```

Typed variants (rather than strings) are what make aggregation work: grouping 312 `WhitespaceInTag { tag: ItemKey::Artist }` entries into one line requires structured data.

## Collection in Library Crates

### `AudioFileContext` (`tfmttools-core`)

`AudioFileContext` implements minijinja's `Object` trait, whose `get_value` method has a fixed signature (`self: &Arc<Self>`) that cannot return warnings alongside the value. Warnings are therefore accumulated internally and drained by the caller after rendering.

```rust
pub struct AudioFileContext {
    audio_file: AudioFile,
    resolved_args: ResolvedArgs,
    warnings: RefCell<Vec<Warning>>,
}

impl AudioFileContext {
    pub fn take_warnings(&self) -> Vec<Warning> {
        self.warnings.borrow_mut().drain(..).collect()
    }
}
```

`RefCell` is appropriate here: rendering is single-threaded and `take_warnings` is called exactly once per context, after rendering completes.

The caller pattern:

```rust
let ctx = Arc::new(AudioFileContext::safe(audio_file, resolved_args));
let result = template.render(Value::from_object(ctx.clone()))?;
let warnings = ctx.take_warnings(); // drain once, then Arc drops
```

One `AudioFileContext` corresponds to one file and is not reused. Warnings are drained inside the per-file loop so the `Arc` can drop promptly rather than accumulating across all files.

### `TemplateLoader` (`tfmttools-fs`)

Warnings arise at load time and can be returned directly. `build()` becomes:

```rust
fn build(sources: ...) -> TFMTResult<(Self, Vec<Warning>)>
```

The three public constructors (`read_directory`, `read_filename`, `read_script`) propagate the `Vec<Warning>` to their callers.

## Integration in `tfmt`

### `rename` command

`RenamePlan` carries warnings out of the planning phase:

```rust
pub(crate) struct RenamePlan {
    actions: Vec<RenameAction>,
    unchanged_files: Vec<Utf8File>,
    metadata: ActionRecordMetadata,
    warnings: Vec<Warning>,
}
```

`create_plan` collects from both sources:
- Template warnings from `TemplateLoader` (at template load time).
- File warnings drained from each `AudioFileContext` after each file render (inside `discovery::create_actions_from_template`).

Because templates are loaded before files are rendered, template warnings naturally appear before file warnings with no special ordering logic needed.

`apply::preview` prints the warning report as a separate section after the file list and before the confirmation prompt:

```
Renaming 500 files:
  artist - title.mp3 → Artist/Title.mp3
  ...

Warnings:
  ⚠ 312 files: leading/trailing whitespace in `artist`
  ⚠   5 files: leading/trailing whitespace in `album`
  ⚠ Template 'my_template': uses positional args[N] without frontmatter

Move files? [y/N]
```

Grouping and count aggregation happen in `tfmt` at display time. The warning section is omitted entirely when there are no warnings.

### `list_templates` command

`TemplateLoader::read_directory()` returns `(loader, warnings)`. After printing the template list, `list_templates` prints a flat warning section:

```
Found 3 templates:
  my_template: Does a thing.
  other_template
  foo_template

Warnings:
  ⚠ 'my_template': uses positional args[N] without frontmatter
  ⚠ 'foo_template': uses a leading comment as its description
```

The warning section is omitted when there are no warnings.

## Out of Scope

- Using `tracing` for user-facing output. `tracing` remains strictly for developer diagnostics; `println!`/`eprintln!` in `tfmt` commands remains the mechanism for user-facing output.
- Warning suppression or configuration flags.
- Commands other than `rename` and `list_templates` (no other command currently generates warnings).
