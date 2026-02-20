use rusqlite::Connection;

use crate::schema;

#[cfg(test)]
const CURRENT_VERSION: i64 = 2;

/// Run all pending migrations.
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Ensure schema_version table exists.
    conn.execute_batch(schema::CREATE_SCHEMA_VERSION)?;

    let current = get_version(conn)?;

    if current < 1 {
        migrate_v1(conn)?;
    }
    if current < 2 {
        migrate_v2(conn)?;
    }

    Ok(())
}

fn get_version(conn: &Connection) -> anyhow::Result<i64> {
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(version)
}

fn set_version(conn: &Connection, version: i64) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?1, datetime('now'))",
        [version],
    )?;
    Ok(())
}

/// Migration v1: create all initial tables.
fn migrate_v1(conn: &Connection) -> anyhow::Result<()> {
    tracing::info!("applying migration v1: initial schema");
    conn.execute_batch(schema::CREATE_CLUSTERS)?;
    conn.execute_batch(schema::CREATE_DRIVES)?;
    conn.execute_batch(schema::CREATE_FILE_INDEX)?;
    conn.execute_batch(schema::CREATE_HASH_CACHE)?;
    conn.execute_batch(schema::CREATE_SYNC_HISTORY)?;
    conn.execute_batch(schema::CREATE_ARCHIVES)?;
    set_version(conn, 1)?;
    Ok(())
}

/// Migration v2: add sync_root column to drives.
fn migrate_v2(conn: &Connection) -> anyhow::Result<()> {
    tracing::info!("applying migration v2: add sync_root to drives");
    // Check if column already exists (fresh installs get it from CREATE_DRIVES)
    let has_column: bool = conn
        .prepare("PRAGMA table_info(drives)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .any(|name| name == "sync_root");
    if !has_column {
        conn.execute_batch("ALTER TABLE drives ADD COLUMN sync_root TEXT")?;
    }
    set_version(conn, 2)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
        assert_eq!(get_version(&conn).unwrap(), CURRENT_VERSION);
    }
}
