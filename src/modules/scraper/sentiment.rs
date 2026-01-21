//! Sentiment analysis types and logic
//!
//! Provides sentiment scoring from -100 (extreme bearish) to +100 (extreme bullish)

use serde::{Deserialize, Serialize};

/// Sentiment type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SentimentType {
    Bullish,
    Bearish,
    Neutral,
}

/// Result of sentiment analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentResult {
    pub sentiment_type: SentimentType,
    pub score: i32, // -100 to +100
    pub confidence: f64, // 0.0 to 1.0
    pub source: String,
    pub raw_text: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SentimentResult {
    pub fn new(score: i32, source: &str) -> Self {
        let sentiment_type = match score {
            s if s > 20 => SentimentType::Bullish,
            s if s < -20 => SentimentType::Bearish,
            _ => SentimentType::Neutral,
        };

        Self {
            sentiment_type,
            score: score.clamp(-100, 100),
            confidence: 0.5,
            source: source.to_string(),
            raw_text: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_raw_text(mut self, text: String) -> Self {
        self.raw_text = Some(text);
        self
    }
}

/// Sentiment analyzer for keyword-based analysis
pub struct SentimentAnalyzer {
    bullish_keywords: Vec<String>,
    bearish_keywords: Vec<String>,
}

impl Default for SentimentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SentimentAnalyzer {
    pub fn new() -> Self {
        Self {
            bullish_keywords: vec![
                "rally".to_string(),
                "surge".to_string(),
                "breakout".to_string(),
                "bullish".to_string(),
                "higher".to_string(),
                "strength".to_string(),
                "upward".to_string(),
                "buy".to_string(),
                "long".to_string(),
                "support".to_string(),
                "demand".to_string(),
                "growth".to_string(),
                "positive".to_string(),
                "optimistic".to_string(),
                "increase".to_string(),
            ],
            bearish_keywords: vec![
                "crash".to_string(),
                "dump".to_string(),
                "bearish".to_string(),
                "lower".to_string(),
                "weakness".to_string(),
                "resistance".to_string(),
                "sell".to_string(),
                "short".to_string(),
                "downward".to_string(),
                "decline".to_string(),
                "drop".to_string(),
                "negative".to_string(),
                "pessimistic".to_string(),
                "decrease".to_string(),
                "slump".to_string(),
            ],
        }
    }

    /// Parse sentiment score from text or fallback to keyword analysis
    pub fn parse_sentiment(&self, text: &str) -> i32 {
        if let Some(score) = extract_score_hint(text) {
            return score;
        }

        self.analyze(text).score
    }

    /// Analyze text and return sentiment score
    pub fn analyze(&self, text: &str) -> SentimentResult {
        let text_lower = text.to_lowercase();

        let bullish_count = self
            .bullish_keywords
            .iter()
            .filter(|kw| text_lower.contains(kw.as_str()))
            .count();

        let bearish_count = self
            .bearish_keywords
            .iter()
            .filter(|kw| text_lower.contains(kw.as_str()))
            .count();

        let total = bullish_count + bearish_count;
        let score = if total == 0 {
            0
        } else {
            let raw_score = (bullish_count as i32 - bearish_count as i32) * 100 / total as i32;
            raw_score.clamp(-100, 100)
        };

        let confidence = if total == 0 {
            0.0
        } else {
            (total as f64 / 10.0).min(1.0)
        };

        SentimentResult::new(score, "keyword_analysis")
            .with_confidence(confidence)
            .with_raw_text(text.to_string())
    }

    /// Aggregate multiple sentiment results
    pub fn aggregate(&self, results: &[SentimentResult]) -> SentimentResult {
        if results.is_empty() {
            return SentimentResult::new(0, "aggregate");
        }

        let total_weight: f64 = results.iter().map(|r| r.confidence).sum();
        if total_weight == 0.0 {
            return SentimentResult::new(0, "aggregate");
        }

        let weighted_score: f64 = results
            .iter()
            .map(|r| r.score as f64 * r.confidence)
            .sum::<f64>()
            / total_weight;

        let avg_confidence = total_weight / results.len() as f64;

        SentimentResult::new(weighted_score.round() as i32, "aggregate")
            .with_confidence(avg_confidence)
    }
}

fn extract_score_hint(text: &str) -> Option<i32> {
    let lower = text.to_lowercase();
    if !lower.contains("score") && !lower.contains("sentiment") {
        return None;
    }

    let mut best: Option<i32> = None;
    for token in lower.split(|c: char| !c.is_ascii_digit() && c != '+' && c != '-') {
        if token.is_empty() || token == "+" || token == "-" {
            continue;
        }
        if let Ok(value) = token.parse::<i32>() {
            best = Some(value);
            break;
        }
    }

    best.map(|v| v.clamp(-100, 100))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bullish_sentiment() {
        let analyzer = SentimentAnalyzer::new();
        let result = analyzer.analyze("Palm oil prices surge on strong demand, rally expected");
        assert_eq!(result.sentiment_type, SentimentType::Bullish);
        assert!(result.score > 0);
    }

    #[test]
    fn test_bearish_sentiment() {
        let analyzer = SentimentAnalyzer::new();
        let result = analyzer.analyze("Palm oil crash continues, bearish outlook as prices drop");
        assert_eq!(result.sentiment_type, SentimentType::Bearish);
        assert!(result.score < 0);
    }

    #[test]
    fn test_neutral_sentiment() {
        let analyzer = SentimentAnalyzer::new();
        let result = analyzer.analyze("Palm oil trading at current levels");
        assert_eq!(result.sentiment_type, SentimentType::Neutral);
    }

    #[test]
    fn test_parse_sentiment_score_hint() {
        let analyzer = SentimentAnalyzer::new();
        let score = analyzer.parse_sentiment("Market update: Score: +75");
        assert_eq!(score, 75);
    }

    #[test]
    fn test_parse_sentiment_fallback() {
        let analyzer = SentimentAnalyzer::new();
        let score = analyzer.parse_sentiment("Palm oil prices surge on strong demand");
        assert!(score > 0);
    }
}
