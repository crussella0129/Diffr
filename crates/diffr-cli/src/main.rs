mod commands;

use clap::Parser;

#[derive(Parser)]
#[command(name = "diffr", version, about = "Local disk diff & sync management")]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,

    /// Output as JSON instead of human-readable text
    #[arg(long, global = true)]
    json: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    commands::run(cli.command, cli.json)
}
