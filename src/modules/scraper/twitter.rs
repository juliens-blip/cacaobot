//! Twitter scraper for backup sentiment analysis
//!
//! Scrapes tweets from KOL accounts when Perplexity API is rate limited.
//! Uses guest access (no API key required) but may be blocked.

use crate::error::{BotError, Result};
use crate::modules::scraper::sentiment::{SentimentAnalyzer, SentimentResult};
use crate::modules::security::ApiRateLimiter;
use scraper::{Html, Selector};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Tweet data structure
#[derive(Debug, Clone)]
pub struct Tweet {
    pub username: String,
    pub text: String,
    pub timestamp: Option<String>,
}

/// Twitter scraper for KOL accounts
pub struct TwitterScraper {
    client: reqwest::Client,
    kols: Vec<String>,
    sentiment_analyzer: SentimentAnalyzer,
    rate_limiter: Arc<ApiRateLimiter>,
}

impl TwitterScraper {
    /// Create a new Twitter scraper
    pub fn new(kols: Vec<String>, rate_limiter: Arc<ApiRateLimiter>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|e| {
                warn!("Failed to build HTTP client: {}. Falling back to default client.", e);
                reqwest::Client::new()
            });

        Self {
            client,
            kols,
            sentiment_analyzer: SentimentAnalyzer::new(),
            rate_limiter,
        }
    }

    /// Scrape tweets from all KOL accounts
    pub async fn scrape_all(&self) -> Result<Vec<Tweet>> {
        let mut all_tweets = Vec::new();

        for kol in &self.kols {
            match self.scrape_user(kol).await {
                Ok(tweets) => {
                    info!("Scraped {} tweets from @{}", tweets.len(), kol);
                    all_tweets.extend(tweets);
                }
                Err(e) => {
                    warn!("Failed to scrape @{}: {}", kol, e);
                }
            }
        }

        Ok(all_tweets)
    }

    /// Scrape tweets from a single user (guest mode)
    async fn scrape_user(&self, username: &str) -> Result<Vec<Tweet>> {
        // Note: Twitter guest scraping is unreliable and may be blocked
        // This is a backup method when Perplexity is rate limited

        // Wait for rate limit before making request
        self.rate_limiter.wait_for_rate_limit().await;

        let url = format!("https://nitter.net/{}", username);
        debug!("Scraping tweets from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| BotError::Twitter(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            warn!("Twitter scrape failed: {}", response.status());
            self.rate_limiter.record_failure().await;
            return Err(BotError::Twitter(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        // Record success
        self.rate_limiter.record_success().await;

        let html = response
            .text()
            .await
            .map_err(|e| BotError::Twitter(format!("Failed to read response: {}", e)))?;

        self.parse_tweets(&html, username)
    }

    /// Parse tweets from HTML (Nitter format)
    fn parse_tweets(&self, html: &str, username: &str) -> Result<Vec<Tweet>> {
        let document = Html::parse_document(html);
        let tweet_selector = Selector::parse(".timeline-item .tweet-content")
            .map_err(|e| BotError::Twitter(format!("Invalid selector: {}", e)))?;

        let tweets: Vec<Tweet> = document
            .select(&tweet_selector)
            .take(10) // Limit to 10 most recent
            .map(|element| Tweet {
                username: username.to_string(),
                text: element.text().collect::<String>().trim().to_string(),
                timestamp: None,
            })
            .filter(|t| !t.text.is_empty())
            .collect();

        Ok(tweets)
    }

    /// Get aggregated sentiment from all scraped tweets
    pub async fn get_sentiment(&self) -> Result<SentimentResult> {
        let tweets = self.scrape_all().await?;

        if tweets.is_empty() {
            warn!("No tweets scraped, returning neutral sentiment");
            return Ok(SentimentResult::new(0, "twitter").with_confidence(0.1));
        }

        // Analyze each tweet
        let results: Vec<SentimentResult> = tweets
            .iter()
            .map(|tweet| self.sentiment_analyzer.analyze(&tweet.text))
            .collect();

        // Aggregate results
        let aggregated = self.sentiment_analyzer.aggregate(&results);
        let result = SentimentResult::new(aggregated.score, "twitter")
            .with_confidence(aggregated.confidence * 0.8); // Lower confidence for scraped data

        info!(
            "Twitter sentiment from {} tweets: {:?} (score: {})",
            tweets.len(),
            result.sentiment_type,
            result.score
        );

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_tweets() {
        let rate_limiter = Arc::new(ApiRateLimiter::for_twitter());
        let scraper = TwitterScraper::new(vec!["test".to_string()], rate_limiter);
        let html = "<html><body></body></html>";
        let tweets = scraper.parse_tweets(html, "test").unwrap();
        assert!(tweets.is_empty());
    }
}
