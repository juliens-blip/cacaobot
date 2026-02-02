//! Palm Oil Trading Bot - Main Entry Point
//!
//! Uses TradingBot runtime from bot.rs

use palm_oil_bot::bot::TradingBot;
use palm_oil_bot::config::Config;
use palm_oil_bot::modules::security::SecretValidator;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("palm_oil_bot=info".parse()?)
                .add_directive("reqwest=warn".parse()?)
        )
        .init();

    info!("========================================");
    info!("  Palm Oil Trading Bot v0.1.0");
    info!("  Symbol: FCPO (Palm Oil CFD)");
    info!("  Strategy: RSI + Sentiment Analysis");
    info!("========================================");

    // Load .env before validation
    dotenvy::dotenv().ok();

    // Validate secrets before loading config
    SecretValidator::validate_required_secrets();

    let config = Config::from_env()?;
    config.validate()?;

    info!("Configuration loaded:");
    info!("  Server: {}:{}", config.ctrader.server, config.ctrader.port);
    info!("  Account: {}", config.ctrader.account_id);
    info!("  Dry Run: {}", config.bot.dry_run);
    info!("  Cycle Interval: {}s", config.bot.cycle_interval_secs);

    let mut bot = TradingBot::new(config.clone())?;

    if let Err(err) = bot.run().await {
        error!("Bot stopped with error: {}", err);
        return Err(err.into());
    }

    Ok(())
}
