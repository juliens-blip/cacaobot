//! Custom error types for the Palm Oil Trading Bot
//!
//! This module defines all error types used throughout the application.

use thiserror::Error;

/// Main error type for the bot
#[derive(Debug, Error)]
pub enum BotError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// cTrader API errors
    #[error("cTrader API error: {0}")]
    CTrader(#[from] CTraderError),

    /// Perplexity API errors
    #[error("Perplexity API error: {0}")]
    Perplexity(#[from] PerplexityError),

    /// Twitter scraping errors
    #[error("Twitter scraping error: {0}")]
    Twitter(String),

    /// Strategy errors
    #[error("Strategy error: {0}")]
    Strategy(String),

    /// Network errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// cTrader specific errors
#[derive(Debug, Error)]
pub enum CTraderError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Order rejected: {0}")]
    OrderRejected(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Disconnected from server")]
    Disconnected,

    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Perplexity API specific errors
#[derive(Debug, Error)]
pub enum PerplexityError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Result type alias for bot operations
pub type Result<T> = std::result::Result<T, BotError>;
