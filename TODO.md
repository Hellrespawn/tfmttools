# TODO.md

## Backlog

- [ ] Notify user when files aren't moved
- [ ] Don't offer to delete files when there are files with same destination

- [ ] Do test with interactive app?
- [ ] Some sort of transactions mechanism?
- [ ] Fix/warn tag with leading/trailing whitespace?
- [ ] Add year to singles?
- [ ] Standardize use of date/year?
- [ ] Replace custom history format with SQLite?
    ~~This doesn't want to work, because the current rust implementation requires getting references to Records, whereas you can't return a reference to data returned from SQLite (it's owned).~~

    This is now fixed
- [ ] i18n?

## In progress

## Done

- [x] Add more description when running `rename` under `--yes`
- [x] Handle undone records superseded by new actions.
- [x] Actually check the moved files is the same file in tests?
  - [x] Add checksum to testcase
- [x] Handle reading checksum of too big file. Only read/checksum first x bytes?
- [x] Handle checksum of multiple files with same name and contents
- [x] Add PKGBUILD
- [x] Move config back to home dir
