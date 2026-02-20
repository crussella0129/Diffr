pub mod migration;
pub mod ops;
pub mod schema;

use rusqlite::Connection;
use std::path::Path;

/// Open (or create) the Diffr database at the given path and run migrations.
pub fn open_db(path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migration::run_migrations(&conn)?;
    Ok(conn)
}

/// Open an in-memory database for testing.
pub fn open_memory_db() -> anyhow::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    migration::run_migrations(&conn)?;
    Ok(conn)
}
