//! Bot modules
//!
//! This module contains all the core functionality:
//! - `scraper`: Sentiment analysis from Perplexity API and Twitter
//! - `trading`: cTrader API client and trading logic
//! - `monitoring`: Dashboard and metrics
//! - `utils`: Helper functions

pub mod monitoring;
pub mod scraper;
pub mod trading;
pub mod utils;
