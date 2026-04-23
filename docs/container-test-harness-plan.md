# Container Test Harness Plan

This plan captures the design decisions for an opt-in container-backed
test harness. The goal is to run the actual `tfmt` application in a
container so filesystem behavior that is hard to reproduce in the host
fixture harness can be tested, starting with cross-filesystem rename
fallback behavior.

## Goals

- Add a slower, opt-in test suite for real filesystem situations.
- Keep the current host CLI integration harness fast and dependency-light.
- Exercise the real `tfmt` binary, built inside a Linux container.
- Start with a cross-filesystem rename case using separate named volumes.
- Verify both user-visible filesystem state and targeted internal history
  semantics when that is the behavior under test.

## Non-Goals

- Do not include container tests in `cargo xtask test` initially.
- Do not require Docker or Podman for normal workspace builds or linting.
- Do not add broad stale Docker/Podman cleanup.
- Do not add arbitrary shell commands to scenario JSON in the first version.
- Do not add test-only CLI flags just for the harness.

## Milestones

### ~~1. Refactor Existing Harness~~

Create clearer crate boundaries before adding container behavior.

- Rename the package in `crates/test-harness` from `tfmttools-test` to
  `tfmttools-test-harness`.
- Keep the directory name `crates/test-harness`.
- Reorganize `crates/test-harness` into shared common code, not a runner.
- Add `crates/test-cli` with package name `tfmttools-test-cli`.
- Move the existing host CLI runner and host report rendering into
  `crates/test-cli`.
- Update `crates/cli/tests/integration.rs` to call
  `tfmttools_test_cli::test_runner`.
- Keep the existing CLI fixture JSON schema unchanged.
- Keep `cargo xtask test-integration` behavior equivalent.
- Migrate test outcomes to a saved JSON report file.
- Render reports from a static HTML file that loads the adjacent saved
  JSON report file.
- Embed the static report viewer in the runner crate with `include_str!`.
  The viewer is harness code, not fixture data.
- Build the report viewer with Preact using the no-build-tools route:
  browser module imports, no Vite/npm build pipeline, and no JSX transform.
  See <https://preactjs.com/guide/v10/getting-started#no-build-tools-route>.
- Use HTM with Preact so the viewer stays readable without JSX or a build
  step.
- Vendor pinned Preact/HTM browser module assets in the repo and inline
  them into generated `report.html`. The report viewer should avoid
  external network access and remain usable offline.
- Store vendored report-viewer assets under
  `crates/test-harness/assets/report/`.
- Commit the exact pinned upstream Preact/HTM files plus a small
  documented refresh command or script. The harness should not depend on
  network access, but future updates should be reproducible.
- Put the refresh script under `scripts/`, for example
  `scripts/vendor-report-assets.sh`, and document the exact refresh
  command in `crates/test-harness/assets/report/README.md`.
- Keep upstream license text and source URL/version metadata next to
  vendored Preact/HTM assets.
- Inline minified pinned vendor assets in generated `report.html`; keep
  dependency details in source comments or asset metadata rather than
  visible report UI.
- Keep report viewer source as a standalone HTML template under
  `crates/test-harness/assets/report/`; Rust should substitute or inline
  the pinned assets rather than assembling the full viewer as Rust string
  fragments.
- Use explicit named placeholders in the report HTML template for inlined
  CSS and JavaScript assets. Report generation should fail if any required
  placeholder is missing or left unreplaced.
- Inline report viewer CSS in generated `report.html` as well. Keep
  `report.html` as a single-file viewer apart from adjacent
  `report.json`.
- Do not add a content security policy to generated `report.html` in the
  first version. Reconsider once the viewer stabilizes.
- Use stable report filenames: `report.html` for the static viewer and
  `report.json` for the saved outcome data.
- Copy `report.html` into each report directory so each report artifact is
  self-contained except for its adjacent JSON.
- Include every discovered case and every step outcome in `report.json`,
  including passed, failed, and skipped cases.
- Record unexecuted later steps after a failed, skipped, or timed-out step
  as explicit `skipped` step outcomes with a reason such as
  `previous_step_failed`, so reports preserve the full planned case flow.
- Do not include cases filtered out by test-name filters. Record the
  active filter arguments at the top level instead.
- Include an explicit `status` enum on run, case, and step outcomes
  rather than inferring status from failure fields or skipped flags.
- Support initial shared report statuses:
  - `passed`
  - `failed`
  - `skipped`
  - `timed_out`
- Keep `timed_out` distinct from generic failure because timeout cleanup
  and diagnostics are special.
- Use one shared `report.json` envelope for host and container runs,
  covering common metadata, filters, summary counts, and case outcomes.
  Keep runner-specific fields in dedicated detail sections.
- Store run-level summary counts in `report.json` and let the viewer
  display them directly. The report should remain useful to machine
  consumers without reimplementing aggregation.
- Include an explicit `runner` discriminator, such as `cli` or
  `container`, in the shared report envelope.
- Include a `schema_version` field in the shared report envelope. Reports
  are machine-readable artifacts and may outlive the checkout that
  produced them.
- Use integer `1` as the initial shared report `schema_version`.
- Use `snake_case` field names in generated `report.json`, matching Rust
  data structures and common JSON consumers. Keep fixture case/scenario
  field names as currently sketched unless there is a strong reason to
  change them.
- Use paths relative to the report directory for report artifacts, with an
  optional top-level absolute `report_dir` for convenience.
- Store report artifact paths as slash-separated relative paths in JSON,
  even on non-Unix hosts. Convert with platform path APIs only when
  reading or writing them on disk.
- Emit absolute `report_dir` by default for local convenience. If
  canonicalizing the report directory fails, record `null` instead of
  failing the run.
- Record only harness-relevant environment variables in `report.json`,
  such as `TFMT_CONTAINER_*` controls and forwarded test filters. Do not
  dump the full process environment.
- Include explicit environment variables passed by the harness to runtime
  commands, but not the inherited host environment.
- Record raw duration milliseconds in `report.json`; let the viewer format
  human-readable durations.
- Measure case and step durations with wall-clock milliseconds only. Do
  not capture command CPU time initially.
- Timed-out command outcomes should record both the configured timeout
  seconds and observed elapsed milliseconds.
- Record timestamps as UTC ISO 8601 / RFC 3339 strings with millisecond
  precision.
- Include both `started_at` and `generated_at` timestamps. `started_at`
  describes the run; `generated_at` describes the report artifact.
- Record the exact harness argv at the top level without redacting
  path-like arguments.
- Do not record Cargo package versions for the runner crates initially.
  Git metadata and schema version are enough for local workspace reports.
- If golden report fixtures are added later, normalize or omit absolute
  `report_dir` so fixtures stay stable across machines.

Suggested shared crate modules:

```text
crates/test-harness/src/
  lib.rs
  fixtures.rs
  expectations.rs
  outcome.rs
  report.rs
```

Suggested host runner modules:

```text
crates/test-cli/src/
  lib.rs
  case.rs
  runner.rs
  report.rs
```

The shared crate should contain lower-level utilities and common data
structures only. Host and container schemas should stay separate.

### ~~2. Container Harness Skeleton~~

- Add `crates/test-container` with package name
  `tfmttools-test-container`.
- Add `crates/cli/tests/container.rs` as a Cargo integration test entry
  point.
- Add `cargo xtask test-container`.
- Keep `test-container` opt-in; do not include it in `test-cli`,
  `test-integration`, or `test`.
- If a case step fails, is skipped, or times out, stop executing later
  steps in that case for the first version, then verify/export the
  resulting state for diagnostics as appropriate.
- A failed container case should not stop the whole container suite.
  Continue to later cases after exporting diagnostics unless setup/runtime
  state is so broken that the runner cannot proceed.
- Setup failure for a specific case should produce a failed case outcome.
  Use a run-level failure only when setup infrastructure is broken before
  a case can be associated.
- Detect runtime in this order:
  1. `TFMT_CONTAINER_RUNTIME`
  2. `docker`
  3. `podman`
- If no runtime is found, skip successfully by default.
- If `TFMT_CONTAINER_REQUIRED=1` is set, missing runtime is a failure.
- If `TFMT_CONTAINER_RUNTIME` names a missing runtime, fail.
- Compile `crates/cli/tests/container.rs` during normal workspace test
  builds, even though the container suite is opt-in. The test should skip
  successfully unless explicitly required or invoked through
  `cargo xtask test-container`.
- Build the image automatically by default.
- Add controls:
  - `TFMT_CONTAINER_IMAGE`
  - `TFMT_CONTAINER_SKIP_BUILD=1`
  - `TFMT_CONTAINER_REBUILD=1`
  - `TFMT_CONTAINER_PRESERVE=1`
  - `TFMT_CONTAINER_REQUIRED=1`
- Do not add `TFMT_CONTAINER_PROFILE` initially. Build debug binaries by
  default and add profile selection later only if needed.
- `TFMT_CONTAINER_REBUILD=1` should force the build command, but it
  should not remove existing images. Image pruning should stay outside the
  harness.
- Do not add per-step timeout fields to container cases initially. Use one
  conservative runner-level timeout for container commands and add
  per-step timeout schema only when there is a concrete slow-step need.
- Add `TFMT_CONTAINER_TIMEOUT_SECONDS` to configure the runner-level
  container command timeout.
- Default `TFMT_CONTAINER_TIMEOUT_SECONDS` to `300`.
- Reject zero, negative, and invalid `TFMT_CONTAINER_TIMEOUT_SECONDS`
  values instead of disabling timeouts or falling back silently.
- Accept positive integer seconds only for `TFMT_CONTAINER_TIMEOUT_SECONDS`;
  do not accept fractional seconds.
- Do not apply `TFMT_CONTAINER_TIMEOUT_SECONDS` to image builds initially.
  Add image build timeout behavior separately only if builds hang in
  practice.

Suggested container runner modules:

```text
crates/test-container/src/
  lib.rs
  case.rs
  image.rs
  protocol.rs
  report.rs
  runner.rs
  runtime.rs
  scenario.rs
  bin/
    tfmt-container-verify.rs
```

### ~~3. Container Image~~

- Add `tests/container/Containerfile`.
- Use the local workspace checkout as the build context so uncommitted
  changes are tested.
- Add a root `.dockerignore` to exclude at least:
  - `target/`
  - `.git/`
  - `tests/fixtures/cli/report/`
  - `tests/fixtures/container/report/`
- Build inside the image, not by mounting a host-built binary.
- Use the workspace MSRV as the builder base, such as `rust:1.85-bookworm`.
- Use a compatible runtime/test base, such as `debian:bookworm-slim`.
- Build debug binaries by default.
- Use a stable default local image tag; do not include the git commit hash
  in the default tag. Record the resulting image id and source metadata in
  the report instead.
- Use `tfmttools-test-container:local` as the default local image tag.
- Keep the first image build conventional and rely on layer caching.
  Do not add BuildKit or buildah cache mounts until build time proves it
  is worth the extra runtime-specific behavior.
- The image should contain:
  - `tfmt`
  - `tfmt-container-verify`
  - basic tools such as `cp`, `mkdir`, `stat`, `mount`, and `find`
- Use the same image for setup, app, verifier, diagnostics, and export
  containers.
- Create a non-root app user with UID/GID `1000:1000`.

### ~~4. Container Fixture Tree~~

Use a separate self-contained fixture tree:

```text
tests/fixtures/container/
  README.md
  audio/
  cases/
  extra/
  report/
  scenarios/
  template/
  test-template.html
```

- Do not reuse or symlink `tests/fixtures/cli/*`.
- Copy only the minimal assets needed for container tests.
- Copy one existing small compatible audio fixture into this tree during
  implementation so the suite is self-contained.
- Keep container reports separate from host CLI reports.
- Each runner owns its own report rendering.
- Use container case and scenario filenames as stable IDs initially. Do
  not add explicit `id` fields to each JSON file in the first version.
- Use one case per container case JSON file so artifact paths, failure
  reports, and filters remain straightforward.
- Sort fixture paths before execution so case discovery is deterministic.
  Preserve discovery order in `report.json` and in the report viewer
  rather than sorting cases again by fixture ID.
- Match filters against case IDs initially. Add description or step-name
  matching later only if real usage shows that case-ID filtering is too
  narrow.

### 5. ~~First Real Scenario~~

Start with cross-filesystem rename fallback.

- Use named volumes for source, target, and config.
- Use `/work` as the process current directory.
- Keep `/work` ephemeral except for `/work/config`.
- Mounts:
  - `source` -> `/mnt/source`
  - `target` -> `/mnt/target`
  - `config` -> `/work/config`
- Run setup/export containers as root.
- After setup copies fixtures into named volumes, normalize app-readable
  and app-writable paths created by setup operations to UID/GID
  `1000:1000` unless a future scenario explicitly tests permission
  behavior.
- Run app containers as non-root `1000:1000`.
- Verifier and diagnostic containers should also run non-root where
  possible.
- If volume export or diagnostics need root, keep root limited to export
  or diagnostic containers.
- Run one fresh app container per test step.
- Keep named volumes for the whole case so history persists across
  apply/undo/redo.
- Force serial container case execution initially.

Scenario file example:

```json
{
  "description": "Runs with source and target on separate named volumes",
  "mounts": {
    "source": {
      "kind": "volume",
      "container-path": "/mnt/source"
    },
    "target": {
      "kind": "volume",
      "container-path": "/mnt/target"
    },
    "config": {
      "kind": "volume",
      "container-path": "/work/config"
    }
  },
  "workdir": "/work",
  "input": {
    "mount": "source",
    "path": "input"
  },
  "setup": [
    {
      "op": "copy-fixture-dir",
      "from": "audio",
      "to": { "mount": "source", "path": "input" }
    },
    {
      "op": "copy-fixture-dir",
      "from": "extra",
      "to": { "mount": "source", "path": "input/extra" }
    },
    {
      "op": "copy-fixture-dir",
      "from": "template",
      "to": { "mount": "config", "path": "" }
    }
  ],
  "preconditions": [
    {
      "kind": "different-devices",
      "left": "source",
      "right": "target"
    }
  ]
}
```

Make the scenario `preconditions` array optional and default it to an
empty list. Many future scenarios may not need preconditions.

Validate scenario mount paths. Initially allow only:

- `/mnt/<name>`
- `/work/config`
- `/work` as the workdir

Reject duplicate paths, relative paths, `..`, and invalid alias names.

### 6. First Case

Use one audio file and a normal template file.

- Store the target template in the config volume.
- Use `-t cross_device`, not `--script`.
- Template should render an absolute nested target path:

```jinja
/mnt/target/{{ artist }}/{{ title }}
```

The first case should include ordered steps:

1. Initial state.
2. Apply rename.
3. Undo.
4. Redo.

Container cases should use argv arrays, not whitespace-split command
strings:

```json
{
  "description": "Rename across source and target volumes falls back to copy/remove",
  "scenario": "cross-device-volumes",
  "expectations": {
    "initial-state": [
      {
        "mount": "source",
        "path": "input/Nightwish - Nemo.mp3",
        "checksum": "820F92DA"
      }
    ],
    "applied": [
      {
        "mount": "target",
        "path": "Nightwish/Nemo.mp3",
        "checksum": "820F92DA"
      }
    ]
  },
  "history": {
    "copy-remove-applied": {
      "mount": "config",
      "path": "tfmttools-cli.hist",
      "record": 0,
      "contains-actions": ["CopyFile", "RemoveFile"],
      "does-not-contain-actions": ["MoveFile"]
    }
  },
  "steps": [
    {
      "name": "initial",
      "expectations": "initial-state"
    },
    {
      "name": "apply",
      "command": [
        "--simple",
        "--yes",
        "rename",
        "--input-directory",
        "/mnt/source/input",
        "-t",
        "cross_device"
      ],
      "exit-code": 0,
      "expectations": "applied",
      "before": "initial-state",
      "history": "copy-remove-applied"
    },
    {
      "name": "undo",
      "command": ["--simple", "--yes", "undo"],
      "exit-code": 0,
      "expectations": "initial-state",
      "before": "applied"
    },
    {
      "name": "redo",
      "command": ["--simple", "--yes", "redo"],
      "exit-code": 0,
      "expectations": "applied",
      "before": "initial-state"
    }
  ]
}
```

Use an explicit ordered `steps` array for case execution. Do not rely on
JSON object order for `expectations`, `history`, or command/check entries.
Use `steps`, not `tests`, for ordered command/check entries and reserve
`tests` for harness-level concepts if needed later.
Steps may omit `command` and perform only expectation and history checks,
as the initial-state step does. Use `before` for pre-step expectations and
`expectations` for after-step expectations.
Run filesystem expectations before history checks; the user-visible state
is the primary behavior, and history assertions are targeted internals.

Default `exit-code` to `0`. Record stdout and stderr in outcomes, but do
not add stdout/stderr matching until a case needs it.

Keep full stdout and stderr in `report.json` initially because the first
suite is small. Add truncation with separate artifact files only if
reports become too large.

For the first case, assert the applied history record's action variants
only. Do not assert history record states after undo and redo unless a
state-specific bug appears. Filesystem state after undo and redo is enough
to prove the initial user-visible behavior.

The first case should also assert absence explicitly: after apply, the
original source path should be absent; after undo, the target path should
be absent. This prevents copy/remove fallback from leaving both source and
target files behind.

For the first case, check file absence only. Directory cleanup semantics
are separate behavior and should get explicit tests if they matter.
Absence checks should assert non-existence only; checksum assertions apply
only to files expected to exist.

### 7. Verification

Run verification inside a verifier container.

- Add a small Rust verifier binary, not a shell verifier.
- Run the verifier as non-root first.
- The verifier should use production types where practical:
  - `tfmttools_core::action::Action`
  - `tfmttools_core::history::ActionRecordMetadata`
  - `tfmttools_history::History`
  - `tfmttools_fs::get_path_checksum`
- Convert production action variants to stable assertion names:
  - `MoveFile`
  - `CopyFile`
  - `RemoveFile`
  - `MakeDir`
  - `RemoveDir`
- Share runner/verifier JSON through a `protocol` module in
  `tfmttools-test-container`.
- Store verifier request files in a separate host tempdir mounted read-only
  at `/verify`, not in the app config volume.
- The verifier prints a structured JSON response to stdout.
- The verifier response should include both structured failure
  codes/paths and concise human-readable messages.
- The runner parses the verifier response and maps it into report outcomes.

Use logical mount aliases in expectations:

```json
{ "mount": "target", "path": "Nightwish/Nemo.mp3" }
```

Do not put raw absolute paths throughout cases. Reject `..` in logical
paths.
Keep expected checksums as strings in fixture JSON and parse/normalize
them in the verifier using the production checksum format.

Do not add schema version fields to case or scenario files initially.
Use strict deserialization with unknown fields denied; add explicit
versions only once migrations become real.

Do not add a separate verifier protocol version initially. The runner and
verifier are built from the same checkout and communicate through shared
Rust types.

### 8. Setup and Export

Use setup containers with fixture root bind-mounted read-only and named
volumes mounted writable.

- The image should contain binaries and tools, not fixture data.
- The runner copies fixture data into volumes at test time.
- Create all declared scenario volumes before setup rather than lazily per
  mount alias. This catches invalid mount declarations early and
  simplifies cleanup.
- Execute first-version setup operations from the runner by invoking basic
  container tools for `mkdir` and `copy-fixture-dir`. Add a dedicated
  setup binary only if quoting, ownership, or portability becomes
  fragile.
- Use structured setup operations:
  - `mkdir`
  - `copy-fixture-dir`
- Add later only if needed:
  - `chmod`
  - `chown`
  - `write-file`
  - `copy-fixture-file`
- Do not support arbitrary shell commands in scenario JSON initially.
- `copy-fixture-dir` should copy symlinks as symlinks if they appear,
  matching `cp -a` export semantics. Avoid symlinks in first-version
  fixtures unless a future scenario explicitly tests them.
- Do not add explicit ownership overrides to setup operations initially.
  Keep ownership policy centralized and add per-operation ownership only
  for future permission-test scenarios.
- Track paths created by setup operations and normalize that set once
  after all setup operations finish.
- Record setup-created paths in report diagnostics even when setup
  succeeds, so ownership normalization and later cleanup can be debugged.

On failure:

- Export each current-run volume into:

```text
tests/fixtures/container/report/artifacts/<case-name>/<alias>/
```

- Use an export container with:
  - volume mounted read-only at `/from`
  - artifact directory bind-mounted writable at `/to`
  - `cp -a /from/. /to/`
- Also write:
  - `diagnostics.json`
  - `commands.json`
  - `verify-request-<step>.json`
  - `verify-response-<step>.json`
- Include verifier request/response artifact paths for every step that
  runs verifier checks.
- When exporting an empty volume, include a small marker file or
  diagnostics entry so an empty exported directory is distinguishable from
  a failed export.
- Save the report source as an adjacent machine-readable JSON file and
  render it through a static report page. Do not embed the JSON in the
  HTML. This applies to the refactored host runner and the container
  runner.
- Use `report.html` for the static viewer and `report.json` for the saved
  outcome data in each report directory.
- Copy `report.html` into every report directory so a report artifact can
  be moved or archived together with its adjacent JSON.
- Generate `report.html` from the runner-embedded static viewer.
- Inline vendored pinned Preact/HTM browser module assets into
  `report.html`; do not require copying separate viewer dependency files
  into report directories.
- Vendor those assets under `crates/test-harness/assets/report/` with
  license files and source URL/version metadata.
- Keep dependency/version details out of the visible report UI.
- Include all discovered cases and step outcomes in `report.json`; the
  HTML can emphasize failures first, but the JSON should preserve the
  complete run.
- Treat the filtered case set as the run. Do not include filtered-out
  cases in `report.json`; record filter arguments at the top level.
- Record source metadata in `report.json` on a best-effort basis:
  `git_head`, `git_dirty`, and `git_diff_stat`. If git metadata is
  unavailable, record `null` rather than failing the run.
- Use one shared report envelope for host and container runs, with common
  metadata and runner-specific detail sections.
- Include `schema_version`, `runner`, and relative artifact paths in the
  shared report envelope. Optionally include an absolute `report_dir` for
  convenience.
- Use one shared `report.html` viewer for host and container reports. The
  viewer should branch on the report envelope's `runner` field and
  runner-specific detail sections.
- The shared viewer should fail closed for unsupported major
  `schema_version` values and show a clear unsupported-schema message.
  Unknown fields within the supported schema should be ignored.
- Implement the shared viewer with Preact's no-build-tools route. Do not
  add frontend build tooling for the report viewer in the first version.
- Use HTM with Preact, avoid external network access, and keep
  `report.html` as a single-file viewer apart from the adjacent
  `report.json`.
- Test the shared `report.html` viewer against a small fixture
  `report.json` or equivalent unit test so schema changes do not silently
  break the viewer.
- Keep shared report viewer tests crate-local in
  `crates/test-harness/tests/` with small fixture data under
  `crates/test-harness/tests/fixtures/report/`.
- Start with a Rust test that verifies expected embedded assets and the
  JSON-loading hook exist; add browser-based tests later only if viewer
  behavior becomes complex.
- Save a skip report when the container suite skips because Docker or
  Podman is missing. The report should include the exact missing-runtime
  reason.
- Include image build stdout/stderr in `report.json` when image build
  fails. Do not store successful build logs unless a verbosity control is
  added later.
- Store image build failure output directly in `report.json` initially.
- Write a run-level failure report when image build fails before any cases
  run. The report should include zero executed cases and captured build
  output.
- Include a top-level summary for image-build-failed runs that reports
  zero executed cases and a run-level failure reason.
- Represent run-level failures, such as image build failure, with both
  summary counts and a top-level `run_failure` object. Keep `cases` empty
  when no cases execute.
- Document that the static HTML report may need to be served by a small
  local static server if a browser blocks direct `file://` JSON loading.
- Keep verifier request/response artifacts for passing verifier-backed
  steps for now, but allow moving them behind a future verbosity or
  retention control if report directories become noisy.

Cleanup policy:

- Passing cases remove current-run volumes.
- Failing cases export artifacts, then remove current-run volumes.
- Timeout failures follow the same cleanup policy as other failures.
- Timeout failures should keep partial stdout/stderr captured before the
  process was killed and mark the command outcome as timed out.
- Timed-out container commands should use the runtime's normal stop
  behavior with a short grace period, then force removal and cleanup if
  needed.
- If `TFMT_CONTAINER_PRESERVE=1`, keep failed current-run volumes and
  print exact cleanup commands.
- If `TFMT_CONTAINER_PRESERVE=1` is active during a timeout, preserve all
  diagnostic artifacts that help explain the timeout, including verifier
  request/response files and command logs.
- Do not add a Ctrl-C or signal handler for cleanup in the first version.
  Use normal scope-based cleanup for handled failures and document manual
  cleanup commands when volumes are preserved or a run is interrupted.
- Never scan or prune old `tfmttools-*` volumes automatically.
- Never prune images automatically.

Volume names should be readable and collision-resistant:

```text
tfmttools-<run-id>-<case-slug>-<alias>-<random>
```

Use the same random suffix for all volumes in a case.
Sanitize case slugs and cap their length so runtime volume-name limits are
not reached. Rely on the run id and random suffix for uniqueness.
Do not include the scenario ID in generated volume names initially. The
report already records the scenario ID.

### 9. Diagnostics

Verifier diagnostics should include:

- resolved expectation paths
- exists/not-exists results
- checksum actual/expected values
- history file path
- parsed history record states
- action variants seen
- mount alias mapping used

Runner diagnostics should include:

- runtime name and version
- raw runtime version output, such as `docker --version` or
  `podman --version`, rather than a parsed structured version initially
- image tag/id
- volume names
- container IDs or names for setup, app, verifier, diagnostics, and export
  commands when the runtime returns them
- exact runtime argv for setup, app, verifier, diagnostics, and export
- app stdout/stderr/exit code
- verifier stdout/stderr/exit code
- app container mount info or `/proc/self/mountinfo`
- artifact paths
- cleanup commands when volumes are preserved

Precondition policy:

- Missing runtime: skip by default, fail with `TFMT_CONTAINER_REQUIRED=1`.
- Missing explicitly requested runtime: fail.
- Image build failure: fail.
- Setup failure: fail.
- Verifier failure: fail.
- Named volumes violating declared preconditions: fail.
- Privileged or optional future network scenarios without required support:
  skip.

When a missing runtime causes a skip, still write `report.json` and
`report.html` so local and CI behavior is auditable.

Keep the runtime abstraction compatible with Docker and Podman from the
start, but validate Docker first. Do not add Podman CI until the Docker
path is stable.

### 10. xtask

Add `test-container` and improve argument forwarding for both harnesses.

- Existing:

```sh
cargo xtask test-integration
```

- New:

```sh
cargo xtask test-container
```

- Filtering should work for both:

```sh
cargo xtask test-integration simple_input
cargo xtask test-container cross_device
```

The task should forward extra args after `--` to the underlying Cargo
test invocation while preserving `--nocapture`.

`cargo xtask test-container` should not imply
`TFMT_CONTAINER_REQUIRED=1`. Keep missing runtimes as skips by default
through the xtask command, and let CI or users opt into failure with
`TFMT_CONTAINER_REQUIRED=1`.

Do not make `cargo xtask lint` build images or run containers. The new
Rust crates will be linted automatically because lint already runs
workspace clippy and format checks.
