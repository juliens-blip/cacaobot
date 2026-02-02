//! OAuth module for cTrader Open API
//!
//! This module handles OAuth authentication flow for cTrader production accounts.
//! Documentation: https://help.ctrader.com/open-api/account-authentication/
//!
//! OAuth Flow:
//! 1. User authorizes via browser at `https://connect.spotware.com/apps/auth`
//! 2. Redirect with authorization code to callback URL
//! 3. Exchange code for access_token and refresh_token via POST to token endpoint
//! 4. Use refresh_token to get new access_token before expiration

use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{CTraderError, Result};

/// OAuth endpoints for cTrader Spotware Connect
const AUTH_URL: &str = "https://connect.spotware.com/apps/auth";
const TOKEN_URL: &str = "https://connect.spotware.com/apps/token";

/// Buffer time before token expiration to trigger refresh (5 minutes)
const REFRESH_BUFFER_SECS: i64 = 300;

/// OAuth token with expiration tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Access token for API authentication
    pub access_token: String,
    /// Refresh token for obtaining new access tokens
    pub refresh_token: String,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
}

impl OAuthToken {
    /// Create a new OAuthToken from token response
    pub fn new(access_token: String, refresh_token: String, expires_in_secs: i64) -> Self {
        let expires_at = Utc::now() + Duration::seconds(expires_in_secs);
        Self {
            access_token,
            refresh_token,
            expires_at,
        }
    }

    /// Check if the token is expired or about to expire
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at - Duration::seconds(REFRESH_BUFFER_SECS)
    }

    /// Time remaining until expiration
    pub fn time_remaining(&self) -> Duration {
        self.expires_at - Utc::now()
    }
}

/// OAuth token response from cTrader API
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    /// Token validity in seconds (typically 7200 = 2 hours)
    expires_in: i64,
    token_type: String,
}

/// OAuth error response from cTrader API
#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: String,
    error_description: Option<String>,
}

/// Environment for cTrader API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    /// Demo environment (demo.ctraderapi.com)
    Demo,
    /// Live production environment (live.ctraderapi.com)
    Live,
}

impl Environment {
    /// Get the server hostname for this environment
    pub fn server(&self) -> &'static str {
        match self {
            Environment::Demo => "demo.ctraderapi.com",
            Environment::Live => "live.ctraderapi.com",
        }
    }

    /// Get the port for this environment (both use 5035)
    pub fn port(&self) -> u16 {
        5035
    }
}

/// OAuth configuration for cTrader
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub environment: Environment,
}

/// OAuth client for cTrader authentication
pub struct OAuthClient {
    config: OAuthConfig,
    http_client: Client,
    token: Arc<RwLock<Option<OAuthToken>>>,
}

impl OAuthClient {
    /// Create a new OAuth client
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// Create OAuth client with an existing token
    pub fn with_token(config: OAuthConfig, token: OAuthToken) -> Self {
        Self {
            config,
            http_client: Client::new(),
            token: Arc::new(RwLock::new(Some(token))),
        }
    }

    /// Get the authorization URL for user login
    ///
    /// User should be redirected to this URL in a browser.
    /// After authorization, they will be redirected to redirect_uri with a code.
    pub fn get_auth_url(&self, scope: Option<&str>) -> String {
        let scope = scope.unwrap_or("trading");
        format!(
            "{}?client_id={}&redirect_uri={}&scope={}&response_type=code",
            AUTH_URL,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(scope)
        )
    }

    /// Exchange authorization code for access token
    ///
    /// This should be called after the user authorizes and is redirected back
    /// with the authorization code.
    pub async fn exchange_code(&self, authorization_code: &str) -> Result<OAuthToken> {
        info!("Exchanging authorization code for access token");

        let params = [
            ("grant_type", "authorization_code"),
            ("code", authorization_code),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("redirect_uri", &self.config.redirect_uri),
        ];

        let response = self
            .http_client
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| CTraderError::AuthFailed(format!("Token request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| CTraderError::AuthFailed(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if let Ok(error_resp) = serde_json::from_str::<OAuthErrorResponse>(&body) {
                let msg = error_resp
                    .error_description
                    .unwrap_or(error_resp.error);
                error!("OAuth error: {}", msg);
                return Err(CTraderError::AuthFailed(msg).into());
            }
            return Err(CTraderError::AuthFailed(format!(
                "Token exchange failed with status {}: {}",
                status, body
            ))
            .into());
        }

        let token_resp: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| CTraderError::AuthFailed(format!("Failed to parse token response: {}", e)))?;

        debug!(
            "Token obtained: type={}, expires_in={}s",
            token_resp.token_type, token_resp.expires_in
        );

        let token = OAuthToken::new(
            token_resp.access_token,
            token_resp.refresh_token,
            token_resp.expires_in,
        );

        // Store token
        *self.token.write().await = Some(token.clone());

        info!("OAuth token obtained successfully, expires at {}", token.expires_at);
        Ok(token)
    }

    /// Refresh the access token using the refresh token
    pub async fn refresh_token(&self) -> Result<OAuthToken> {
        let current_token = self.token.read().await;
        let refresh_token = match current_token.as_ref() {
            Some(t) => t.refresh_token.clone(),
            None => {
                return Err(CTraderError::AuthFailed(
                    "No token available to refresh".into(),
                )
                .into())
            }
        };
        drop(current_token);

        info!("Refreshing access token");

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        let response = self
            .http_client
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| CTraderError::AuthFailed(format!("Refresh request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| CTraderError::AuthFailed(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if let Ok(error_resp) = serde_json::from_str::<OAuthErrorResponse>(&body) {
                let msg = error_resp
                    .error_description
                    .unwrap_or(error_resp.error);
                error!("OAuth refresh error: {}", msg);
                return Err(CTraderError::AuthFailed(msg).into());
            }
            return Err(CTraderError::AuthFailed(format!(
                "Token refresh failed with status {}: {}",
                status, body
            ))
            .into());
        }

        let token_resp: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| CTraderError::AuthFailed(format!("Failed to parse refresh response: {}", e)))?;

        let new_token = OAuthToken::new(
            token_resp.access_token,
            token_resp.refresh_token,
            token_resp.expires_in,
        );

        // Store new token
        *self.token.write().await = Some(new_token.clone());

        info!(
            "Token refreshed successfully, expires at {}",
            new_token.expires_at
        );
        Ok(new_token)
    }

    /// Get a valid access token, refreshing if necessary
    ///
    /// This is the main method to use when you need an access token.
    /// It will automatically refresh the token if it's expired or about to expire.
    pub async fn get_valid_token(&self) -> Result<String> {
        let token = self.token.read().await;

        match token.as_ref() {
            Some(t) if !t.is_expired() => {
                debug!(
                    "Using existing token, expires in {:?}",
                    t.time_remaining()
                );
                Ok(t.access_token.clone())
            }
            Some(t) => {
                warn!(
                    "Token expired or expiring soon (expires_at: {}), refreshing...",
                    t.expires_at
                );
                drop(token); // Release read lock
                let new_token = self.refresh_token().await?;
                Ok(new_token.access_token)
            }
            None => Err(CTraderError::AuthFailed(
                "No OAuth token available. Please authenticate first.".into(),
            )
            .into()),
        }
    }

    /// Set token from external source (e.g., loaded from storage)
    pub async fn set_token(&self, token: OAuthToken) {
        *self.token.write().await = Some(token);
    }

    /// Get current token (if any)
    pub async fn get_token(&self) -> Option<OAuthToken> {
        self.token.read().await.clone()
    }

    /// Check if we have a valid (non-expired) token
    pub async fn has_valid_token(&self) -> bool {
        self.token
            .read()
            .await
            .as_ref()
            .map(|t| !t.is_expired())
            .unwrap_or(false)
    }

    /// Clear stored token
    pub async fn clear_token(&self) {
        *self.token.write().await = None;
    }
}

/// Token storage for persisting OAuth tokens
pub trait TokenStorage: Send + Sync {
    /// Save token to storage
    fn save(&self, token: &OAuthToken) -> Result<()>;
    /// Load token from storage
    fn load(&self) -> Result<Option<OAuthToken>>;
}

/// File-based token storage
pub struct FileTokenStorage {
    path: std::path::PathBuf,
}

impl FileTokenStorage {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl TokenStorage for FileTokenStorage {
    fn save(&self, token: &OAuthToken) -> Result<()> {
        let json = serde_json::to_string_pretty(token)?;
        std::fs::write(&self.path, json)?;
        info!("Token saved to {:?}", self.path);
        Ok(())
    }

    fn load(&self) -> Result<Option<OAuthToken>> {
        if !self.path.exists() {
            debug!("Token file does not exist: {:?}", self.path);
            return Ok(None);
        }

        let json = std::fs::read_to_string(&self.path)?;
        let token: OAuthToken = serde_json::from_str(&json)?;

        if token.is_expired() {
            warn!("Loaded token is expired, will need refresh");
        }

        info!("Token loaded from {:?}", self.path);
        Ok(Some(token))
    }
}

/// OAuth manager that combines client with persistent storage
pub struct OAuthManager {
    client: OAuthClient,
    storage: Option<Box<dyn TokenStorage>>,
}

impl OAuthManager {
    /// Create new OAuth manager
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            client: OAuthClient::new(config),
            storage: None,
        }
    }

    /// Set token storage
    pub fn with_storage(mut self, storage: Box<dyn TokenStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Initialize: load token from storage if available
    pub async fn init(&self) -> Result<()> {
        if let Some(storage) = &self.storage {
            if let Some(token) = storage.load()? {
                self.client.set_token(token).await;
                info!("Initialized OAuth with stored token");
            }
        }
        Ok(())
    }

    /// Get authorization URL
    pub fn get_auth_url(&self, scope: Option<&str>) -> String {
        self.client.get_auth_url(scope)
    }

    /// Exchange code for token (and save it)
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthToken> {
        let token = self.client.exchange_code(code).await?;
        if let Some(storage) = &self.storage {
            storage.save(&token)?;
        }
        Ok(token)
    }

    /// Refresh token (and save it)
    pub async fn refresh_token(&self) -> Result<OAuthToken> {
        let token = self.client.refresh_token().await?;
        if let Some(storage) = &self.storage {
            storage.save(&token)?;
        }
        Ok(token)
    }

    /// Get valid access token (auto-refresh if needed)
    pub async fn get_valid_token(&self) -> Result<String> {
        // Check if we need to refresh
        let needs_refresh = {
            let token = self.client.get_token().await;
            token.as_ref().map(|t| t.is_expired()).unwrap_or(false)
        };

        if needs_refresh {
            let token = self.refresh_token().await?;
            return Ok(token.access_token);
        }

        self.client.get_valid_token().await
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.client.has_valid_token().await
    }

    /// Get underlying client
    pub fn client(&self) -> &OAuthClient {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_token_expiration() {
        // Token that expires in 2 hours
        let token = OAuthToken::new(
            "access123".to_string(),
            "refresh456".to_string(),
            7200,
        );
        assert!(!token.is_expired());

        // Token that expires in 1 minute (within buffer)
        let token_expiring = OAuthToken::new(
            "access123".to_string(),
            "refresh456".to_string(),
            60,
        );
        assert!(token_expiring.is_expired()); // Within 5min buffer

        // Token that expired 1 hour ago
        let mut token_expired = OAuthToken::new(
            "access123".to_string(),
            "refresh456".to_string(),
            0,
        );
        token_expired.expires_at = Utc::now() - Duration::hours(1);
        assert!(token_expired.is_expired());
    }

    #[test]
    fn test_environment_config() {
        assert_eq!(Environment::Demo.server(), "demo.ctraderapi.com");
        assert_eq!(Environment::Live.server(), "live.ctraderapi.com");
        assert_eq!(Environment::Demo.port(), 5035);
        assert_eq!(Environment::Live.port(), 5035);
    }

    #[test]
    fn test_auth_url_generation() {
        let config = OAuthConfig {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:8080/callback".to_string(),
            environment: Environment::Demo,
        };

        let client = OAuthClient::new(config);
        let url = client.get_auth_url(Some("trading"));

        assert!(url.contains("connect.spotware.com/apps/auth"));
        assert!(url.contains("client_id=test_client"));
        assert!(url.contains("scope=trading"));
        assert!(url.contains("response_type=code"));
    }

    #[test]
    fn test_oauth_demo_vs_live() {
        let demo_config = OAuthConfig {
            client_id: "demo_client".to_string(),
            client_secret: "demo_secret".to_string(),
            redirect_uri: "http://localhost/callback".to_string(),
            environment: Environment::Demo,
        };

        let live_config = OAuthConfig {
            client_id: "live_client".to_string(),
            client_secret: "live_secret".to_string(),
            redirect_uri: "http://localhost/callback".to_string(),
            environment: Environment::Live,
        };

        assert_eq!(demo_config.environment, Environment::Demo);
        assert_eq!(live_config.environment, Environment::Live);
    }

    #[tokio::test]
    async fn test_token_storage() {
        let token = OAuthToken::new(
            "access_test".to_string(),
            "refresh_test".to_string(),
            7200,
        );

        // Serialize/deserialize
        let json = serde_json::to_string(&token).unwrap();
        let loaded: OAuthToken = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.access_token, "access_test");
        assert_eq!(loaded.refresh_token, "refresh_test");
    }
}
