# TODO.md

## Backlog

- [ ] Do test with interactive app?
- [ ] Actually check the moved files is the same file in tests?
- [ ] Some sort of transactions mechanism?
- [ ] Fix/warn tag with leading/trailing whitespace?
- [ ] Add year to singles?
- [ ] Standardize use of date/year?
- [ ] Replace custom history format with SQLite?
    ~~This doesn't want to work, because the current rust implementation requires getting references to Records, whereas you can't return a reference to data returned from SQLite (it's owned).~~

    This is now fixed

## In progress

## Done

- [x] Add more description when running `rename` under `--yes`
- [x] Handle undone records superseded by new actions.
