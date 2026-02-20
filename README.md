# Diffr

Local-first CLI tool for diffing, syncing, and archiving files across physical drives.

## Quick Start

```bash
# Build from source
cargo build --release -p diffr-cli

# Initialize global config (~/.diffr/)
diffr config init

# Initialize sync repos on two drives
diffr init /mnt/usb-a/projects
diffr init /mnt/usb-b/projects

# Create a cluster and add the drives
diffr cluster create my-cluster
diffr drive add usb-a --cluster my-cluster --path /mnt/usb-a/projects
diffr drive add usb-b --cluster my-cluster --path /mnt/usb-b/projects

# Preview what would sync, then sync for real
diffr sync my-cluster --dry-run
diffr sync my-cluster
```

## Usage

### Configuration

```bash
diffr config init     # Create ~/.diffr/ with default config.toml and database
diffr config show     # Print current configuration
```

Global state lives in `~/.diffr/`:
- `config.toml` -- default topology, conflict strategy, retention policy
- `diffr.db` -- SQLite database (clusters, drives, file index, sync history, archives)

### Repo Initialization

```bash
diffr init [path]     # Defaults to current directory
```

Creates a `.diffr/repo.toml` marker at the given path, designating it as a **sync root**. Only files under this directory participate in scanning and syncing. A `.diffrignore` template is also created if one doesn't exist.

### Clusters

A cluster is a named group of drives that sync together.

```bash
diffr cluster create <name> [--topology mesh|primary-replica] [--conflict newest-wins|keep-both|interactive]
diffr cluster list
diffr cluster info <name>
diffr cluster remove <name>
```

### Drives

```bash
diffr drive scan                              # Detect connected drives
diffr drive add <identity> --cluster <name>   # Add by hardware serial (whole-drive sync)
diffr drive add <identity> --cluster <name> --path /mnt/usb/repo  # Scoped to a diffr repo
diffr drive add <identity> --cluster <name> --role archive-only   # Archive-only role
diffr drive list
diffr drive info <identity>
diffr drive remove <identity>
```

Drive roles:
- **normal** -- full sync participant (default)
- **archive-assist** -- syncs files and stores extra archive copies
- **archive-only** -- stores archives only, does not participate in active sync

When `--path` is provided, the drive's sync scope is limited to that directory (must be initialized with `diffr init` first). Without `--path`, the entire mount point is used.

### Syncing

```bash
diffr sync <cluster> [--dry-run] [--verify] [--no-archive]
```

- `--dry-run` -- show what would happen without copying or deleting
- `--verify` -- check file integrity with SHA-256 after each copy
- `--no-archive` -- skip archiving files before overwrite/delete

### Archives

Files are archived (zstd-compressed) before being overwritten or deleted during sync.

```bash
diffr archive list --path <file>       # List archived versions of a file
diffr archive list --drive <identity>  # List all archives on a drive
diffr archive restore <id> [--dest <path>]
diffr archive prune <drive-identity>   # Enforce retention policy
```

Retention policy (configured in `config.toml`):
- `max_versions` -- max archived versions per file
- `max_age_days` -- delete archives older than N days
- `max_total_bytes` -- cap total archive size per drive

### Status & History

```bash
diffr status [cluster]          # Show cluster overview, drive connectivity, last sync
diffr history <cluster> [--limit N]
```

### Global Flags

```bash
diffr --json <command>    # Machine-readable JSON output for all commands
```

## Architecture

Diffr is a Rust workspace split into focused crates:

| Crate | Purpose |
|---|---|
| `diffr-core` | Shared models (Drive, Cluster, FileEntry, Archive), config, error types |
| `diffr-discovery` | Platform-specific drive detection (serial numbers, mount points) |
| `diffr-scan` | Directory walker with `.diffrignore` support, xxh3/SHA-256 hashing, hash cache |
| `diffr-db` | SQLite schema, migrations, and CRUD operations |
| `diffr-sync` | Diff engine, topology-aware sync plan generation, atomic file copy executor |
| `diffr-archive` | Zstd-compressed file archiving, restore with hash verification, retention enforcement |
| `diffr-cli` | Clap-based CLI wiring all crates together |

### Data Layout

```
~/.diffr/                    # Global (one per machine)
  config.toml
  diffr.db

/mnt/usb/projects/           # Per-drive sync root
  .diffr/
    repo.toml                # Repo marker with init timestamp
    archive/                 # Versioned backups (zstd-compressed)
  .diffrignore               # Gitignore-style exclusion patterns
  <your files>/
```

## Roadmap

- [ ] **Conflict resolution UI** -- interactive TUI for choosing between conflicting file versions
- [ ] **Watch mode** -- filesystem event monitoring for real-time incremental sync
- [ ] **Encryption at rest** -- encrypt archives and optionally synced files
- [ ] **Partial sync** -- sync individual files or subdirectories within a repo
- [ ] **Network sync** -- extend beyond local drives to LAN/remote targets
- [ ] **Deduplication** -- content-addressable storage to avoid redundant copies across drives
- [ ] **Plugin system** -- user-defined pre/post-sync hooks
- [ ] **Cross-platform drive identity** -- improve synthetic ID persistence on macOS/Linux
