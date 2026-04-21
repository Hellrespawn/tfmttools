# Release Checklist

Use this checklist from the workspace root.

## Before Tagging

1. Confirm the version in `Cargo.toml`.
2. Update `CHANGELOG.md` with the release date.
3. Run the local quality gate:

   ```sh
   scripts/ci.sh
   ```

4. Confirm GitHub and Forgejo CI pass on Linux and Windows.
5. Build local release artifacts where practical:

   ```sh
   scripts/package-release.sh
   ```

## Smoke Test

1. Extract the release archive.
2. Run `tfmt --version`.
3. Run `tfmt --help`.
4. Run a dry run against copied audio files:

   ```sh
   tfmt --dry-run --simple --yes rename -t simple_input
   ```

5. Run a real rename against copied files.
6. Run `tfmt undo`.
7. Run `tfmt redo`.

## Publishing

1. Tag the release:

   ```sh
   git tag v0.24.0
   git push origin v0.24.0
   ```

   Release artifact workflows only accept tags shaped like `v0.24.0`, and the
   tag must match the workspace version in `Cargo.toml`.

2. Download the Linux and Windows artifacts from the tag workflow.
3. Combine the per-platform checksum files into one release checksum file:

   ```sh
   cat SHA256SUMS-* > SHA256SUMS
   sha256sum --check SHA256SUMS
   ```

4. Publish release notes and artifacts manually on GitHub.
5. Publish the same release notes and artifacts manually on Forgejo.
6. Verify both release pages include:
   - source archive
   - Linux x86_64 archive
   - Windows x86_64 archive
   - `SHA256SUMS`
   - changelog text
   - packaged `README.md`, `LICENSE`, `CHANGELOG.md`, and `examples/`
