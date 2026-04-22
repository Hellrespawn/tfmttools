# TODO.md

## Backlog

- [ ] Fix(?) encoding of German characters
- [ ] Add some sort of strict interpolation of forbidden characters.
- [ ] Do test with interactive app?
- [ ] Some sort of transactions mechanism?
- [ ] Fix/warn tag with leading/trailing whitespace?
- [ ] Add year to singles?
- [ ] Standardize use of date/year?
- [ ] Add shell completions to release archives?
- [ ] Add man pages to release archives?
- [ ] Evaluate Windows long-path mode support.
- [ ] Test Windows behavior on network shares.
- [ ] Test Unicode normalization behavior across supported platforms.
- [ ] Add targeted coverage for cross-volume moves.
- [ ] Separate build scripts for Windows?
- [ ] Replace custom history format with SQLite?
- [ ] i18n?
- [ ] Refactor the test harness report to output JSON, have a static HTML-page to render that JSON?

## In progress

## Done

- [x] Don't (offer) to remove files that are renamed to the same location.
- [x] Notify user when files are renamed to the same location.
- [x] Add more description when running `rename` under `--yes`
- [x] Handle undone records superseded by new actions.
- [x] Actually check the moved files is the same file in tests?
  - [x] Add checksum to testcase
- [x] Handle reading checksum of too big file. Only read/checksum first x bytes?
- [x] Handle checksum of multiple files with same name and contents
- [x] Add PKGBUILD
- [x] Move config back to home dir
