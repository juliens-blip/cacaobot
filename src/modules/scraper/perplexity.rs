//! Perplexity API client for real-time sentiment analysis
//!
//! Uses the Perplexity Sonar model to search the web for current market sentiment.
//! Includes in-memory caching with TTL to avoid rate limits and reduce API costs.

use crate::config::PerplexityConfig;
use crate::error::{BotError, PerplexityError, Result};
use crate::modules::scraper::sentiment::{SentimentAnalyzer, SentimentResult};
use crate::modules::scraper::sentiment_cache::SentimentCache;
use crate::modules::security::ApiRateLimiter;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Build a market sentiment prompt dynamically based on the trading symbol
fn build_sentiment_prompt(symbol: &str) -> String {
    format!(
        r#"Analyze the SHORT-TERM market sentiment for {symbol} RIGHT NOW (next 5-30 minutes).

Focus on INTRADAY factors only:
1. Latest price action and intraday technical levels for {symbol} (support/resistance, recent candles)
2. Breaking news or events in the last 1-2 hours affecting {symbol}
3. Real-time trader sentiment: X/Twitter, StockTwits, trading forums - what are traders saying RIGHT NOW
4. Intraday order flow, momentum, and volatility
5. Correlated markets current direction (USD index, yields, related pairs)

IMPORTANT: Ignore long-term fundamentals. Only consider what moves the price in the NEXT 30 MINUTES.
Be aggressive. Even small edges matter. Take a clear directional stance.

Provide:
1. A sentiment score from -100 (extremely bearish) to +100 (extremely bullish). Do NOT default to 0.
2. Key intraday factors (max 3)
3. Confidence level (low/medium/high)

Format your response as:
SENTIMENT_SCORE: [number]
CONFIDENCE: [low/medium/high]
SOCIAL_SENTIMENT: [bullish/bearish/neutral + 1 sentence]
SUMMARY: [1-2 sentences on immediate direction]"#
    )
}

/// Perplexity API client with caching
pub struct PerplexityClient {
    client: reqwest::Client,
    config: PerplexityConfig,
    sentiment_analyzer: SentimentAnalyzer,
    cache: SentimentCache,
    rate_limiter: Arc<ApiRateLimiter>,
    symbol: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl PerplexityClient {
    /// Create a new Perplexity API client with default cache (5 min TTL)
    pub fn new(config: PerplexityConfig, rate_limiter: Arc<ApiRateLimiter>) -> Self {
        Self::with_cache(config, SentimentCache::new(), rate_limiter)
    }

    /// Create a new Perplexity API client with symbol and default cache
    pub fn with_symbol(config: PerplexityConfig, rate_limiter: Arc<ApiRateLimiter>, symbol: &str) -> Self {
        let mut client = Self::with_cache(config, SentimentCache::new(), rate_limiter);
        client.symbol = symbol.to_string();
        client
    }

    /// Create a new Perplexity API client with custom cache
    pub fn with_cache(config: PerplexityConfig, cache: SentimentCache, rate_limiter: Arc<ApiRateLimiter>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|e| {
                warn!("Failed to build HTTP client: {}. Falling back to default client.", e);
                reqwest::Client::new()
            });

        Self {
            client,
            config,
            sentiment_analyzer: SentimentAnalyzer::new(),
            cache,
            rate_limiter,
            symbol: "SUGARRAW".to_string(),
        }
    }

    /// Get cached sentiment or fetch from API if cache miss/expired
    pub async fn get_cached_sentiment(&self) -> Result<SentimentResult> {
        let prompt = build_sentiment_prompt(&self.symbol);
        if let Some(score) = self.cache.get(&prompt) {
            info!("Using cached sentiment for {} (score: {})", self.symbol, score);
            return Ok(SentimentResult::new(score, "perplexity_cache"));
        }

        info!("Cache miss - fetching fresh sentiment for {} from Perplexity API", self.symbol);
        let result = self.get_market_sentiment_uncached().await?;
        self.cache.set(&prompt, result.score);
        Ok(result)
    }

    /// Query Perplexity for current market sentiment (bypasses cache)
    pub async fn get_market_sentiment(&self) -> Result<SentimentResult> {
        self.get_cached_sentiment().await
    }

    /// Direct API call without cache (for testing or force refresh)
    pub async fn get_market_sentiment_uncached(&self) -> Result<SentimentResult> {
        let prompt = build_sentiment_prompt(&self.symbol);
        let response = self.query(&prompt).await?;
        self.parse_sentiment_response(&response)
    }

    /// Query Perplexity with a custom prompt
    pub async fn query(&self, prompt: &str) -> Result<String> {
        // Wait for rate limit before making request
        self.rate_limiter.wait_for_rate_limit().await;

        let system_prompt = format!(
            "You are an aggressive commodities trader specializing in {}. \
            You look for short-term momentum plays and take decisive positions. \
            Explicitly incorporate social/trader sentiment from X/Twitter, StockTwits, Reddit, and trading forums. \
            Provide concise, data-driven analysis. Always include a numerical sentiment score. \
            Never sit on the fence - if there's any edge, take a strong directional view.",
            self.symbol
        );

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.clone(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(500),
        };

        debug!("Sending request to Perplexity API");

        let response = self
            .client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Perplexity API request failed: {}", e);
                BotError::Perplexity(PerplexityError::RequestFailed(e.to_string()))
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Perplexity API rate limited (429)");
            self.rate_limiter.record_failure().await;
            return Err(BotError::Perplexity(PerplexityError::RateLimited));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            error!("Invalid Perplexity API key");
            return Err(BotError::Perplexity(PerplexityError::InvalidApiKey));
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Perplexity API error: {} - {}", status, error_text);
            self.rate_limiter.record_failure().await;
            return Err(BotError::Perplexity(PerplexityError::RequestFailed(
                format!("{}: {}", status, error_text),
            )));
        }

        // Record success for successful responses
        self.rate_limiter.record_success().await;

        let chat_response: ChatResponse = response.json().await.map_err(|e| {
            error!("Failed to parse Perplexity response: {}", e);
            BotError::Perplexity(PerplexityError::ParseError(e.to_string()))
        })?;

        if let Some(usage) = &chat_response.usage {
            debug!(
                "Perplexity API tokens used: {} (prompt: {}, completion: {})",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
            );
        }

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| {
                BotError::Perplexity(PerplexityError::ParseError(
                    "No response content".to_string(),
                ))
            })?;

        info!("Received Perplexity response ({} chars)", content.len());
        Ok(content)
    }

    /// Parse the sentiment score from Perplexity's response
    fn parse_sentiment_response(&self, response: &str) -> Result<SentimentResult> {
        // Try to extract structured sentiment score
        let score_regex = Regex::new(r"SENTIMENT_SCORE:\s*(-?\d+)").map_err(|e| {
            BotError::Perplexity(PerplexityError::ParseError(e.to_string()))
        })?;
        let confidence_regex = Regex::new(r"CONFIDENCE:\s*(low|medium|high)").map_err(|e| {
            BotError::Perplexity(PerplexityError::ParseError(e.to_string()))
        })?;

        let score = if let Some(caps) = score_regex.captures(response) {
            caps.get(1)
                .and_then(|m| m.as_str().parse::<i32>().ok())
                .unwrap_or(0)
        } else {
            // Fallback to keyword analysis
            let fallback = self.sentiment_analyzer.analyze(response);
            fallback.score
        };

        let confidence = if let Some(caps) = confidence_regex.captures(response) {
            match caps.get(1).map(|m| m.as_str()) {
                Some("high") => 0.9,
                Some("medium") => 0.6,
                Some("low") => 0.3,
                _ => 0.5,
            }
        } else {
            0.5
        };

        let result = SentimentResult::new(score.clamp(-100, 100), "perplexity")
            .with_confidence(confidence)
            .with_raw_text(response.to_string());

        info!(
            "Perplexity sentiment: {:?} (score: {}, confidence: {:.1})",
            result.sentiment_type, result.score, result.confidence
        );

        Ok(result)
    }

    /// Get reference to the cache
    pub fn cache(&self) -> &SentimentCache {
        &self.cache
    }

    /// Clear the cache (useful for testing or force refresh)
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PerplexityConfig {
        PerplexityConfig {
            api_key: "test".to_string(),
            endpoint: "https://api.perplexity.ai".to_string(),
            model: "sonar".to_string(),
        }
    }

    #[test]
    fn test_parse_sentiment_bullish() {
        let rate_limiter = Arc::new(ApiRateLimiter::for_perplexity());
        let client = PerplexityClient::new(test_config(), rate_limiter);

        let response = "SENTIMENT_SCORE: 65\nCONFIDENCE: high\nSUMMARY: Strong bullish outlook";
        let result = client.parse_sentiment_response(response).unwrap();

        assert_eq!(result.score, 65);
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_parse_sentiment_bearish() {
        let rate_limiter = Arc::new(ApiRateLimiter::for_perplexity());
        let client = PerplexityClient::new(test_config(), rate_limiter);

        let response = "SENTIMENT_SCORE: -40\nCONFIDENCE: medium\nSUMMARY: Bearish due to oversupply";
        let result = client.parse_sentiment_response(response).unwrap();

        assert_eq!(result.score, -40);
    }

    #[test]
    fn test_client_with_custom_cache() {
        let cache = SentimentCache::new();
        let rate_limiter = Arc::new(ApiRateLimiter::for_perplexity());
        let client = PerplexityClient::with_cache(test_config(), cache, rate_limiter);

        assert_eq!(client.cache().get(&build_sentiment_prompt("SUGARRAW")), None);
    }
}
