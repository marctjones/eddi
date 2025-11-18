// eddi message server binary

use clap::Parser;
use eddi::msgserver::{MsgSrvCli, execute_command};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "eddi=info,warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse CLI arguments
    let cli = MsgSrvCli::parse();

    // Execute command
    if let Err(e) = execute_command(cli.command).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
