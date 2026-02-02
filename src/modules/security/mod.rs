//! Security module for Palm Oil Trading Bot
//!
//! Provides:
//! - Secret validation and sanitized logging
//! - Rate limiting for API calls (Perplexity, Twitter, cTrader)

pub mod rate_limiter;
pub mod secrets_manager;

pub use rate_limiter::{ApiRateLimiter, RateLimiterConfig};
pub use secrets_manager::{SecretValidator, SecretString};
