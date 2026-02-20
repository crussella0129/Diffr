use chrono::Utc;
use diffr_core::models::cluster::ConflictStrategy;
use diffr_core::models::drive::Drive;
use diffr_core::models::sync_state::{ConflictResolution, SyncOp, SyncOpKind};
use std::io::{self, Write};
use std::path::PathBuf;
use uuid::Uuid;

use crate::diff::DiffEntry;

/// Resolve a conflict according to the configured strategy.
pub fn resolve_conflict(
    strategy: &ConflictStrategy,
    entry: &DiffEntry,
    left_drive: &Drive,
    right_drive: &Drive,
) -> anyhow::Result<(Vec<SyncOp>, ConflictResolution)> {
    match strategy {
        ConflictStrategy::NewestWins => {
            resolve_newest_wins(entry, left_drive, right_drive)
        }
        ConflictStrategy::KeepBoth => {
            resolve_keep_both(entry, left_drive, right_drive)
        }
        ConflictStrategy::Interactive => {
            resolve_interactive(entry, left_drive, right_drive)
        }
    }
}

fn resolve_newest_wins(
    entry: &DiffEntry,
    left_drive: &Drive,
    right_drive: &Drive,
) -> anyhow::Result<(Vec<SyncOp>, ConflictResolution)> {
    let left_mtime = entry.left.as_ref().map(|e| e.mtime);
    let right_mtime = entry.right.as_ref().map(|e| e.mtime);

    let (winner, loser, size) = match (left_mtime, right_mtime) {
        (Some(l), Some(r)) if l >= r => {
            let size = entry.left.as_ref().map(|e| e.size).unwrap_or(0);
            (left_drive, right_drive, size)
        }
        _ => {
            let size = entry.right.as_ref().map(|e| e.size).unwrap_or(0);
            (right_drive, left_drive, size)
        }
    };

    let op = SyncOp {
        id: Uuid::now_v7(),
        kind: SyncOpKind::Overwrite,
        rel_path: entry.rel_path.clone(),
        source_drive: Some(winner.id.clone()),
        target_drive: loser.id.clone(),
        size_bytes: size,
    };

    let resolution = ConflictResolution {
        rel_path: entry.rel_path.clone(),
        winner_drive: winner.id.clone(),
        loser_drive: loser.id.clone(),
        strategy_used: "newest_wins".to_string(),
        resolved_at: Utc::now(),
    };

    Ok((vec![op], resolution))
}

fn resolve_keep_both(
    entry: &DiffEntry,
    left_drive: &Drive,
    right_drive: &Drive,
) -> anyhow::Result<(Vec<SyncOp>, ConflictResolution)> {
    // Generate a conflict name: file.txt -> file.conflict-<drive-label>.txt
    let conflict_name = generate_conflict_name(&entry.rel_path, right_drive);

    let left_size = entry.left.as_ref().map(|e| e.size).unwrap_or(0);
    let right_size = entry.right.as_ref().map(|e| e.size).unwrap_or(0);

    let ops = vec![
        // Copy left version to right (overwrite)
        SyncOp {
            id: Uuid::now_v7(),
            kind: SyncOpKind::Overwrite,
            rel_path: entry.rel_path.clone(),
            source_drive: Some(left_drive.id.clone()),
            target_drive: right_drive.id.clone(),
            size_bytes: left_size,
        },
        // Copy right version to left under conflict name
        SyncOp {
            id: Uuid::now_v7(),
            kind: SyncOpKind::CopyNew,
            rel_path: conflict_name.clone(),
            source_drive: Some(right_drive.id.clone()),
            target_drive: left_drive.id.clone(),
            size_bytes: right_size,
        },
        // Also keep conflict name on right
        SyncOp {
            id: Uuid::now_v7(),
            kind: SyncOpKind::CopyNew,
            rel_path: conflict_name,
            source_drive: Some(right_drive.id.clone()),
            target_drive: right_drive.id.clone(),
            size_bytes: right_size,
        },
    ];

    let resolution = ConflictResolution {
        rel_path: entry.rel_path.clone(),
        winner_drive: left_drive.id.clone(),
        loser_drive: right_drive.id.clone(),
        strategy_used: "keep_both".to_string(),
        resolved_at: Utc::now(),
    };

    Ok((ops, resolution))
}

fn resolve_interactive(
    entry: &DiffEntry,
    left_drive: &Drive,
    right_drive: &Drive,
) -> anyhow::Result<(Vec<SyncOp>, ConflictResolution)> {
    println!("\nConflict: {}", entry.rel_path.display());
    if let Some(ref left) = entry.left {
        println!(
            "  [L] {} — size: {}, modified: {}",
            left_drive.mount_point.display(),
            left.size,
            left.mtime
        );
    }
    if let Some(ref right) = entry.right {
        println!(
            "  [R] {} — size: {}, modified: {}",
            right_drive.mount_point.display(),
            right.size,
            right.mtime
        );
    }
    print!("Choose [L]eft, [R]ight, or [B]oth: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim().to_lowercase();

    match choice.as_str() {
        "l" | "left" => resolve_newest_wins_with_winner(entry, left_drive, right_drive),
        "r" | "right" => resolve_newest_wins_with_winner(entry, right_drive, left_drive),
        "b" | "both" => resolve_keep_both(entry, left_drive, right_drive),
        _ => {
            println!("Invalid choice, defaulting to keep-both");
            resolve_keep_both(entry, left_drive, right_drive)
        }
    }
}

fn resolve_newest_wins_with_winner(
    entry: &DiffEntry,
    winner: &Drive,
    loser: &Drive,
) -> anyhow::Result<(Vec<SyncOp>, ConflictResolution)> {
    let size = entry
        .left
        .as_ref()
        .or(entry.right.as_ref())
        .map(|e| e.size)
        .unwrap_or(0);

    let op = SyncOp {
        id: Uuid::now_v7(),
        kind: SyncOpKind::Overwrite,
        rel_path: entry.rel_path.clone(),
        source_drive: Some(winner.id.clone()),
        target_drive: loser.id.clone(),
        size_bytes: size,
    };

    let resolution = ConflictResolution {
        rel_path: entry.rel_path.clone(),
        winner_drive: winner.id.clone(),
        loser_drive: loser.id.clone(),
        strategy_used: "interactive".to_string(),
        resolved_at: Utc::now(),
    };

    Ok((vec![op], resolution))
}

/// Generate a conflict-renamed path.
/// `file.txt` -> `file.conflict-<label>.txt`
fn generate_conflict_name(path: &PathBuf, drive: &Drive) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    let label = drive
        .label
        .as_deref()
        .unwrap_or("unknown");
    let conflict_name = format!("{}.conflict-{}{}", stem, label, ext);
    path.with_file_name(conflict_name)
}
