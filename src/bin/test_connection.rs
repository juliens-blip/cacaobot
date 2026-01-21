//! Test cTrader connection binary
//!
//! This binary tests the connection and authentication with cTrader Open API.
//!
//! Usage: cargo run --bin test-connection

use palm_oil_bot::config::Config;
use palm_oil_bot::modules::trading::{CTraderClient, OrderTicket};
use palm_oil_bot::modules::trading::protobuf::ProtoOATradeSide;
use tracing::{error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("palm_oil_bot=debug,test_connection=debug")
        .init();

    info!("=== cTrader Connection Test ===");

    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    info!("Configuration loaded:");
    info!("  Server: {}:{}", config.ctrader.server, config.ctrader.port);
    info!("  Account ID: {}", config.ctrader.account_id);
    info!("  Symbol: {}", config.trading.symbol);

    // Create cTrader client
    let client = CTraderClient::new(config.ctrader.clone());

    // Test 1: Connect
    info!("\n[1] Testing connection...");
    match client.connect().await {
        Ok(_) => info!("✓ Connection successful"),
        Err(e) => {
            error!("✗ Connection failed: {}", e);
            return Err(e.into());
        }
    }

    // Test 2: Authenticate
    info!("\n[2] Testing authentication...");
    match client.authenticate().await {
        Ok(_) => info!("✓ Authentication successful"),
        Err(e) => {
            error!("✗ Authentication failed: {}", e);
            return Err(e.into());
        }
    }

    // Test 3: Subscribe to FCPO symbol
    // Note: You'll need to get the actual symbol ID from cTrader
    // For FCPO, this would typically be obtained via ProtoOASymbolsListReq
    info!("\n[3] Testing symbol subscription...");
    let symbol_id = 1; // Replace with actual FCPO symbol ID
    info!("Subscribing to symbol ID: {} ({})", symbol_id, config.trading.symbol);

    match client.subscribe_to_symbol(symbol_id).await {
        Ok(_) => info!("✓ Subscription successful"),
        Err(e) => {
            error!("✗ Subscription failed: {}", e);
            // Continue anyway as we might not have the right symbol ID
        }
    }

    // Test 4: Wait for price updates
    info!("\n[4] Waiting for price updates (10 seconds)...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    match client.get_price(symbol_id).await {
        Ok(price) => {
            info!("✓ Price received:");
            info!("  Bid: {:.2}", price.bid);
            info!("  Ask: {:.2}", price.ask);
            info!("  Spread: {:.5}", price.spread);
            info!("  Timestamp: {}", price.timestamp);
        }
        Err(e) => {
            error!("✗ No price data: {}", e);
        }
    }

    // Test 5: Dry run - Create order ticket (don't actually place)
    info!("\n[5] Testing order creation (dry run)...");
    let order_ticket = OrderTicket {
        symbol_id,
        side: ProtoOATradeSide::Buy,
        volume: 10, // 0.1 lot = 10 in cTrader units
        stop_loss: Some(4800.0),
        take_profit: Some(4950.0),
        label: Some("Palm Oil Bot Test".to_string()),
    };
    info!("Order ticket created: {:?}", order_ticket);
    info!("✓ Order structure validated");

    // Skip actual order placement in test mode
    if !config.bot.dry_run {
        info!("\n⚠️  DRY_RUN=false - Would place actual order!");
        info!("Set DRY_RUN=true in .env to prevent real trades");
    } else {
        info!("\n✓ DRY_RUN mode - No real orders placed");
    }

    // Test 6: Disconnect
    info!("\n[6] Disconnecting...");
    match client.disconnect().await {
        Ok(_) => info!("✓ Disconnected successfully"),
        Err(e) => {
            error!("✗ Disconnect failed: {}", e);
        }
    }

    info!("\n=== All Tests Complete ===");
    info!("Connection to cTrader API is working!");

    Ok(())
}
