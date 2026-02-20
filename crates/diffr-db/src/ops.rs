use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use diffr_core::models::archive::{ArchiveEntry, ArchiveReason, CompressionFormat};
use diffr_core::models::cluster::{Cluster, ClusterId, ConflictStrategy, Topology};
use diffr_core::models::drive::{Drive, DriveId, DriveIdentity, DriveRole};
use diffr_core::models::file_entry::{FileEntry, HashCacheEntry};
use diffr_core::models::sync_state::{SyncRecord, SyncStatus};

// ── Helpers ──

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn fmt_dt(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

// ── Clusters ──

pub fn insert_cluster(conn: &Connection, cluster: &Cluster) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO clusters (id, name, topology, conflict_strategy, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            cluster.id.0.to_string(),
            cluster.name,
            cluster.topology.to_string(),
            cluster.conflict_strategy.to_string(),
            fmt_dt(&cluster.created_at),
            fmt_dt(&cluster.updated_at),
        ],
    )?;
    Ok(())
}

pub fn get_cluster_by_name(conn: &Connection, name: &str) -> anyhow::Result<Option<Cluster>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, topology, conflict_strategy, created_at, updated_at
         FROM clusters WHERE name = ?1",
    )?;
    let mut rows = stmt.query(params![name])?;
    match rows.next()? {
        Some(row) => {
            let id_str: String = row.get(0)?;
            let topo_str: String = row.get(2)?;
            let cs_str: String = row.get(3)?;
            let created_str: String = row.get(4)?;
            let updated_str: String = row.get(5)?;
            Ok(Some(Cluster {
                id: ClusterId::from_uuid(Uuid::parse_str(&id_str)?),
                name: row.get(1)?,
                topology: topo_str.parse().unwrap_or(Topology::Mesh),
                conflict_strategy: cs_str.parse().unwrap_or(ConflictStrategy::NewestWins),
                created_at: parse_dt(&created_str),
                updated_at: parse_dt(&updated_str),
            }))
        }
        None => Ok(None),
    }
}

pub fn get_cluster_by_id(conn: &Connection, id: &ClusterId) -> anyhow::Result<Option<Cluster>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, topology, conflict_strategy, created_at, updated_at
         FROM clusters WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id.0.to_string()])?;
    match rows.next()? {
        Some(row) => {
            let id_str: String = row.get(0)?;
            let topo_str: String = row.get(2)?;
            let cs_str: String = row.get(3)?;
            let created_str: String = row.get(4)?;
            let updated_str: String = row.get(5)?;
            Ok(Some(Cluster {
                id: ClusterId::from_uuid(Uuid::parse_str(&id_str)?),
                name: row.get(1)?,
                topology: topo_str.parse().unwrap_or(Topology::Mesh),
                conflict_strategy: cs_str.parse().unwrap_or(ConflictStrategy::NewestWins),
                created_at: parse_dt(&created_str),
                updated_at: parse_dt(&updated_str),
            }))
        }
        None => Ok(None),
    }
}

pub fn list_clusters(conn: &Connection) -> anyhow::Result<Vec<Cluster>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, topology, conflict_strategy, created_at, updated_at
         FROM clusters ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        let id_str: String = row.get(0)?;
        let topo_str: String = row.get(2)?;
        let cs_str: String = row.get(3)?;
        let created_str: String = row.get(4)?;
        let updated_str: String = row.get(5)?;
        Ok(Cluster {
            id: ClusterId::from_uuid(Uuid::parse_str(&id_str).unwrap_or_default()),
            name: row.get(1)?,
            topology: topo_str.parse().unwrap_or(Topology::Mesh),
            conflict_strategy: cs_str.parse().unwrap_or(ConflictStrategy::NewestWins),
            created_at: parse_dt(&created_str),
            updated_at: parse_dt(&updated_str),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn delete_cluster(conn: &Connection, id: &ClusterId) -> anyhow::Result<()> {
    conn.execute("DELETE FROM clusters WHERE id = ?1", params![id.0.to_string()])?;
    Ok(())
}

// ── Drives ──

pub fn insert_drive(conn: &Connection, drive: &Drive) -> anyhow::Result<()> {
    let (id_type, id_value) = match &drive.identity {
        DriveIdentity::Hardware { serial } => ("hardware", serial.clone()),
        DriveIdentity::Synthetic { id } => ("synthetic", id.clone()),
    };
    conn.execute(
        "INSERT INTO drives (id, identity_type, identity_value, label, mount_point, sync_root, cluster_id, role, is_primary, total_bytes, free_bytes, last_seen, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            drive.id.0.to_string(),
            id_type,
            id_value,
            drive.label,
            drive.mount_point.to_string_lossy().to_string(),
            drive.sync_root.as_ref().map(|p| p.to_string_lossy().to_string()),
            drive.cluster_id.as_ref().map(|c| c.0.to_string()),
            drive.role.to_string(),
            drive.is_primary as i32,
            drive.total_bytes.map(|b| b as i64),
            drive.free_bytes.map(|b| b as i64),
            fmt_dt(&drive.last_seen),
            fmt_dt(&drive.created_at),
        ],
    )?;
    Ok(())
}

pub fn get_drive_by_identity(conn: &Connection, identity: &DriveIdentity) -> anyhow::Result<Option<Drive>> {
    let (id_type, id_value) = match identity {
        DriveIdentity::Hardware { serial } => ("hardware", serial.as_str()),
        DriveIdentity::Synthetic { id } => ("synthetic", id.as_str()),
    };
    let mut stmt = conn.prepare(
        "SELECT id, identity_type, identity_value, label, mount_point, sync_root, cluster_id, role, is_primary, total_bytes, free_bytes, last_seen, created_at
         FROM drives WHERE identity_type = ?1 AND identity_value = ?2",
    )?;
    let mut rows = stmt.query(params![id_type, id_value])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_drive(row)?)),
        None => Ok(None),
    }
}

pub fn list_drives_for_cluster(conn: &Connection, cluster_id: &ClusterId) -> anyhow::Result<Vec<Drive>> {
    let mut stmt = conn.prepare(
        "SELECT id, identity_type, identity_value, label, mount_point, sync_root, cluster_id, role, is_primary, total_bytes, free_bytes, last_seen, created_at
         FROM drives WHERE cluster_id = ?1 ORDER BY created_at",
    )?;
    let rows = stmt.query_map(params![cluster_id.0.to_string()], |row| row_to_drive(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn list_all_drives(conn: &Connection) -> anyhow::Result<Vec<Drive>> {
    let mut stmt = conn.prepare(
        "SELECT id, identity_type, identity_value, label, mount_point, sync_root, cluster_id, role, is_primary, total_bytes, free_bytes, last_seen, created_at
         FROM drives ORDER BY created_at",
    )?;
    let rows = stmt.query_map([], |row| row_to_drive(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn update_drive_cluster(conn: &Connection, drive_id: &DriveId, cluster_id: Option<&ClusterId>) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE drives SET cluster_id = ?1 WHERE id = ?2",
        params![
            cluster_id.map(|c| c.0.to_string()),
            drive_id.0.to_string(),
        ],
    )?;
    Ok(())
}

pub fn delete_drive(conn: &Connection, drive_id: &DriveId) -> anyhow::Result<()> {
    conn.execute("DELETE FROM drives WHERE id = ?1", params![drive_id.0.to_string()])?;
    Ok(())
}

pub fn update_drive_sync_root(
    conn: &Connection,
    drive_id: &DriveId,
    sync_root: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE drives SET sync_root = ?1 WHERE id = ?2",
        params![
            sync_root.map(|p| p.to_string_lossy().to_string()),
            drive_id.0.to_string(),
        ],
    )?;
    Ok(())
}

fn row_to_drive(row: &rusqlite::Row) -> rusqlite::Result<Drive> {
    let id_str: String = row.get(0)?;
    let id_type: String = row.get(1)?;
    let id_value: String = row.get(2)?;
    let label: Option<String> = row.get(3)?;
    let mount_point: String = row.get(4)?;
    let sync_root: Option<String> = row.get(5)?;
    let cluster_id: Option<String> = row.get(6)?;
    let role_str: String = row.get(7)?;
    let is_primary: i32 = row.get(8)?;
    let total_bytes: Option<i64> = row.get(9)?;
    let free_bytes: Option<i64> = row.get(10)?;
    let last_seen_str: String = row.get(11)?;
    let created_str: String = row.get(12)?;

    let identity = match id_type.as_str() {
        "hardware" => DriveIdentity::Hardware { serial: id_value },
        _ => DriveIdentity::Synthetic { id: id_value },
    };

    Ok(Drive {
        id: DriveId::from_uuid(Uuid::parse_str(&id_str).unwrap_or_default()),
        identity,
        label,
        mount_point: mount_point.into(),
        sync_root: sync_root.map(Into::into),
        cluster_id: cluster_id
            .and_then(|s| Uuid::parse_str(&s).ok())
            .map(ClusterId::from_uuid),
        role: role_str.parse().unwrap_or(DriveRole::Normal),
        is_primary: is_primary != 0,
        total_bytes: total_bytes.map(|b| b as u64),
        free_bytes: free_bytes.map(|b| b as u64),
        last_seen: parse_dt(&last_seen_str),
        created_at: parse_dt(&created_str),
    })
}

// ── File Index ──

pub fn upsert_file_entry(conn: &Connection, entry: &FileEntry) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO file_index (rel_path, drive_id, is_dir, size, mtime, xxh3_hash, sha256_hash, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            entry.rel_path.to_string_lossy().to_string(),
            entry.drive_id.0.to_string(),
            entry.is_dir as i32,
            entry.size as i64,
            fmt_dt(&entry.mtime),
            entry.xxh3_hash,
            entry.sha256_hash,
            fmt_dt(&entry.indexed_at),
        ],
    )?;
    Ok(())
}

pub fn get_file_entries_for_drive(conn: &Connection, drive_id: &DriveId) -> anyhow::Result<Vec<FileEntry>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, drive_id, is_dir, size, mtime, xxh3_hash, sha256_hash, indexed_at
         FROM file_index WHERE drive_id = ?1 ORDER BY rel_path",
    )?;
    let rows = stmt.query_map(params![drive_id.0.to_string()], |row| {
        let rel_path: String = row.get(0)?;
        let drive_id_str: String = row.get(1)?;
        let is_dir: i32 = row.get(2)?;
        let size: i64 = row.get(3)?;
        let mtime_str: String = row.get(4)?;
        let xxh3: Option<String> = row.get(5)?;
        let sha256: Option<String> = row.get(6)?;
        let indexed_str: String = row.get(7)?;
        Ok(FileEntry {
            rel_path: rel_path.into(),
            drive_id: DriveId::from_uuid(Uuid::parse_str(&drive_id_str).unwrap_or_default()),
            is_dir: is_dir != 0,
            size: size as u64,
            mtime: parse_dt(&mtime_str),
            xxh3_hash: xxh3,
            sha256_hash: sha256,
            indexed_at: parse_dt(&indexed_str),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn clear_file_index_for_drive(conn: &Connection, drive_id: &DriveId) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM file_index WHERE drive_id = ?1",
        params![drive_id.0.to_string()],
    )?;
    Ok(())
}

// ── Hash Cache ──

pub fn upsert_hash_cache(conn: &Connection, entry: &HashCacheEntry) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO hash_cache (rel_path, drive_id, size, mtime, xxh3_hash, sha256_hash, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            entry.rel_path.to_string_lossy().to_string(),
            entry.drive_id.0.to_string(),
            entry.size as i64,
            fmt_dt(&entry.mtime),
            entry.xxh3_hash,
            entry.sha256_hash,
            fmt_dt(&entry.cached_at),
        ],
    )?;
    Ok(())
}

pub fn get_hash_cache_entry(
    conn: &Connection,
    drive_id: &DriveId,
    rel_path: &str,
) -> anyhow::Result<Option<HashCacheEntry>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, drive_id, size, mtime, xxh3_hash, sha256_hash, cached_at
         FROM hash_cache WHERE drive_id = ?1 AND rel_path = ?2",
    )?;
    let mut rows = stmt.query(params![drive_id.0.to_string(), rel_path])?;
    match rows.next()? {
        Some(row) => {
            let rel_path: String = row.get(0)?;
            let drive_id_str: String = row.get(1)?;
            let size: i64 = row.get(2)?;
            let mtime_str: String = row.get(3)?;
            let xxh3: String = row.get(4)?;
            let sha256: Option<String> = row.get(5)?;
            let cached_str: String = row.get(6)?;
            Ok(Some(HashCacheEntry {
                rel_path: rel_path.into(),
                drive_id: DriveId::from_uuid(Uuid::parse_str(&drive_id_str).unwrap_or_default()),
                size: size as u64,
                mtime: parse_dt(&mtime_str),
                xxh3_hash: xxh3,
                sha256_hash: sha256,
                cached_at: parse_dt(&cached_str),
            }))
        }
        None => Ok(None),
    }
}

// ── Sync History ──

pub fn insert_sync_record(conn: &Connection, record: &SyncRecord) -> anyhow::Result<()> {
    let errors_json = serde_json::to_string(&record.errors).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO sync_history (id, cluster_id, started_at, finished_at, files_synced, bytes_transferred, conflicts_resolved, errors, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            record.id.to_string(),
            record.cluster_id.0.to_string(),
            fmt_dt(&record.started_at),
            fmt_dt(&record.finished_at),
            record.files_synced as i64,
            record.bytes_transferred as i64,
            record.conflicts_resolved as i64,
            errors_json,
            record.status.to_string(),
        ],
    )?;
    Ok(())
}

pub fn list_sync_history(conn: &Connection, cluster_id: &ClusterId, limit: u32) -> anyhow::Result<Vec<SyncRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, cluster_id, started_at, finished_at, files_synced, bytes_transferred, conflicts_resolved, errors, status
         FROM sync_history WHERE cluster_id = ?1 ORDER BY started_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![cluster_id.0.to_string(), limit], |row| {
        let id_str: String = row.get(0)?;
        let cluster_str: String = row.get(1)?;
        let started_str: String = row.get(2)?;
        let finished_str: String = row.get(3)?;
        let files: i64 = row.get(4)?;
        let bytes: i64 = row.get(5)?;
        let conflicts: i64 = row.get(6)?;
        let errors_str: String = row.get(7)?;
        let status_str: String = row.get(8)?;
        let errors: Vec<String> = serde_json::from_str(&errors_str).unwrap_or_default();
        let status = match status_str.as_str() {
            "success" => SyncStatus::Success,
            "partial_success" => SyncStatus::PartialSuccess,
            _ => SyncStatus::Failed,
        };
        Ok(SyncRecord {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            cluster_id: ClusterId::from_uuid(Uuid::parse_str(&cluster_str).unwrap_or_default()),
            started_at: parse_dt(&started_str),
            finished_at: parse_dt(&finished_str),
            files_synced: files as u64,
            bytes_transferred: bytes as u64,
            conflicts_resolved: conflicts as u64,
            errors,
            status,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ── Archives ──

pub fn insert_archive(conn: &Connection, entry: &ArchiveEntry) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO archives (id, original_path, archive_path, drive_id, original_size, compressed_size, compression, xxh3_hash, reason, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            entry.id.to_string(),
            entry.original_path.to_string_lossy().to_string(),
            entry.archive_path.to_string_lossy().to_string(),
            entry.drive_id.0.to_string(),
            entry.original_size as i64,
            entry.compressed_size as i64,
            entry.compression.to_string(),
            entry.xxh3_hash,
            entry.reason.to_string(),
            fmt_dt(&entry.archived_at),
        ],
    )?;
    Ok(())
}

pub fn list_archives_for_path(conn: &Connection, original_path: &str) -> anyhow::Result<Vec<ArchiveEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, original_path, archive_path, drive_id, original_size, compressed_size, compression, xxh3_hash, reason, archived_at
         FROM archives WHERE original_path = ?1 ORDER BY archived_at DESC",
    )?;
    let rows = stmt.query_map(params![original_path], |row| row_to_archive(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn list_archives_for_drive(conn: &Connection, drive_id: &DriveId) -> anyhow::Result<Vec<ArchiveEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, original_path, archive_path, drive_id, original_size, compressed_size, compression, xxh3_hash, reason, archived_at
         FROM archives WHERE drive_id = ?1 ORDER BY archived_at DESC",
    )?;
    let rows = stmt.query_map(params![drive_id.0.to_string()], |row| row_to_archive(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn delete_archive(conn: &Connection, id: &Uuid) -> anyhow::Result<()> {
    conn.execute("DELETE FROM archives WHERE id = ?1", params![id.to_string()])?;
    Ok(())
}

pub fn get_total_archive_size(conn: &Connection, drive_id: &DriveId) -> anyhow::Result<u64> {
    let size: i64 = conn.query_row(
        "SELECT COALESCE(SUM(compressed_size), 0) FROM archives WHERE drive_id = ?1",
        params![drive_id.0.to_string()],
        |row| row.get(0),
    )?;
    Ok(size as u64)
}

fn row_to_archive(row: &rusqlite::Row) -> rusqlite::Result<ArchiveEntry> {
    let id_str: String = row.get(0)?;
    let original_path: String = row.get(1)?;
    let archive_path: String = row.get(2)?;
    let drive_id_str: String = row.get(3)?;
    let original_size: i64 = row.get(4)?;
    let compressed_size: i64 = row.get(5)?;
    let compression_str: String = row.get(6)?;
    let xxh3: String = row.get(7)?;
    let reason_str: String = row.get(8)?;
    let archived_str: String = row.get(9)?;

    let compression = match compression_str.as_str() {
        "none" => CompressionFormat::None,
        _ => CompressionFormat::Zstd,
    };
    let reason = match reason_str.as_str() {
        "before_overwrite" => ArchiveReason::BeforeOverwrite,
        "before_delete" => ArchiveReason::BeforeDelete,
        _ => ArchiveReason::Manual,
    };

    Ok(ArchiveEntry {
        id: Uuid::parse_str(&id_str).unwrap_or_default(),
        original_path: original_path.into(),
        archive_path: archive_path.into(),
        drive_id: DriveId::from_uuid(Uuid::parse_str(&drive_id_str).unwrap_or_default()),
        original_size: original_size as u64,
        compressed_size: compressed_size as u64,
        compression,
        xxh3_hash: xxh3,
        reason,
        archived_at: parse_dt(&archived_str),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory_db;
    use diffr_core::models::cluster::{ConflictStrategy, Topology};

    #[test]
    fn test_cluster_crud() {
        let conn = open_memory_db().unwrap();
        let cluster = Cluster::new("test".to_string(), Topology::Mesh, ConflictStrategy::NewestWins);
        insert_cluster(&conn, &cluster).unwrap();

        let found = get_cluster_by_name(&conn, "test").unwrap().unwrap();
        assert_eq!(found.name, "test");
        assert_eq!(found.topology, Topology::Mesh);

        let all = list_clusters(&conn).unwrap();
        assert_eq!(all.len(), 1);

        delete_cluster(&conn, &cluster.id).unwrap();
        let gone = get_cluster_by_name(&conn, "test").unwrap();
        assert!(gone.is_none());
    }

    #[test]
    fn test_drive_crud() {
        let conn = open_memory_db().unwrap();
        let drive = Drive::new(
            DriveIdentity::new_hardware("ABC123".to_string()),
            "/mnt/usb".into(),
        );
        insert_drive(&conn, &drive).unwrap();

        let found = get_drive_by_identity(&conn, &DriveIdentity::new_hardware("ABC123".to_string()))
            .unwrap()
            .unwrap();
        assert_eq!(found.identity.identity_string(), "ABC123");

        let all = list_all_drives(&conn).unwrap();
        assert_eq!(all.len(), 1);
    }
}
