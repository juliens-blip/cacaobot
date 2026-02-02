//! OAuth environment switching and credential validation tests

use palm_oil_bot::config::{CTraderConfig, TradingEnvironment};
use palm_oil_bot::modules::trading::ctrader::{CTraderClient, CTraderEnvironment};

fn demo_config() -> CTraderConfig {
    CTraderConfig {
        environment: TradingEnvironment::Demo,
        client_id: "demo_client_id_12345".to_string(),
        client_secret: "demo_secret_abcdef".to_string(),
        account_id: "10092792".to_string(),
        access_token: Some("demo_access_token".to_string()),
        server: "demo.ctraderapi.com".to_string(),
        port: 5035,
        client_id_live: Some("live_client_id_67890".to_string()),
        client_secret_live: Some("live_secret_ghijkl".to_string()),
        account_id_live: Some("20185634".to_string()),
    }
}

fn live_config() -> CTraderConfig {
    CTraderConfig {
        environment: TradingEnvironment::Live,
        client_id: "live_client_id_67890".to_string(),
        client_secret: "live_secret_ghijkl".to_string(),
        account_id: "20185634".to_string(),
        access_token: None,
        server: "live.ctraderapi.com".to_string(),
        port: 5035,
        client_id_live: None,
        client_secret_live: None,
        account_id_live: None,
    }
}

// ============================================================================
// Environment Parsing Tests
// ============================================================================

#[test]
fn test_environment_from_string() {
    assert_eq!(
        "demo".parse::<CTraderEnvironment>().ok(),
        Some(CTraderEnvironment::Demo)
    );
    assert_eq!(
        "DEMO".parse::<CTraderEnvironment>().ok(),
        Some(CTraderEnvironment::Demo)
    );
    assert_eq!(
        "Demo".parse::<CTraderEnvironment>().ok(),
        Some(CTraderEnvironment::Demo)
    );
}

#[test]
fn test_environment_live_variants() {
    assert_eq!(
        "live".parse::<CTraderEnvironment>().ok(),
        Some(CTraderEnvironment::Live)
    );
    assert_eq!(
        "LIVE".parse::<CTraderEnvironment>().ok(),
        Some(CTraderEnvironment::Live)
    );
    assert_eq!(
        "production".parse::<CTraderEnvironment>().ok(),
        Some(CTraderEnvironment::Live)
    );
    assert_eq!(
        "prod".parse::<CTraderEnvironment>().ok(),
        Some(CTraderEnvironment::Live)
    );
}

#[test]
fn test_environment_invalid() {
    assert!("".parse::<CTraderEnvironment>().is_err());
    assert!("test".parse::<CTraderEnvironment>().is_err());
    assert!("staging".parse::<CTraderEnvironment>().is_err());
    assert!("invalid".parse::<CTraderEnvironment>().is_err());
}

// ============================================================================
// Server Endpoint Tests
// ============================================================================

#[test]
fn test_demo_endpoint() {
    let env = CTraderEnvironment::Demo;
    assert_eq!(env.server_endpoint(), "demo.ctraderapi.com");
    assert_eq!(env.default_port(), 5035);
    assert!(!env.is_live());
}

#[test]
fn test_live_endpoint() {
    let env = CTraderEnvironment::Live;
    assert_eq!(env.server_endpoint(), "live.ctraderapi.com");
    assert_eq!(env.default_port(), 5035);
    assert!(env.is_live());
}

// ============================================================================
// Client Creation Tests
// ============================================================================

#[tokio::test]
async fn test_client_default_is_demo() {
    let client = CTraderClient::new(demo_config());
    assert_eq!(client.environment(), CTraderEnvironment::Demo);
    assert!(!client.environment().is_live());
}

#[tokio::test]
async fn test_client_explicit_live() {
    let client = CTraderClient::with_environment(live_config(), CTraderEnvironment::Live);
    assert_eq!(client.environment(), CTraderEnvironment::Live);
    assert!(client.environment().is_live());
}

#[tokio::test]
async fn test_client_auto_detect_demo() {
    let config = demo_config();
    let client = CTraderClient::from_config(config);
    assert_eq!(client.environment(), CTraderEnvironment::Demo);
}

#[tokio::test]
async fn test_client_auto_detect_live() {
    let config = live_config();
    let client = CTraderClient::from_config(config);
    assert_eq!(client.environment(), CTraderEnvironment::Live);
}

// ============================================================================
// Credential Validation Tests
// ============================================================================

#[tokio::test]
async fn test_valid_credentials() {
    let client = CTraderClient::new(demo_config());
    assert!(client.validate_credentials().is_ok());
}

#[tokio::test]
async fn test_empty_client_id_rejected() {
    let mut config = demo_config();
    config.client_id = "".to_string();
    let client = CTraderClient::new(config);
    let result = client.validate_credentials();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Client ID"));
}

#[tokio::test]
async fn test_empty_client_secret_rejected() {
    let mut config = demo_config();
    config.client_secret = "".to_string();
    let client = CTraderClient::new(config);
    let result = client.validate_credentials();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Client Secret"));
}

#[tokio::test]
async fn test_empty_account_id_rejected() {
    let mut config = demo_config();
    config.account_id = "".to_string();
    let client = CTraderClient::new(config);
    let result = client.validate_credentials();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Account ID"));
}

// ============================================================================
// Credential Format Tests
// ============================================================================

#[test]
fn test_client_id_format_typical() {
    let client_id = "20529_Gn21P7WCQ4im4xX4RZt2QKnYvYK55Ni08ERLINu6q5BbRXTHdL";
    
    assert!(client_id.len() > 20, "Client ID should be reasonably long");
    assert!(
        client_id.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "Client ID should contain only alphanumeric and underscore"
    );
}

#[test]
fn test_account_id_format_numeric() {
    let account_id = "10092792";
    
    assert!(
        account_id.parse::<i64>().is_ok(),
        "Account ID should be numeric"
    );
}

// ============================================================================
// Environment Display Tests
// ============================================================================

#[test]
fn test_environment_display() {
    assert_eq!(format!("{}", CTraderEnvironment::Demo), "DEMO");
    assert_eq!(format!("{}", CTraderEnvironment::Live), "LIVE");
}

#[test]
fn test_environment_debug() {
    assert_eq!(format!("{:?}", CTraderEnvironment::Demo), "Demo");
    assert_eq!(format!("{:?}", CTraderEnvironment::Live), "Live");
}

// ============================================================================
// Environment Switching Simulation
// ============================================================================

#[tokio::test]
async fn test_environment_switch_demo_to_live() {
    let demo_client = CTraderClient::new(demo_config());
    assert_eq!(demo_client.environment(), CTraderEnvironment::Demo);
    
    let live_client = CTraderClient::with_environment(live_config(), CTraderEnvironment::Live);
    assert_eq!(live_client.environment(), CTraderEnvironment::Live);
}

// ============================================================================
// Mock Connection Tests (for CI/CD)
// ============================================================================

#[tokio::test]
async fn test_client_not_authenticated_initially() {
    let client = CTraderClient::new(demo_config());
    assert!(!client.is_authenticated().await);
}

#[tokio::test]
async fn test_live_client_not_authenticated_initially() {
    let client = CTraderClient::with_environment(live_config(), CTraderEnvironment::Live);
    assert!(!client.is_authenticated().await);
}

// ============================================================================
// Configuration Preservation Tests
// ============================================================================

#[tokio::test]
async fn test_config_preserved_with_environment() {
    let config = demo_config();
    let original_account = config.account_id.clone();
    
    let client = CTraderClient::with_environment(config.clone(), CTraderEnvironment::Demo);
    assert!(client.validate_credentials().is_ok());
    
    let config2 = demo_config();
    assert_eq!(config2.account_id, original_account);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_environment_default() {
    let env: CTraderEnvironment = Default::default();
    assert_eq!(env, CTraderEnvironment::Demo);
}

#[test]
fn test_environment_clone() {
    let env = CTraderEnvironment::Live;
    let cloned = env;
    assert_eq!(env, cloned);
}

#[test]
fn test_environment_equality() {
    assert_eq!(CTraderEnvironment::Demo, CTraderEnvironment::Demo);
    assert_eq!(CTraderEnvironment::Live, CTraderEnvironment::Live);
    assert_ne!(CTraderEnvironment::Demo, CTraderEnvironment::Live);
}
