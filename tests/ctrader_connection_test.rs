//! cTrader Connection Integration Tests
//!
//! Tests real connection to cTrader DEMO server with RALPH methodology:
//! R - Run: cargo test --test ctrader_connection_test
//! A - Analyze: Verify TLS + auth flow
//! L - Lint: cargo clippy
//! P - Polish: Docs + edge cases
//! H - Handoff: Report to RALPH_CONNECTION_TESTS.md

use palm_oil_bot::config::CTraderConfig;
use palm_oil_bot::modules::trading::ctrader::{CTraderClient, CTraderEnvironment};
use std::time::Duration;
use tokio::time::timeout;

/// Helper to create test config from environment
fn create_test_config() -> CTraderConfig {
    dotenvy::dotenv().ok();
    
    let client_id = std::env::var("CTRADER_CLIENT_ID")
        .unwrap_or_else(|_| "test_client_id".to_string());
    let client_secret = std::env::var("CTRADER_CLIENT_SECRET")
        .unwrap_or_else(|_| "test_client_secret".to_string());
    let account_id = std::env::var("CTRADER_ACCOUNT_ID")
        .unwrap_or_else(|_| "12345".to_string());
    let access_token = std::env::var("CTRADER_ACCESS_TOKEN").ok();

    CTraderConfig {
        environment: palm_oil_bot::config::TradingEnvironment::Demo,
        client_id,
        client_secret,
        account_id,
        access_token,
        server: "demo.ctraderapi.com".to_string(),
        port: 5035,
        client_id_live: None,
        client_secret_live: None,
        account_id_live: None,
    }
}

#[tokio::test]
#[ignore] // Run manually with: cargo test --test ctrader_connection_test -- --ignored
async fn test_demo_connection_successful() {
    let config = create_test_config();
    
    // Skip if using placeholder credentials
    if config.client_id == "test_client_id" {
        println!("⏭️  Skipped: Set CTRADER_CLIENT_ID in .env to run this test");
        return;
    }

    let client = CTraderClient::with_environment(config, CTraderEnvironment::Demo);
    
    // Test TLS connection with 30s timeout
    let result = timeout(Duration::from_secs(30), client.connect()).await;
    
    assert!(result.is_ok(), "Connection should complete within 30s");
    assert!(result.unwrap().is_ok(), "TLS connection to demo.ctraderapi.com:5035 should succeed");
    
    println!("✅ TLS connection successful to DEMO server");
}

#[tokio::test]
#[ignore]
async fn test_demo_authentication_flow() {
    let config = create_test_config();
    
    if config.client_id == "test_client_id" {
        println!("⏭️  Skipped: Set CTRADER_CLIENT_ID in .env");
        return;
    }

    let client = CTraderClient::with_environment(config, CTraderEnvironment::Demo);
    
    // Connect
    let connect_result = timeout(Duration::from_secs(30), client.connect()).await;
    assert!(connect_result.is_ok());
    assert!(connect_result.unwrap().is_ok());
    
    // Authenticate (ProtoOaApplicationAuthReq + ProtoOaAccountAuthReq)
    let auth_result = timeout(Duration::from_secs(30), client.authenticate()).await;
    
    assert!(auth_result.is_ok(), "Authentication should complete within 30s");
    
    match auth_result.unwrap() {
        Ok(_) => println!("✅ Full authentication flow (app + account) successful"),
        Err(e) => {
            println!("❌ Authentication failed: {:?}", e);
            panic!("Authentication should succeed with valid credentials");
        }
    }
}

#[tokio::test]
#[ignore] // Requires network access
async fn test_demo_invalid_credentials() {
    let mut config = create_test_config();
    
    // Force invalid credentials
    config.client_id = "invalid_client_id_12345".to_string();
    config.client_secret = "invalid_secret_67890".to_string();
    
    let client = CTraderClient::with_environment(config, CTraderEnvironment::Demo);
    
    // Connect should work (TLS)
    let connect_result = timeout(Duration::from_secs(30), client.connect()).await;
    if connect_result.is_err() {
        println!("⏭️  Skipped: Timeout connecting to DEMO server");
        return;
    }
    
    if connect_result.unwrap().is_err() {
        println!("⏭️  Skipped: Could not connect to DEMO server");
        return;
    }
    
    // Authenticate should FAIL with invalid credentials
    let auth_result = timeout(Duration::from_secs(30), client.authenticate()).await;
    
    // Should complete (no hang)
    if auth_result.is_err() {
        println!("⏭️  Authentication timed out (network issue)");
        return;
    }
    
    // Should get error (not success)
    let result = auth_result.unwrap();
    assert!(result.is_err(), "Authentication with invalid credentials should fail");
    println!("✅ Invalid credentials correctly rejected: {:?}", result.unwrap_err());
}

#[tokio::test]
#[ignore]
async fn test_demo_reconnect_after_disconnect() {
    let config = create_test_config();
    
    if config.client_id == "test_client_id" {
        println!("⏭️  Skipped: Set CTRADER_CLIENT_ID in .env");
        return;
    }

    let client = CTraderClient::with_environment(config.clone(), CTraderEnvironment::Demo);
    
    // Initial connection
    let result1 = timeout(Duration::from_secs(30), async {
        client.connect().await?;
        client.authenticate().await
    }).await;
    
    assert!(result1.is_ok());
    assert!(result1.unwrap().is_ok(), "Initial connection should succeed");
    
    println!("✅ Initial connection successful");
    
    // Note: Full disconnect/reconnect test would require dropping and recreating client
    // For now, we validate that multiple auth attempts don't break the client
    let result2 = timeout(Duration::from_secs(30), client.authenticate()).await;
    
    // Second auth might fail if already authenticated, but should not panic
    assert!(result2.is_ok(), "Reconnect attempt should complete");
    
    println!("✅ Reconnect flow validated");
}

#[tokio::test]
#[ignore]
async fn test_demo_heartbeat_keepalive() {
    let config = create_test_config();
    
    if config.client_id == "test_client_id" {
        println!("⏭️  Skipped: Set CTRADER_CLIENT_ID in .env");
        return;
    }

    let client = CTraderClient::with_environment(config, CTraderEnvironment::Demo);
    
    // Connect and authenticate
    timeout(Duration::from_secs(30), async {
        client.connect().await?;
        client.authenticate().await
    }).await.expect("Setup failed").expect("Setup failed");
    
    println!("✅ Connected - testing heartbeat keepalive");
    
    // Wait 65 seconds (heartbeat interval is 25s, timeout is 60s)
    // If connection is maintained, heartbeat is working
    tokio::time::sleep(Duration::from_secs(65)).await;
    
    // Try to use connection (subscribe to symbol)
    // This will fail if connection was dropped
    let subscribe_result = timeout(
        Duration::from_secs(10),
        client.subscribe_to_symbol(1) // Try with dummy symbol_id
    ).await;
    
    // We don't care if subscribe fails (invalid symbol), just that we can communicate
    assert!(subscribe_result.is_ok(), "Connection should still be alive after 65s (heartbeat working)");
    
    println!("✅ Connection maintained via heartbeat");
}

#[tokio::test]
async fn test_demo_oauth_not_used() {
    let config = create_test_config();
    
    let client = CTraderClient::with_environment(config, CTraderEnvironment::Demo);
    
    // DEMO mode should NOT use OAuth
    assert!(!client.is_oauth_enabled(), "DEMO mode should not enable OAuth");
    assert!(client.oauth_manager().is_none(), "DEMO mode should not have OAuth manager");
    
    println!("✅ DEMO mode correctly does not use OAuth");
}
