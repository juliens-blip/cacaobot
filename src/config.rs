//! Configuration module for the Palm Oil Trading Bot
//!
//! Loads configuration from environment variables and .env file.

use crate::error::{BotError, Result};
use serde::Deserialize;
use std::env;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub ctrader: CTraderConfig,
    pub perplexity: PerplexityConfig,
    pub trading: TradingConfig,
    pub strategy: StrategyConfig,
    pub kols: Vec<String>,
    pub bot: BotConfig,
}

/// cTrader API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CTraderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub account_id: String,
    pub server: String,
    pub port: u16,
}

/// Perplexity API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct PerplexityConfig {
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
}

/// Trading parameters
#[derive(Debug, Clone, Deserialize)]
pub struct TradingConfig {
    pub symbol: String,
    pub risk_per_trade: f64,
    pub take_profit_percent: f64,
    pub stop_loss_percent: f64,
    pub max_positions: usize,
    pub max_daily_loss_percent: f64,
}

/// Strategy parameters
#[derive(Debug, Clone, Deserialize)]
pub struct StrategyConfig {
    pub rsi_period: usize,
    pub rsi_oversold: f64,
    pub rsi_overbought: f64,
    pub rsi_timeframe: String,
    pub sentiment_threshold: i32,
}

/// Bot runtime settings
#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    pub cycle_interval_secs: u64,
    pub dry_run: bool,
    pub log_level: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        let config = Config {
            ctrader: CTraderConfig {
                client_id: get_env("CTRADER_CLIENT_ID")?,
                client_secret: get_env("CTRADER_CLIENT_SECRET")?,
                account_id: get_env("CTRADER_ACCOUNT_ID")?,
                server: get_env_or("CTRADER_SERVER", "demo.ctraderapi.com"),
                port: get_env_or("CTRADER_PORT", "5035").parse().unwrap_or(5035),
            },
            perplexity: PerplexityConfig {
                api_key: get_env("PERPLEXITY_API_KEY")?,
                endpoint: get_env_or(
                    "PERPLEXITY_ENDPOINT",
                    "https://api.perplexity.ai/chat/completions",
                ),
                model: get_env_or("PERPLEXITY_MODEL", "sonar"),
            },
            trading: TradingConfig {
                symbol: get_env_or("SYMBOL", "FCPO"),
                risk_per_trade: get_env_or("RISK_PER_TRADE", "1.0").parse().unwrap_or(1.0),
                take_profit_percent: get_env_or("TAKE_PROFIT_PERCENT", "2.0")
                    .parse()
                    .unwrap_or(2.0),
                stop_loss_percent: get_env_or("STOP_LOSS_PERCENT", "1.5")
                    .parse()
                    .unwrap_or(1.5),
                max_positions: get_env_or("MAX_POSITIONS", "1").parse().unwrap_or(1),
                max_daily_loss_percent: get_env_or("MAX_DAILY_LOSS_PERCENT", "5.0")
                    .parse()
                    .unwrap_or(5.0),
            },
            strategy: StrategyConfig {
                rsi_period: get_env_or("RSI_PERIOD", "14").parse().unwrap_or(14),
                rsi_oversold: get_env_or("RSI_OVERSOLD", "30").parse().unwrap_or(30.0),
                rsi_overbought: get_env_or("RSI_OVERBOUGHT", "70").parse().unwrap_or(70.0),
                rsi_timeframe: get_env_or("RSI_TIMEFRAME", "5m"),
                sentiment_threshold: get_env_or("SENTIMENT_THRESHOLD", "30")
                    .parse()
                    .unwrap_or(30),
            },
            kols: vec![
                get_env_or("KOL_1", "PalmOilTrader"),
                get_env_or("KOL_2", "CommodityInsights"),
                get_env_or("KOL_3", "AgriMarketsDaily"),
                get_env_or("KOL_4", "OilseedMarkets"),
                get_env_or("KOL_5", "SEAsiaTrader"),
            ],
            bot: BotConfig {
                cycle_interval_secs: get_env_or("CYCLE_INTERVAL_SECS", "60")
                    .parse()
                    .unwrap_or(60),
                dry_run: get_env_or("DRY_RUN", "true").parse().unwrap_or(true),
                log_level: get_env_or("RUST_LOG", "info"),
            },
        };

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.ctrader.client_id.is_empty() {
            return Err(BotError::Config("CTRADER_CLIENT_ID is required".into()));
        }
        if self.ctrader.client_secret.is_empty() {
            return Err(BotError::Config("CTRADER_CLIENT_SECRET is required".into()));
        }
        if self.perplexity.api_key.is_empty() {
            return Err(BotError::Config("PERPLEXITY_API_KEY is required".into()));
        }
        if self.trading.take_profit_percent <= 0.0 {
            return Err(BotError::Config(
                "TAKE_PROFIT_PERCENT must be positive".into(),
            ));
        }
        if self.trading.stop_loss_percent <= 0.0 {
            return Err(BotError::Config("STOP_LOSS_PERCENT must be positive".into()));
        }
        Ok(())
    }
}

/// Default configuration for backtesting (no API keys required)
impl Default for Config {
    fn default() -> Self {
        Self {
            ctrader: CTraderConfig {
                client_id: String::new(),
                client_secret: String::new(),
                account_id: "0".to_string(),
                server: "demo.ctraderapi.com".to_string(),
                port: 5035,
            },
            perplexity: PerplexityConfig {
                api_key: String::new(),
                endpoint: "https://api.perplexity.ai/chat/completions".to_string(),
                model: "sonar".to_string(),
            },
            trading: TradingConfig {
                symbol: "FCPO".to_string(),
                risk_per_trade: 1.0,
                take_profit_percent: 2.0,
                stop_loss_percent: 1.5,
                max_positions: 1,
                max_daily_loss_percent: 5.0,
            },
            strategy: StrategyConfig {
                rsi_period: 14,
                rsi_oversold: 30.0,
                rsi_overbought: 70.0,
                rsi_timeframe: "5m".to_string(),
                sentiment_threshold: 30,
            },
            kols: vec![
                "PalmOilTrader".to_string(),
                "CommodityInsights".to_string(),
            ],
            bot: BotConfig {
                cycle_interval_secs: 60,
                dry_run: true,
                log_level: "info".to_string(),
            },
        }
    }
}

/// Get required environment variable
fn get_env(key: &str) -> Result<String> {
    env::var(key).map_err(|_| BotError::Config(format!("Missing environment variable: {}", key)))
}

/// Get environment variable with default value
fn get_env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = Config {
            ctrader: CTraderConfig {
                client_id: "test".into(),
                client_secret: "test".into(),
                account_id: "123".into(),
                server: "demo.ctraderapi.com".into(),
                port: 5035,
            },
            perplexity: PerplexityConfig {
                api_key: "test-key".into(),
                endpoint: "https://api.perplexity.ai".into(),
                model: "sonar".into(),
            },
            trading: TradingConfig {
                symbol: "FCPO".into(),
                risk_per_trade: 1.0,
                take_profit_percent: 2.0,
                stop_loss_percent: 1.5,
                max_positions: 1,
                max_daily_loss_percent: 5.0,
            },
            strategy: StrategyConfig {
                rsi_period: 14,
                rsi_oversold: 30.0,
                rsi_overbought: 70.0,
                rsi_timeframe: "5m".into(),
                sentiment_threshold: 30,
            },
            kols: vec!["test".into()],
            bot: BotConfig {
                cycle_interval_secs: 60,
                dry_run: true,
                log_level: "info".into(),
            },
        };

        assert!(config.validate().is_ok());
    }
}
