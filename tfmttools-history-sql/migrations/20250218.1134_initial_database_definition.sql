CREATE TABLE records (
    id INTEGER PRIMARY KEY,
    state INTEGER DEFAULT 0,
    datetime TEXT NOT NULL,
    template TEXT NOT NULL,
    arguments TEXT NOT NULL,
    -- superseded_by_id INTEGER DEFAULT NULL,
    -- FOREIGN KEY(superseded_by_id) REFERENCES records(id)

) STRICT;

CREATE TABLE actions (
    id INTEGER PRIMARY KEY,
    type TEXT NOT NULL,
    target TEXT NOT NULL,
    source TEXT DEFAULT NULL,
    record_id INTEGER NOT NULL,
    FOREIGN KEY(record_id) REFERENCES records(id)
) STRICT;
