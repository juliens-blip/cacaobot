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

/// Trading environment (DEMO or LIVE)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TradingEnvironment {
    #[default]
    Demo,
    Live,
}

impl TradingEnvironment {
    /// Get the server endpoint for this environment
    pub fn server_endpoint(&self) -> &'static str {
        match self {
            Self::Demo => "demo.ctraderapi.com",
            Self::Live => "live.ctraderapi.com",
        }
    }

    /// Check if this is live trading
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Live)
    }
}

impl std::str::FromStr for TradingEnvironment {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "live" | "production" | "prod" => Self::Live,
            _ => Self::Demo,
        })
    }
}

impl std::fmt::Display for TradingEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Demo => write!(f, "demo"),
            Self::Live => write!(f, "live"),
        }
    }
}

/// cTrader API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CTraderConfig {
    /// Current trading environment
    pub environment: TradingEnvironment,
    /// Demo client ID
    pub client_id: String,
    /// Demo client secret
    pub client_secret: String,
    /// Demo account ID
    pub account_id: String,
    /// OAuth access token (required for both DEMO and LIVE)
    pub access_token: Option<String>,
    /// Server hostname (auto-set based on environment)
    pub server: String,
    /// Server port
    pub port: u16,
    /// Live production credentials (optional)
    pub client_id_live: Option<String>,
    pub client_secret_live: Option<String>,
    pub account_id_live: Option<String>,
}

impl CTraderConfig {
    /// Load cTrader configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(CTraderConfig {
            environment: get_env_or("CTRADER_ENVIRONMENT", "demo")
                .parse()
                .unwrap_or_default(),
            client_id: get_env("CTRADER_CLIENT_ID")?,
            client_secret: get_env("CTRADER_CLIENT_SECRET")?,
            account_id: get_env("CTRADER_ACCOUNT_ID")?,
            access_token: env::var("CTRADER_ACCESS_TOKEN").ok(),
            server: get_env_or("CTRADER_SERVER", "demo.ctraderapi.com"),
            port: get_env_or("CTRADER_PORT", "5035").parse().unwrap_or(5035),
            client_id_live: env::var("CTRADER_CLIENT_ID_LIVE").ok(),
            client_secret_live: env::var("CTRADER_CLIENT_SECRET_LIVE").ok(),
            account_id_live: env::var("CTRADER_ACCOUNT_ID_LIVE").ok(),
        })
    }

    /// Get the active client ID based on environment
    pub fn active_client_id(&self) -> &str {
        if self.environment.is_live() {
            self.client_id_live.as_deref().unwrap_or(&self.client_id)
        } else {
            &self.client_id
        }
    }

    /// Get the active client secret based on environment
    pub fn active_client_secret(&self) -> &str {
        if self.environment.is_live() {
            self.client_secret_live.as_deref().unwrap_or(&self.client_secret)
        } else {
            &self.client_secret
        }
    }

    /// Get the active account ID based on environment
    pub fn active_account_id(&self) -> &str {
        if self.environment.is_live() {
            self.account_id_live.as_deref().unwrap_or(&self.account_id)
        } else {
            &self.account_id
        }
    }

    /// Get the active server endpoint based on environment
    pub fn active_server(&self) -> &str {
        self.environment.server_endpoint()
    }
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
    pub initial_balance: f64,
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
                environment: get_env_or("CTRADER_ENVIRONMENT", "demo")
                    .parse()
                    .unwrap_or_default(),
                client_id: get_env("CTRADER_CLIENT_ID")?,
                client_secret: get_env("CTRADER_CLIENT_SECRET")?,
                account_id: get_env("CTRADER_ACCOUNT_ID")?,
                access_token: env::var("CTRADER_ACCESS_TOKEN").ok(),
                server: get_env_or("CTRADER_SERVER", "demo.ctraderapi.com"),
                port: get_env_or("CTRADER_PORT", "5035").parse().unwrap_or(5035),
                client_id_live: env::var("CTRADER_CLIENT_ID_LIVE").ok(),
                client_secret_live: env::var("CTRADER_CLIENT_SECRET_LIVE").ok(),
                account_id_live: env::var("CTRADER_ACCOUNT_ID_LIVE").ok(),
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
                initial_balance: get_env_or("INITIAL_BALANCE", "10000.0")
                    .parse()
                    .unwrap_or(10000.0),
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
        if self.ctrader.account_id.is_empty() {
            return Err(BotError::Config("CTRADER_ACCOUNT_ID is required".into()));
        }
        if self.ctrader.access_token.as_deref().unwrap_or("").is_empty() {
            if self.bot.dry_run {
                tracing::warn!("CTRADER_ACCESS_TOKEN not set — running in offline dry-run mode");
            } else {
                return Err(BotError::Config(
                    "CTRADER_ACCESS_TOKEN is required. Run `cargo run --bin get-token` to obtain one.".into(),
                ));
            }
        }
        if self.ctrader.environment.is_live() {
            if self.ctrader.client_id_live.as_deref().unwrap_or("").is_empty() {
                return Err(BotError::Config("CTRADER_CLIENT_ID_LIVE is required for LIVE trading".into()));
            }
            if self.ctrader.client_secret_live.as_deref().unwrap_or("").is_empty() {
                return Err(BotError::Config("CTRADER_CLIENT_SECRET_LIVE is required for LIVE trading".into()));
            }
            if self.ctrader.account_id_live.as_deref().unwrap_or("").is_empty() {
                return Err(BotError::Config("CTRADER_ACCOUNT_ID_LIVE is required for LIVE trading".into()));
            }
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
                environment: TradingEnvironment::Demo,
                client_id: String::new(),
                client_secret: String::new(),
                account_id: "0".to_string(),
                access_token: None,
                server: "demo.ctraderapi.com".to_string(),
                port: 5035,
                client_id_live: None,
                client_secret_live: None,
                account_id_live: None,
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
                initial_balance: 10000.0,
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
    use std::env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_validation() {
        let config = Config {
            ctrader: CTraderConfig {
                environment: TradingEnvironment::Demo,
                client_id: "test".into(),
                client_secret: "test".into(),
                account_id: "123".into(),
                access_token: Some("test-token".into()),
                server: "demo.ctraderapi.com".into(),
                port: 5035,
                client_id_live: None,
                client_secret_live: None,
                account_id_live: None,
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
                initial_balance: 10000.0,
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

    #[test]
    fn test_ctrader_config_from_env_reads_access_token() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        env::set_var("CTRADER_CLIENT_ID", "client_id");
        env::set_var("CTRADER_CLIENT_SECRET", "client_secret");
        env::set_var("CTRADER_ACCOUNT_ID", "account_id");
        env::set_var("CTRADER_ACCESS_TOKEN", "demo_access_token");

        let config = CTraderConfig::from_env().expect("ctrader config");
        assert_eq!(config.access_token.as_deref(), Some("demo_access_token"));

        env::remove_var("CTRADER_CLIENT_ID");
        env::remove_var("CTRADER_CLIENT_SECRET");
        env::remove_var("CTRADER_ACCOUNT_ID");
        env::remove_var("CTRADER_ACCESS_TOKEN");
    }

    #[test]
    fn test_ctrader_access_token_default_is_none() {
        let config = Config::default();
        assert!(config.ctrader.access_token.is_none());
    }
}
