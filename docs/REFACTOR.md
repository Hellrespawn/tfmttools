# Repo Refactor Plan

This document proposes a pragmatic cleanup of the repository structure.
The goal is to make the workspace easier to navigate without forcing
unnecessary crate churn.

## Goals

- Make the top-level layout easier to scan.
- Clarify which directories are product code, test support, fixtures, and
  packaging.
- Reduce naming noise from repeated `tfmttools-` directory prefixes.
- Keep published crate/package changes proportional to the benefit.

## Recommended Direction

The repo is not too large, but it looks more fragmented than it is because
every workspace member sits at the root with a long `tfmttools-*` name. The
highest-value improvement is to reorganize layout and naming, not to split the
code into more crates.

Target shape:

```text
crates/
  cli/
  core/
  fs/
  history-core/
  history-serde/
  test-support/
tests/
  fixtures/
    cli/
examples/
docs/
packaging/
  arch/
```

This preserves the current conceptual boundaries while making the workspace
easier to understand at a glance.

## Do Now

These changes are low-risk and produce immediate organizational benefit.

### ~~1. Move workspace members under `crates/`~~

Current root layout is dominated by:

- `tfmttools-cli/`
- `tfmttools-core/`
- `tfmttools-fs/`
- `tfmttools-history-core/`
- `tfmttools-history-serde/`
- `tfmttools-test/`

Recommended directory layout:

- `crates/cli/`
- `crates/core/`
- `crates/fs/`
- `crates/history-core/`
- `crates/history-serde/`
- `crates/test-support/`

Notes:

- Package names in each `Cargo.toml` can stay unchanged initially.
- Update workspace member paths in the root `Cargo.toml`.
- This is mostly a path change, so it is disruptive to local links but low-risk
  from a code-behavior perspective.

### 2. ~~Rename `tfmttools-test` to reflect its purpose~~

The crate currently provides shared test utilities, not the entire test suite.
Its current name makes the repository structure less clear.

Recommended name:

- Directory: `crates/test-support/`
- Package: either keep `tfmttools-test` for compatibility or rename to
  `tfmttools-test-support`

Recommendation:

- Rename the directory now.
- Rename the package only if there is no external tooling or downstream usage
  that depends on the current package name.

Execution:

Renamed to crates/test-harness

### ~~3. Move fixture data to a top-level `tests/fixtures/` directory~~

Current fixture layout is under `tfmttools-test/testdata/`, which hides core
CLI behavior behind an internal support crate.

Recommended layout:

- `tests/fixtures/cli/cases/`
- `tests/fixtures/cli/audio/`
- `tests/fixtures/cli/extra/`
- `tests/fixtures/cli/template/`
- `tests/fixtures/cli/test-template.html`

Benefits:

- Makes scenario coverage visible from the repo root.
- Separates reusable Rust test helpers from static fixture assets.
- Aligns better with how integration tests are usually discovered.

### ~~4. Move packaging files out of the root~~

Current root-level packaging file:

- `PKGBUILD`

Recommended layout:

- `packaging/arch/PKGBUILD`

Benefits:

- Keeps the root focused on workspace concerns.
- Creates room for future packaging metadata without root clutter.

### 5. Expand `README.md` with a workspace map

The current README is minimal and does not explain why the workspace is split
into multiple crates.

Add a short section covering:

- What the `tfmt` binary does.
- What each workspace crate is responsible for.
- Where examples live.
- Where integration fixtures live.

This is cheap and removes much of the cognitive overhead of the current layout.

## Do Later

These changes may be worthwhile, but they should follow layout cleanup rather
than precede it.

### 6. Reassess the `history-core` and `history-serde` split

Current state:

- `history-core` defines the abstractions and types.
- `history-serde` provides the concrete serde-backed implementation.

This split only pays for itself if multiple storage backends are expected.

Recommendation:

- Keep the split for now if additional history backends are plausible.
- Merge them into a single `history` crate if serde-on-disk is the only real
  implementation path.

Decision rule:

- If no second backend is planned, prefer merging.
- If alternative persistence backends are expected, keep both crates but group
  them under `crates/` so the relationship is clearer.

### 7. Simplify the CLI crate’s internal layout

Current CLI sources are spread across:

- `args.rs`
- `options.rs`
- `cli.rs`
- `term.rs`
- `history/`
- `ui/`
- `commands/`

This is not wrong, but it is slightly diffuse for the current codebase size.

Possible cleanup options:

- Merge `args.rs` and `options.rs` into a single argument-parsing area.
- Group presentation concerns under a clearer `ui/` or `presentation/` module.
- Keep command wiring and execution under `commands/`.

Recommendation:

- Do not refactor this until there is friction in everyday development.
- If touched, prefer small naming and grouping improvements over a full rewrite.

### 8. Split human-facing examples from test-only assets more aggressively

Right now some template/report assets sit with test data even though they are
useful for contributors as examples.

Recommendation:

- Keep test-only fixtures under `tests/fixtures/`.
- Move contributor-facing examples to `examples/` or `docs/`.
- Keep one canonical sample report/template path referenced from the README.

## Don’t Bother Yet

These changes are unlikely to pay off right now.

### 9. Do not add more crates just to match module names

The workspace already has several crates. Additional crate extraction would add
build, dependency, and navigation overhead without obvious benefit.

Avoid:

- Splitting UI into its own crate.
- Splitting template logic into a new crate unless it is reused independently.
- Extracting tiny abstractions into separate workspace members.

### 10. Do not chase “perfect” conventional layout at the cost of churn

This repository is a tool workspace, not a generic template project. The goal is
clarity, not strict adherence to someone else’s directory scheme.

Avoid:

- Renaming modules solely for style.
- Moving files that already have clear ownership unless the move improves
  discoverability.
- Refactoring crate boundaries before fixing the top-level structure.

## Suggested Order

1. Move workspace crates under `crates/`.
2. Rename the test helper directory to `test-support`.
3. Move fixture assets to `tests/fixtures/cli/`.
4. Move `PKGBUILD` under `packaging/arch/`.
5. Update `README.md` with a workspace map.
6. Reevaluate whether `history-core` and `history-serde` should remain separate.
7. Tidy the CLI crate layout only if it still feels noisy after the above.

## Expected Outcome

After the “Do Now” changes:

- The root will communicate the project structure more clearly.
- Contributors will find test scenarios faster.
- Crate responsibilities will read as intentional rather than historical.
- Further refactors will become easier because the repository layout will no
  longer obscure the architecture.
