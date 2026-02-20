use chrono::Utc;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitArgs {
    /// Path to initialize as a diffr repo (defaults to current directory)
    path: Option<PathBuf>,
}

pub fn run(args: InitArgs) -> anyhow::Result<()> {
    let raw_path = args.path.unwrap_or_else(|| PathBuf::from("."));
    let path = std::fs::canonicalize(&raw_path)
        .map_err(|_| anyhow::anyhow!("path does not exist: {}", raw_path.display()))?;

    let diffr_dir = path.join(".diffr");
    let repo_toml = diffr_dir.join("repo.toml");

    if repo_toml.exists() {
        anyhow::bail!(
            "already initialized: {} exists",
            repo_toml.display()
        );
    }

    std::fs::create_dir_all(&diffr_dir)?;

    let content = format!(
        "[repo]\ninitialized_at = \"{}\"\n",
        Utc::now().to_rfc3339()
    );
    std::fs::write(&repo_toml, content)?;

    // Create .diffrignore template if absent
    let ignore_path = path.join(".diffrignore");
    if !ignore_path.exists() {
        std::fs::write(
            &ignore_path,
            "# Diffr ignore patterns (one per line, gitignore syntax)\n.diffr/\n",
        )?;
    }

    println!("Initialized diffr repo at {}", path.display());
    Ok(())
}
