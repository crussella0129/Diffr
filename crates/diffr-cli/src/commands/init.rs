use chrono::Utc;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitArgs {
    /// Path to initialize as a diffr repo (defaults to current directory)
    path: Option<PathBuf>,
}

/// Canonicalize a path, stripping the `\\?\` extended-path prefix on Windows.
pub fn simplified_canonicalize(path: &std::path::Path) -> std::io::Result<PathBuf> {
    let canon = std::fs::canonicalize(path)?;
    #[cfg(windows)]
    {
        let s = canon.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(stripped));
        }
    }
    Ok(canon)
}

pub fn run(args: InitArgs) -> anyhow::Result<()> {
    let raw_path = args.path.unwrap_or_else(|| PathBuf::from("."));
    // Create the directory if it doesn't exist (like git init)
    std::fs::create_dir_all(&raw_path)?;
    let path = simplified_canonicalize(&raw_path)?;

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
