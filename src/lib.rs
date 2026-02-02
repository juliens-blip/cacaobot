//! Palm Oil Trading Bot Library
//!
//! Automated trading bot for Palm Oil CFDs (FCPO) via cTrader Open API.
//! Uses RSI technical analysis combined with sentiment from Perplexity API.

pub mod bot;
pub mod config;
pub mod error;
pub mod modules;

pub use bot::{TradingBot, SentimentCache};
pub use config::Config;
pub use error::{BotError, CTraderError, PerplexityError, Result};
