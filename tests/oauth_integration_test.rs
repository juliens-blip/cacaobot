//! OAuth integration test
//!
//! Tests that the OAuth flow is properly integrated into cTrader authentication.

use palm_oil_bot::config::{CTraderConfig, TradingEnvironment};
use palm_oil_bot::modules::trading::ctrader::{CTraderClient, CTraderEnvironment};

#[test]
fn test_demo_does_not_use_oauth() {
    let config = CTraderConfig {
        environment: TradingEnvironment::Demo,
        client_id: "test_demo_client".to_string(),
        client_secret: "test_demo_secret".to_string(),
        account_id: "12345".to_string(),
        access_token: None,
        server: "demo.ctraderapi.com".to_string(),
        port: 5035,
        client_id_live: None,
        client_secret_live: None,
        account_id_live: None,
    };

    let client = CTraderClient::with_environment(config, CTraderEnvironment::Demo);
    
    // DEMO mode should NOT have OAuth enabled
    assert!(!client.is_oauth_enabled());
    assert!(client.oauth_manager().is_none());

    println!("✅ DEMO mode does not use OAuth");
}

#[test]
fn test_live_requires_oauth() {
    let config = CTraderConfig {
        environment: TradingEnvironment::Live,
        client_id: "test_demo_client".to_string(),
        client_secret: "test_demo_secret".to_string(),
        account_id: "12345".to_string(),
        access_token: None,
        server: "live.ctraderapi.com".to_string(),
        port: 5035,
        client_id_live: Some("test_live_client".to_string()),
        client_secret_live: Some("test_live_secret".to_string()),
        account_id_live: Some("67890".to_string()),
    };

    let client = CTraderClient::with_environment(config, CTraderEnvironment::Live);
    
    // LIVE mode should have OAuth enabled
    assert!(client.is_oauth_enabled());
    assert!(client.oauth_manager().is_some());

    println!("✅ LIVE mode uses OAuth");
}

#[test]
fn test_active_credentials_demo() {
    let config = CTraderConfig {
        environment: TradingEnvironment::Demo,
        client_id: "demo_client_123".to_string(),
        client_secret: "demo_secret_456".to_string(),
        account_id: "11111".to_string(),
        access_token: None,
        server: "demo.ctraderapi.com".to_string(),
        port: 5035,
        client_id_live: Some("live_client_789".to_string()),
        client_secret_live: Some("live_secret_000".to_string()),
        account_id_live: Some("22222".to_string()),
    };

    // In DEMO mode, should use demo credentials
    assert_eq!(config.active_client_id(), "demo_client_123");
    assert_eq!(config.active_client_secret(), "demo_secret_456");
    assert_eq!(config.active_account_id(), "11111");

    println!("✅ DEMO mode uses demo credentials");
}

#[test]
fn test_active_credentials_live() {
    let mut config = CTraderConfig {
        environment: TradingEnvironment::Live,
        client_id: "demo_client_123".to_string(),
        client_secret: "demo_secret_456".to_string(),
        account_id: "11111".to_string(),
        access_token: None,
        server: "live.ctraderapi.com".to_string(),
        port: 5035,
        client_id_live: Some("live_client_789".to_string()),
        client_secret_live: Some("live_secret_000".to_string()),
        account_id_live: Some("22222".to_string()),
    };

    config.environment = TradingEnvironment::Live;

    // In LIVE mode, should use live credentials
    assert_eq!(config.active_client_id(), "live_client_789");
    assert_eq!(config.active_client_secret(), "live_secret_000");
    assert_eq!(config.active_account_id(), "22222");

    println!("✅ LIVE mode uses live credentials");
}

#[test]
fn test_active_credentials_live_fallback() {
    let mut config = CTraderConfig {
        environment: TradingEnvironment::Live,
        client_id: "demo_client_123".to_string(),
        client_secret: "demo_secret_456".to_string(),
        account_id: "11111".to_string(),
        access_token: None,
        server: "live.ctraderapi.com".to_string(),
        port: 5035,
        client_id_live: None,
        client_secret_live: None,
        account_id_live: None,
    };

    config.environment = TradingEnvironment::Live;

    // In LIVE mode without live creds, should fallback to demo
    assert_eq!(config.active_client_id(), "demo_client_123");
    assert_eq!(config.active_client_secret(), "demo_secret_456");
    assert_eq!(config.active_account_id(), "11111");

    println!("✅ LIVE mode falls back to demo credentials if live creds not set");
}
