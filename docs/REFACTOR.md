# CLI Entrypoint Simplification Plan

## Goal

Simplify the CLI entry flow in `crates/cli` without changing behavior.
The main target is `crates/cli/src/cli/mod.rs`, where command dispatch,
command-specific setup, and user-facing error rendering are currently mixed
together.

## Current Problems

- `cli::run` is responsible for too many concerns:
  tracing setup, argument parsing, command dispatch, and non-debug error
  formatting.
- `execute` contains command-specific setup logic that belongs closer to each
  command.
- `Undo` and `Redo` are wired separately even though their setup is nearly
  identical.
- `Rename` setup builds several intermediate values inline, which makes the
  dispatch path noisy.
- `TFMTSubcommand::name()` exists only to support error rendering and currently
  allocates a `String`.
- Some argument definitions appear stale or disconnected from actual command
  handling, notably `ShowHistoryArgs` and `Seed`.

## Refactor Principles

- Keep `src/main.rs` minimal.
- Keep `cli::run` focused on top-level orchestration.
- Move command-specific setup next to the command that uses it.
- Prefer small helper methods over a single large dispatch function.
- Avoid changing CLI behavior, output text, or clap semantics unless explicitly
  intended.

## Proposed Changes

### 1. Move Subcommand Dispatch Onto `TFMTSubcommand`

Add a method on `TFMTSubcommand` such as:

```rust
pub fn run(self, app_options: &TFMTOptions, fs_handler: &FsHandler) -> Result<()>
```

This method should own the `match` over subcommands. The top-level flow then
becomes:

1. Initialize tracing.
2. Parse args.
3. Build shared application state.
4. Dispatch once through `args.command.run(...)`.

Expected result:

- `cli::run` becomes short and easier to scan.
- Subcommand behavior is easier to find from the enum definition.

### 2. Extract Undo/Redo Shared Setup

Create a helper for the duplicated `Undo` and `Redo` branches. Options:

- `fn run_undo_redo(...) -> Result<()>`
- `impl UndoRedoArgs { fn run(...) -> Result<()> }`

Shared inputs:

- `HistoryMode`
- requested amount
- confirmation mode
- preview list size
- `TFMTOptions`
- `FsHandler`

Expected result:

- one path for undo/redo setup
- less duplication in dispatch
- fewer chances for the two commands to drift apart accidentally

### 3. Move Rename Setup Into a Constructor

The rename branch currently constructs:

- `RenameOptions`
- `PathIteratorOptions`
- `RenameContext`

inline in the dispatcher.

Introduce a constructor such as:

```rust
impl RenameContext<'_> {
    pub fn try_from_args(
        fs_handler: &FsHandler,
        app_options: &TFMTOptions,
        rename_args: RenameArgs,
    ) -> Result<Self>
}
```

or a helper with equivalent responsibility.

Expected result:

- command dispatch stops knowing rename internals
- rename-specific setup lives with rename-specific types
- future rename options can evolve without bloating the entry flow

### 4. Isolate Error Rendering

Extract the non-debug error printing logic from `cli::run` into a dedicated
helper, for example:

```rust
fn render_cli_error(args: &TFMTArgs, err: &color_eyre::Report)
```

This helper should own:

- building the clap `Command`
- selecting the right subcommand help context
- formatting the error for user-facing output

Expected result:

- `cli::run` reads as straight-line control flow
- error presentation logic becomes easier to test and reason about

### 5. Remove `String` Allocation From Subcommand Name Lookup

If subcommand name lookup remains necessary, change:

```rust
pub fn name(&self) -> String
```

to:

```rust
pub fn name(&self) -> &'static str
```

If the error rendering refactor makes this helper unnecessary, remove it
entirely.

Expected result:

- less incidental work in the entry path
- simpler API on `TFMTSubcommand`

### 6. Audit and Remove Stale CLI Argument Types

Review these definitions in `crates/cli/src/cli/args.rs`:

- `ShowHistoryArgs`
- `Seed`

`ShowHistoryArgs` is especially suspicious because `ShowHistory` currently has
no payload, while `show_history` reads verbosity from `TFMTOptions`.

Decide one of:

- wire the args into the actual command shape
- or remove the unused definitions

Expected result:

- fewer misleading types in the CLI layer
- less confusion about which options are truly supported

## Suggested Implementation Order

1. Extract error rendering from `cli::run`.
2. Move subcommand dispatch onto `TFMTSubcommand`.
3. Deduplicate undo/redo setup.
4. Move rename setup into a constructor/helper.
5. Remove or simplify `TFMTSubcommand::name()`.
6. Audit and delete stale argument structs.

This order keeps behavior stable while reducing risk. Each step should leave
the codebase in a cleaner state even if the later steps are postponed.

## Validation

Run after each meaningful step:

```bash
cargo check -p tfmttools-cli
cargo test --workspace
```

Run before considering the refactor complete:

```bash
cargo +nightly fmt --all
cargo +nightly clippy --workspace --all-targets
```

Also manually verify:

- `tfmt --help`
- `tfmt rename --help`
- `tfmt undo --help`
- `tfmt redo --help`
- one failure path to confirm error rendering still points at the right command

## Non-Goals

- changing command names or aliases
- redesigning CLI UX
- rewriting command implementations outside the entry flow
- changing output text unless required by the refactor
