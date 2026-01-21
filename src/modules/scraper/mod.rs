//! Sentiment analysis module
//!
//! This module provides sentiment analysis from multiple sources:
//! - Perplexity API (primary): Real-time web search for market sentiment
//! - Twitter scraping (backup): Direct KOL monitoring

pub mod perplexity;
pub mod sentiment;
pub mod twitter;

pub use perplexity::PerplexityClient;
pub use sentiment::{SentimentAnalyzer, SentimentResult, SentimentType};
pub use twitter::TwitterScraper;
