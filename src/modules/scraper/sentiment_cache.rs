//! Sentiment cache with TTL for Perplexity requests.
//!
//! Stores sentiment scores keyed by query strings to limit API calls.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::info;

const DEFAULT_TTL: Duration = Duration::from_secs(300);

/// Thread-safe in-memory sentiment cache with TTL.
#[derive(Debug, Clone)]
pub struct SentimentCache {
    state: Arc<RwLock<HashMap<String, (i32, Instant)>>>,
    ttl: Duration,
}

impl SentimentCache {
    /// Create a new cache with default TTL (5 minutes).
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_TTL)
    }

    /// Create a new cache with a custom TTL.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Get cached sentiment score for a query if present and not expired.
    pub fn get(&self, query: &str) -> Option<i32> {
        let mut state = self.state.write().ok()?;
        match state.get(query).copied() {
            Some((score, inserted_at)) => {
                if inserted_at.elapsed() <= self.ttl {
                    info!("Sentiment cache hit for query: {}", query);
                    Some(score)
                } else {
                    state.remove(query);
                    info!("Sentiment cache miss (expired) for query: {}", query);
                    None
                }
            }
            None => {
                info!("Sentiment cache miss for query: {}", query);
                None
            }
        }
    }

    /// Store sentiment score in cache.
    pub fn set(&self, query: &str, score: i32) {
        if let Ok(mut state) = self.state.write() {
            state.insert(query.to_string(), (score, Instant::now()));
        }
    }

    /// Clear all entries.
    pub fn clear(&self) {
        if let Ok(mut state) = self.state.write() {
            state.clear();
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.state.read().map(|s| s.len()).unwrap_or(0)
    }
}

impl Default for SentimentCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let cache = SentimentCache::with_ttl(Duration::from_secs(300));
        cache.set("palm oil", 42);
        assert_eq!(cache.get("palm oil"), Some(42));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = SentimentCache::with_ttl(Duration::from_secs(300));
        assert_eq!(cache.get("missing"), None);
    }

    #[test]
    fn test_cache_expiry() {
        let cache = SentimentCache::with_ttl(Duration::from_millis(50));
        cache.set("short", 10);
        assert_eq!(cache.get("short"), Some(10));
        std::thread::sleep(Duration::from_millis(75));
        assert_eq!(cache.get("short"), None);
        assert_eq!(cache.len(), 0);
    }
}
