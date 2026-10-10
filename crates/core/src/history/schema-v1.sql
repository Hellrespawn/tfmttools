PRAGMA application_id=1413893460;
CREATE TABLE records (
    id INTEGER PRIMARY KEY CHECK(id >= 0),
    position INTEGER NOT NULL UNIQUE CHECK(position >= 0),
    state TEXT NOT NULL CHECK(state IN ('applied','undone','redone','superseded')),
    timestamp TEXT NOT NULL,
    metadata TEXT NOT NULL CHECK(json_valid(metadata)),
    complete INTEGER NOT NULL CHECK(complete IN (0,1)),
    applied_count INTEGER NOT NULL DEFAULT 0 CHECK(applied_count >= 0),
    redo_allowed INTEGER NOT NULL DEFAULT 1 CHECK(redo_allowed IN (0,1))
) STRICT;
CREATE TABLE actions (
    record_id INTEGER NOT NULL REFERENCES records(id),
    position INTEGER NOT NULL CHECK(position >= 0),
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    PRIMARY KEY(record_id,position)
) STRICT;
CREATE TABLE patches (
    record_id INTEGER NOT NULL,
    action_position INTEGER NOT NULL,
    format TEXT NOT NULL CHECK(format='bsdiff40-v1'),
    before_length INTEGER NOT NULL CHECK(before_length >= 0),
    after_length INTEGER NOT NULL CHECK(after_length >= 0),
    before_hash BLOB NOT NULL CHECK(length(before_hash)=32),
    after_hash BLOB NOT NULL CHECK(length(after_hash)=32),
    forward BLOB NOT NULL CHECK(length(forward)>0),
    reverse BLOB NOT NULL CHECK(length(reverse)>0),
    PRIMARY KEY(record_id,action_position),
    FOREIGN KEY(record_id,action_position) REFERENCES actions(record_id,position)
) STRICT;
CREATE TABLE runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    record_id INTEGER NOT NULL REFERENCES records(id),
    kind TEXT NOT NULL CHECK(kind IN ('apply','undo','redo'))
) STRICT;
CREATE UNIQUE INDEX single_run ON runs((1));
CREATE TABLE attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL UNIQUE REFERENCES runs(id),
    action_position INTEGER NOT NULL CHECK(action_position >= 0),
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    details TEXT NOT NULL CHECK(json_valid(details))
) STRICT;
CREATE TABLE attempt_patches (
    attempt_id INTEGER PRIMARY KEY REFERENCES attempts(id) ON DELETE CASCADE,
    format TEXT NOT NULL CHECK(format='bsdiff40-v1'),
    before_length INTEGER NOT NULL CHECK(before_length >= 0),
    after_length INTEGER NOT NULL CHECK(after_length >= 0),
    before_hash BLOB NOT NULL CHECK(length(before_hash)=32),
    after_hash BLOB NOT NULL CHECK(length(after_hash)=32),
    forward BLOB NOT NULL CHECK(length(forward)>0),
    reverse BLOB NOT NULL CHECK(length(reverse)>0)
) STRICT;
