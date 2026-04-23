# Container Fixtures

This fixture tree is reserved for the opt-in container-backed harness.
It stays separate from `tests/fixtures/cli` so container scenarios can
model runtime-specific filesystem layouts without affecting the fast host
integration harness.

## Layout

- `audio/`: minimal audio assets copied into container-mounted volumes.
- `cases/`: one case per `*.case.json` file. The filename stem is the
  stable case ID used for discovery and filtering.
- `extra/`: additional fixture files used by container setup steps.
- `scenarios/`: reusable mount/setup definitions in `*.scenario.json`
  files. The filename stem is the stable scenario ID referenced by cases.
- `template/`: template files copied into the config volume.
- `test-template.html`: placeholder viewer/template fixture for future
  container report assets.

## Notes

- Keep this tree self-contained. Do not symlink or depend on
  `tests/fixtures/cli`.
- Copy only the assets needed by container scenarios.
- Container reports are generated under `tests/reports/`.
- The first case and scenario are discovery fixtures for the
  cross-filesystem rename flow and are intentionally small.
