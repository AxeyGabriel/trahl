-- =====================================================
-- Trahl File Database Scheme
-- =====================================================

PRAGMA foreign_keys = ON;

-- Root directories scanned recursively by the system
CREATE TABLE library (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    path            TEXT NOT NULL UNIQUE,
    last_scanned_at DATETIME
);

-- Files discovered within libraries
CREATE TABLE file_entry (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    library_id      INTEGER NOT NULL REFERENCES library(id) ON DELETE CASCADE,
    file_path       TEXT NOT NULL UNIQUE,
    file_size       INTEGER,
    hash            TEXT,
    discovered_at   DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Processing jobs associated with discovered files
CREATE TABLE job (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id         INTEGER NOT NULL REFERENCES file_entry(id) ON DELETE CASCADE,
    status          TEXT NOT NULL DEFAULT 'pending',
    output_path     TEXT,
    output_size     INTEGER,
    ratio           REAL,
    log_path        TEXT,
    started_at      DATETIME,
    finished_at     DATETIME,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);
