/// SQL statements for creating the Diffr database schema.

pub const CREATE_CLUSTERS: &str = "
CREATE TABLE IF NOT EXISTS clusters (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    topology    TEXT NOT NULL DEFAULT 'mesh',
    conflict_strategy TEXT NOT NULL DEFAULT 'newest_wins',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
)";

pub const CREATE_DRIVES: &str = "
CREATE TABLE IF NOT EXISTS drives (
    id              TEXT PRIMARY KEY,
    identity_type   TEXT NOT NULL,
    identity_value  TEXT NOT NULL,
    label           TEXT,
    mount_point     TEXT NOT NULL,
    sync_root       TEXT,
    cluster_id      TEXT,
    role            TEXT NOT NULL DEFAULT 'normal',
    is_primary      INTEGER NOT NULL DEFAULT 0,
    total_bytes     INTEGER,
    free_bytes      INTEGER,
    last_seen       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    FOREIGN KEY (cluster_id) REFERENCES clusters(id) ON DELETE SET NULL,
    UNIQUE(identity_type, identity_value)
)";

pub const CREATE_FILE_INDEX: &str = "
CREATE TABLE IF NOT EXISTS file_index (
    rel_path    TEXT NOT NULL,
    drive_id    TEXT NOT NULL,
    is_dir      INTEGER NOT NULL DEFAULT 0,
    size        INTEGER NOT NULL DEFAULT 0,
    mtime       TEXT NOT NULL,
    xxh3_hash   TEXT,
    sha256_hash TEXT,
    indexed_at  TEXT NOT NULL,
    PRIMARY KEY (rel_path, drive_id),
    FOREIGN KEY (drive_id) REFERENCES drives(id) ON DELETE CASCADE
)";

pub const CREATE_HASH_CACHE: &str = "
CREATE TABLE IF NOT EXISTS hash_cache (
    rel_path    TEXT NOT NULL,
    drive_id    TEXT NOT NULL,
    size        INTEGER NOT NULL,
    mtime       TEXT NOT NULL,
    xxh3_hash   TEXT NOT NULL,
    sha256_hash TEXT,
    cached_at   TEXT NOT NULL,
    PRIMARY KEY (rel_path, drive_id),
    FOREIGN KEY (drive_id) REFERENCES drives(id) ON DELETE CASCADE
)";

pub const CREATE_SYNC_HISTORY: &str = "
CREATE TABLE IF NOT EXISTS sync_history (
    id                TEXT PRIMARY KEY,
    cluster_id        TEXT NOT NULL,
    started_at        TEXT NOT NULL,
    finished_at       TEXT NOT NULL,
    files_synced      INTEGER NOT NULL DEFAULT 0,
    bytes_transferred INTEGER NOT NULL DEFAULT 0,
    conflicts_resolved INTEGER NOT NULL DEFAULT 0,
    errors            TEXT NOT NULL DEFAULT '[]',
    status            TEXT NOT NULL,
    FOREIGN KEY (cluster_id) REFERENCES clusters(id) ON DELETE CASCADE
)";

pub const CREATE_ARCHIVES: &str = "
CREATE TABLE IF NOT EXISTS archives (
    id              TEXT PRIMARY KEY,
    original_path   TEXT NOT NULL,
    archive_path    TEXT NOT NULL,
    drive_id        TEXT NOT NULL,
    original_size   INTEGER NOT NULL,
    compressed_size INTEGER NOT NULL,
    compression     TEXT NOT NULL DEFAULT 'zstd',
    xxh3_hash       TEXT NOT NULL,
    reason          TEXT NOT NULL,
    archived_at     TEXT NOT NULL,
    FOREIGN KEY (drive_id) REFERENCES drives(id) ON DELETE CASCADE
)";

pub const CREATE_SCHEMA_VERSION: &str = "
CREATE TABLE IF NOT EXISTS schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL
)";

/// All table creation statements in order.
pub const ALL_TABLES: &[&str] = &[
    CREATE_SCHEMA_VERSION,
    CREATE_CLUSTERS,
    CREATE_DRIVES,
    CREATE_FILE_INDEX,
    CREATE_HASH_CACHE,
    CREATE_SYNC_HISTORY,
    CREATE_ARCHIVES,
];
