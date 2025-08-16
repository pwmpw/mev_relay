use anyhow::Result;
use mev_relay::{
    infrastructure::{config::Config, app::MevRelay, shutdown::ShutdownSignal},
    shared::Result as MevResult,
};
use std::process;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging()?;

    info!("Starting MEV Relay with Domain-Driven Design Architecture...");

    // Load configuration
    let config = match Config::load() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    // Create shutdown signal
    let shutdown = ShutdownSignal::new();

    // Create and run MEV relay application
    let relay = match MevRelay::new(config, shutdown.clone()) {
        Ok(relay) => {
            info!("MEV Relay application initialized successfully");
            relay
        }
        Err(e) => {
            error!("Failed to initialize MEV Relay application: {}", e);
            process::exit(1);
        }
    };

    // Run the application
    if let Err(e) = relay.run().await {
        error!("MEV Relay application failed: {}", e);
        process::exit(1);
    }

    info!("MEV Relay application shutdown complete");
    Ok(())
}

fn init_logging() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("mev_relay=info,tower=info"));

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
} 