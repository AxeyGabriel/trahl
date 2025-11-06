-- =====================================================
-- Trahl File Database Scheme
-- =====================================================

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

-- Known workers
CREATE TABLE workers (
	id				INTEGER PRIMARY KEY AUTOINCREMENT,
	identifier		TEXT NOT NULL UNIQUE,
	last_conn_at	DATETIME
);

-- Lua Scripts
CREATE TABLE script (
	id			INTEGER PRIMARY KEY AUTOINCREMENT,
	name		TEXT NOT NULL,
	hash		TEXT NOT NULL,
	script		TEXT NOT NULL,
	source		TEXT NOT NULL, -- http:// https:// file:// dblocal
	description TEXT,
	updated_at	DATETIME DEFAULT CURRENT_TIMESTAMP,
	UNIQUE(name, hash)
);

-- Root directories scanned recursively by the system
CREATE TABLE library (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
	name			TEXT NOT NULL UNIQUE,
	source			TEXT NOT NULL, -- dblocal, conf
	destination		TEXT NOT NULL,
	enabled			INTEGER NOT NULL,
    path            TEXT NOT NULL UNIQUE,
	script_id		INTEGER NOT NULL REFERENCES script(id),
    last_scanned_at DATETIME
);

CREATE TABLE variables (
	id				INTEGER PRIMARY KEY AUTOINCREMENT,
	key				TEXT NOT NULL,
	value			TEXT,
	library_id		INTEGER REFERENCES library(id) -- null=global variable
);

-- Files discovered within libraries
CREATE TABLE file_entry (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    library_id      INTEGER NOT NULL REFERENCES library(id) ON DELETE CASCADE,
    file_path       TEXT NOT NULL, -- relative to library root path
    file_size       INTEGER,
    hash            TEXT,
    discovered_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
	UNIQUE(library_id, file_path)
);

-- Processing jobs associated with discovered files
CREATE TABLE job (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id         INTEGER NOT NULL REFERENCES file_entry(id) ON DELETE CASCADE,
	worker_id		INTEGER REFERENCES workers(id),
    status          TEXT NOT NULL, -- queued, processing, success, failure
    log_path        TEXT,
    output_file     TEXT,
    output_size     INTEGER,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at      DATETIME,
    finished_at     DATETIME,
	UNIQUE(file_id, status)
);
