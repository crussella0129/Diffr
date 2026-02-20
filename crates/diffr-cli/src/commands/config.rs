use clap::Subcommand;
use diffr_core::config::DiffrConfig;

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Initialize Diffr (create ~/.diffr/ with default config)
    Init,
    /// Show current configuration
    Show,
}

pub fn run(action: ConfigAction) -> anyhow::Result<()> {
    match action {
        ConfigAction::Init => {
            let home = DiffrConfig::init()?;
            println!("Diffr initialized at {}", home.display());

            // Also ensure database exists
            let db_path = DiffrConfig::db_path()?;
            let _conn = diffr_db::open_db(&db_path)?;
            println!("Database created at {}", db_path.display());

            Ok(())
        }
        ConfigAction::Show => {
            let config = DiffrConfig::load()?;
            let toml = toml::to_string_pretty(&config)?;
            println!("{}", toml);
            Ok(())
        }
    }
}
