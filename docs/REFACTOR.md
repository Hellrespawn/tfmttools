# Refactor Plan

This document proposes a staged simplification of the code and module
structure. The focus is reducing orchestration complexity, tightening
crate boundaries, and removing duplicated logic without changing user
visible behavior.

## Goals

- Make the rename flow readable end-to-end in one pass.
- Move execution logic to the crates that own the behavior.
- Reduce duplicate branching and duplicate action handling.
- Make APIs more explicit about what mutates state and what does not.
- Keep each step small enough to land safely with tests.

## Constraints

- Preserve current CLI behavior and output unless a step explicitly says
  otherwise.
- Prefer moving code before rewriting behavior.
- Keep integration fixtures passing after each stage.
- Avoid broad cross-crate redesign until local simplifications have
  reduced the surface area.

## Stage 1: Low-risk cleanup

These changes are small, local, and should be done first.

### 1. Fix obviously accidental complexity

- Fix the template description parser in
  [crates/fs/src/template.rs](/home/stef/projects/rust/tfmttools/crates/fs/src/template.rs#L98).
  `COMMENT_END` is currently `"{#"` instead of `"#}"`.
- Remove dead or placeholder code that obscures intent:
  - the commented-out iterator experiment in
    [crates/cli/src/commands/rename/mod.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/mod.rs#L81)
  - the commented-out `.rev()` and `// Ok(remaining)` in
    [crates/cli/src/commands/rename/cleanup.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/cleanup.rs#L165)
  - unused interpolation modes in
    [crates/core/src/templates/context.rs](/home/stef/projects/rust/tfmttools/crates/core/src/templates/context.rs#L12)

### 2. Tighten read-only APIs

- Change read-only history methods in
  [crates/history/src/history.rs](/home/stef/projects/rust/tfmttools/crates/history/src/history.rs#L129)
  to take `&self` instead of `&mut self`:
  - `get_previous_record`
  - `get_records_to_undo`
  - `get_records_to_redo`
  - `get_n_records_to_undo`
  - `get_n_records_to_redo`
  - `get_all_records_to_undo`
  - `get_all_records_to_redo`
  - `is_empty`
- Keep `set_record_state`, `push`, `save`, and `remove` mutable.

### 3. Remove local duplication in small helpers

- Add a shared helper for repeated confirmation checks:
  currently duplicated in
  [crates/cli/src/commands/rename/apply.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/apply.rs#L41)
  and
  [crates/cli/src/commands/rename/cleanup.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/cleanup.rs#L106).
- Add a shared helper for previewing paths relative to cwd:
  currently split between
  [crates/cli/src/commands/rename/apply.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/apply.rs#L80)
  and
  [crates/cli/src/commands/rename/cleanup.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/cleanup.rs#L213).

## Stage 2: Simplify rename orchestration

The main complexity is the rename command flow. The current structure
already has `setup`, `apply`, and `cleanup`, but shared helpers live in
the parent module and important decisions are spread across files.

### 4. Introduce a `RenameSession`

Create a type in `crates/cli/src/commands/rename/` that owns the command
flow. Suggested shape:

```rust
pub struct RenameSession<'a> {
    context: &'a RenameContext<'a>,
    history: History<Action, ActionRecordMetadata>,
    load_result: LoadHistoryResult,
}
```

Methods:

- `load(context) -> Result<Self>`
- `plan_actions(&mut self) -> Result<RenamePlan>`
- `preview(&self, plan: &RenamePlan) -> Result<()>`
- `execute(&self, plan: RenamePlan) -> Result<ExecutionResult>`
- `cleanup(&mut self, execution: ExecutionResult) -> Result<()>`
- `save_history(&mut self, actions, metadata) -> Result<()>`

Expected outcome:

- [crates/cli/src/commands/rename/mod.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/mod.rs#L23)
  becomes a short top-level entry point.
- Shared helpers like `move_files_iter` stop living in the parent module
  only to be called from sibling modules.

### 5. Introduce a `RenamePlan`

Bundle together the outputs of setup:

```rust
pub struct RenamePlan {
    actions: Vec<RenameAction>,
    unchanged_files: Vec<Utf8File>,
    metadata: ActionRecordMetadata,
}
```

This removes the current pattern where actions and metadata are created
in one file, unchanged files are computed in another, and everything is
passed around separately.

### 6. Make rename execution return domain types

Replace the current `RenameResult` in
[crates/cli/src/commands/rename/mod.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/mod.rs#L17)
with types that reflect workflow states more directly:

- `RenamePlan`
- `ExecutionResult`
- `CleanupPlan`

This reduces enum branching that currently combines planning and
execution concerns.

## Stage 3: Unify template resolution

Template resolution currently has three parallel code paths in
[crates/cli/src/commands/rename/setup.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/setup.rs#L17):

- reuse previous template
- load named/file template
- load inline script

### 7. Introduce `TemplateSource`

Suggested shape:

```rust
enum TemplateSource {
    PreviousRun,
    FileOrName(FileOrName),
    Script(String),
}
```

Methods:

- `resolve(&self, context, history, load_result) -> Result<ResolvedTemplate>`
- `metadata(&self, run_id: &str, arguments: &[String]) -> ActionRecordMetadata`

Where `ResolvedTemplate` contains:

- a `TemplateLoader`
- the selected template name
- resolved arguments
- `ActionRecordMetadata`

Expected simplification:

- `create_actions_from_previous_template`
- `create_actions_from_file_or_name`
- `create_actions_from_script`

can collapse into one resolution path plus one shared action-creation
path.

### 8. Simplify `TemplateLoader`

In [crates/fs/src/template.rs](/home/stef/projects/rust/tfmttools/crates/fs/src/template.rs#L21),
replace the three constructors with one source-driven entry point:

```rust
enum TemplateLoaderSource<'a> {
    Directory(&'a Utf8Directory),
    File { path: &'a Utf8Path, name: &'a str },
    Script(&'a str),
}
```

Then implement `TemplateLoader::from_source(source)`.

This keeps environment construction in one place and makes loader setup
more uniform.

## Stage 4: Move file operation orchestration to `tfmttools_fs`

The CLI currently owns execution details that belong closer to the file
operation layer.

### 9. Replace `move_files_iter` with an executor type

Move the logic from
[crates/cli/src/commands/rename/mod.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/mod.rs#L55)
into `tfmttools_fs`.

Suggested shape:

```rust
pub struct ActionExecutor<'a> {
    handler: ActionHandler<'a>,
}
```

Methods:

- `apply_rename_actions(rename_actions) -> impl Iterator<Item = TFMTResult<Action>>`
- `apply_actions(actions) -> TFMTResult<Vec<Action>>`
- `remove_directories(directories) -> TFMTResult<Vec<Action>>`

Expected outcome:

- [crates/cli/src/commands/rename/apply.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/apply.rs#L107)
  becomes progress reporting plus error presentation.
- [crates/cli/src/commands/rename/cleanup.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/cleanup.rs#L235)
  stops reusing parent-module execution helpers.

### 10. Reduce duplication inside `ActionHandler`

In [crates/fs/src/action.rs](/home/stef/projects/rust/tfmttools/crates/fs/src/action.rs#L68),
`apply`, `undo`, and `redo` each repeat a full `match Action`.

Refactor target:

- factor file operations into smaller internal helpers
- centralize move/copy direction handling
- keep `rename()` focused on translating one `RenameAction` into one or
  more stored `Action`s

This is a good follow-up after introducing an executor because it makes
the execution layer smaller before changing internals.

## Stage 5: Turn cleanup into planning

Cleanup currently mixes discovery, risk classification, prompting,
execution, and history updates in one function.

### 11. Introduce `CleanupPlan`

Suggested shape:

```rust
pub struct CleanupPlan {
    files_to_bin: Vec<RenameAction>,
    directories_to_remove: Vec<Utf8Directory>,
    requires_confirmation: bool,
}
```

Split current behavior in
[crates/cli/src/commands/rename/cleanup.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/cleanup.rs#L36)
into:

- `discover_remaining_items(...) -> Result<RemainingItems>`
- `plan_cleanup(...) -> Result<Option<CleanupPlan>>`
- `confirm_cleanup(...) -> Result<bool>`
- `execute_cleanup(...) -> Result<Vec<Action>>`

Expected simplification:

- the duplicated “build rename actions, maybe confirm, move files” paths
  at
  [crates/cli/src/commands/rename/cleanup.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/cleanup.rs#L99)
  and
  [crates/cli/src/commands/rename/cleanup.rs](/home/stef/projects/rust/tfmttools/crates/cli/src/commands/rename/cleanup.rs#L127)
  collapse into one execution path
- history storage becomes a separate final step instead of being mixed
  into cleanup logic

## Stage 6: Simplify template context evaluation

This is lower priority because it does not dominate the main workflow,
but it has a few easy wins.

### 12. Normalize field lookup once

In [crates/core/src/templates/context.rs](/home/stef/projects/rust/tfmttools/crates/core/src/templates/context.rs#L124),
lowercase the incoming field once and match on normalized names.

This removes repetitive variants such as:

- `"args" | "Args" | "ARGS" | ...`
- `"date" | "Date" | "DATE"`

### 13. Separate tag extraction from output coercion

Split:

- read raw tag value
- parse numeric current/total values
- apply safe interpolation cleanup
- convert to `minijinja::Value`

This makes the behavior easier to test in small unit tests and reduces
the amount of branching inside `Object::get_value`.

## Recommended implementation order

If this is executed as actual refactor work, use this order:

1. Stage 1 low-risk cleanup
2. Stage 2 rename orchestration
3. Stage 3 template resolution
4. Stage 4 execution layer move to `tfmttools_fs`
5. Stage 5 cleanup planning
6. Stage 6 template context cleanup

This order keeps behavior stable while moving the highest-complexity
flow into clearer abstractions first.

## Validation after each stage

Run these after each meaningful step:

- `cargo test --workspace`
- `cargo test -p tfmttools-cli --test integration -- --nocapture`
- `cargo +nightly clippy --workspace --all-targets`

If a stage changes output formatting intentionally, update the fixture
expectations in `tests/fixtures/cli/` in the same change.

## What not to do

- Do not merge `core`, `fs`, and `history` prematurely. The current
  crate split is mostly reasonable; the problem is orchestration and
  duplication inside the CLI command flow.
- Do not start by rewriting the template engine interface. The loader
  and context code can be improved later, but they are not the main
  readability problem.
- Do not combine behavioral changes with structural refactors. Keep
  those separate so fixture failures stay attributable.
