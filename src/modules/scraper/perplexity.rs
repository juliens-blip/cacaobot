//! Perplexity API client for real-time sentiment analysis
//!
//! Uses the Perplexity Sonar model to search the web for current market sentiment.

use crate::config::PerplexityConfig;
use crate::error::{BotError, PerplexityError, Result};
use crate::modules::scraper::sentiment::{SentimentAnalyzer, SentimentResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// Perplexity API client
pub struct PerplexityClient {
    client: reqwest::Client,
    config: PerplexityConfig,
    sentiment_analyzer: SentimentAnalyzer,
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
    /// Create a new Perplexity API client
    pub fn new(config: PerplexityConfig) -> Self {
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
        }
    }

    /// Query Perplexity for current palm oil market sentiment
    pub async fn get_market_sentiment(&self) -> Result<SentimentResult> {
        let prompt = r#"Analyze the current market sentiment for FCPO (Crude Palm Oil Futures) and Malaysian palm oil market.

Search for:
1. Recent news about palm oil prices
2. Export/import data
3. Weather conditions affecting production
4. Government policies
5. Social media sentiment from traders

Based on your analysis, provide:
1. A sentiment score from -100 (extremely bearish) to +100 (extremely bullish)
2. Key factors driving the sentiment
3. Confidence level (low/medium/high)

Format your response as:
SENTIMENT_SCORE: [number]
CONFIDENCE: [low/medium/high]
SUMMARY: [brief explanation]"#;

        let response = self.query(prompt).await?;
        self.parse_sentiment_response(&response)
    }

    /// Query Perplexity with a custom prompt
    pub async fn query(&self, prompt: &str) -> Result<String> {
        let system_prompt = "You are a commodities market analyst specializing in palm oil futures (FCPO). \
            Provide concise, data-driven analysis. Always include a numerical sentiment score.";

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
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
            warn!("Perplexity API rate limited");
            return Err(BotError::Perplexity(PerplexityError::RateLimited));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED {
            error!("Invalid Perplexity API key");
            return Err(BotError::Perplexity(PerplexityError::InvalidApiKey));
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Perplexity API error: {} - {}", status, error_text);
            return Err(BotError::Perplexity(PerplexityError::RequestFailed(
                format!("{}: {}", status, error_text),
            )));
        }

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sentiment_bullish() {
        let config = PerplexityConfig {
            api_key: "test".to_string(),
            endpoint: "https://api.perplexity.ai".to_string(),
            model: "sonar".to_string(),
        };
        let client = PerplexityClient::new(config);

        let response = "SENTIMENT_SCORE: 65\nCONFIDENCE: high\nSUMMARY: Strong bullish outlook";
        let result = client.parse_sentiment_response(response).unwrap();

        assert_eq!(result.score, 65);
        assert!(result.confidence > 0.8);
    }

    #[test]
    fn test_parse_sentiment_bearish() {
        let config = PerplexityConfig {
            api_key: "test".to_string(),
            endpoint: "https://api.perplexity.ai".to_string(),
            model: "sonar".to_string(),
        };
        let client = PerplexityClient::new(config);

        let response = "SENTIMENT_SCORE: -40\nCONFIDENCE: medium\nSUMMARY: Bearish due to oversupply";
        let result = client.parse_sentiment_response(response).unwrap();

        assert_eq!(result.score, -40);
    }
}
