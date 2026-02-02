//! Rate limiting for API calls with exponential backoff and jitter
//!
//! Prevents API bans and manages rate limits for:
//! - Perplexity API (sentiment analysis)
//! - Twitter scraping
//! - cTrader API (trading operations)

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Configuration for rate limiter
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum requests per window
    pub max_requests: usize,
    /// Time window duration
    pub window_duration: Duration,
    /// Exponential backoff base (seconds)
    pub backoff_base: f64,
    /// Maximum backoff duration (seconds)
    pub max_backoff: f64,
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_requests: 10,
            window_duration: Duration::from_secs(60),
            backoff_base: 2.0,
            max_backoff: 300.0, // 5 minutes
            jitter_factor: 0.1,
        }
    }
}

/// Request record for rate limiting
#[derive(Debug, Clone)]
struct RequestRecord {
    timestamp: Instant,
}

/// Thread-safe rate limiter with exponential backoff
pub struct ApiRateLimiter {
    pub(crate) config: RateLimiterConfig,
    requests: Arc<Mutex<Vec<RequestRecord>>>,
    pub(crate) consecutive_failures: Arc<Mutex<usize>>,
}

impl ApiRateLimiter {
    /// Create a new rate limiter with default config
    pub fn new() -> Self {
        Self::with_config(RateLimiterConfig::default())
    }

    /// Create a rate limiter with custom config
    pub fn with_config(config: RateLimiterConfig) -> Self {
        Self {
            config,
            requests: Arc::new(Mutex::new(Vec::new())),
            consecutive_failures: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a rate limiter for Perplexity API (60 requests/minute)
    pub fn for_perplexity() -> Self {
        Self::with_config(RateLimiterConfig {
            max_requests: 60,
            window_duration: Duration::from_secs(60),
            backoff_base: 2.0,
            max_backoff: 300.0,
            jitter_factor: 0.1,
        })
    }

    /// Create a rate limiter for Twitter scraping (10 requests/minute - conservative)
    pub fn for_twitter() -> Self {
        Self::with_config(RateLimiterConfig {
            max_requests: 10,
            window_duration: Duration::from_secs(60),
            backoff_base: 3.0,
            max_backoff: 600.0,
            jitter_factor: 0.2,
        })
    }

    /// Create a rate limiter for cTrader API (100 requests/second)
    pub fn for_ctrader() -> Self {
        Self::with_config(RateLimiterConfig {
            max_requests: 100,
            window_duration: Duration::from_secs(1),
            backoff_base: 1.5,
            max_backoff: 60.0,
            jitter_factor: 0.05,
        })
    }

    /// Check if a request is allowed and wait if necessary
    /// Returns true if request can proceed, false if rate limited
    pub async fn check_rate_limit(&self) -> bool {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        // Remove expired requests outside the window
        requests.retain(|record| now.duration_since(record.timestamp) < self.config.window_duration);

        // Check if we're under the limit
        if requests.len() < self.config.max_requests {
            requests.push(RequestRecord { timestamp: now });
            debug!(
                "Rate limit check PASS: {}/{} requests in {:?} window",
                requests.len(),
                self.config.max_requests,
                self.config.window_duration
            );
            true
        } else {
            warn!(
                "Rate limit EXCEEDED: {}/{} requests in {:?} window",
                requests.len(),
                self.config.max_requests,
                self.config.window_duration
            );
            false
        }
    }

    /// Wait for rate limit to be available, with exponential backoff on failures
    pub async fn wait_for_rate_limit(&self) {
        let failures = *self.consecutive_failures.lock().await;

        // Calculate backoff duration with exponential growth and jitter
        if failures > 0 {
            let backoff_secs = (self.config.backoff_base.powi(failures as i32))
                .min(self.config.max_backoff);

            // Add jitter to avoid thundering herd
            let jitter = backoff_secs * self.config.jitter_factor * rand::random::<f64>();
            let total_wait = backoff_secs + jitter;

            warn!(
                "Rate limited after {} failures. Waiting {:.2}s (base: {:.2}s + jitter: {:.2}s)",
                failures, total_wait, backoff_secs, jitter
            );

            tokio::time::sleep(Duration::from_secs_f64(total_wait)).await;
        }

        // Wait until we're under the limit
        loop {
            if self.check_rate_limit().await {
                break;
            }

            // Wait for a portion of the window before retrying
            let wait_duration = self.config.window_duration / (self.config.max_requests as u32);
            debug!("Waiting {:?} before retry", wait_duration);
            tokio::time::sleep(wait_duration).await;
        }
    }

    /// Record a successful API call (resets consecutive failures)
    pub async fn record_success(&self) {
        let mut failures = self.consecutive_failures.lock().await;
        *failures = 0;
        debug!("Request successful, reset failure count");
    }

    /// Record a failed API call (increments consecutive failures)
    pub async fn record_failure(&self) {
        let mut failures = self.consecutive_failures.lock().await;
        *failures += 1;
        warn!("Request failed, consecutive failures: {}", *failures);
    }

    /// Get current request count in window
    pub async fn current_request_count(&self) -> usize {
        let requests = self.requests.lock().await;
        let now = Instant::now();
        requests
            .iter()
            .filter(|r| now.duration_since(r.timestamp) < self.config.window_duration)
            .count()
    }

    /// Reset the rate limiter (clear all requests and failures)
    pub async fn reset(&self) {
        let mut requests = self.requests.lock().await;
        requests.clear();
        let mut failures = self.consecutive_failures.lock().await;
        *failures = 0;
        debug!("Rate limiter reset");
    }
}

impl Default for ApiRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_under_limit() {
        let limiter = ApiRateLimiter::with_config(RateLimiterConfig {
            max_requests: 3,
            window_duration: Duration::from_secs(1),
            ..Default::default()
        });

        assert!(limiter.check_rate_limit().await);
        assert!(limiter.check_rate_limit().await);
        assert!(limiter.check_rate_limit().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = ApiRateLimiter::with_config(RateLimiterConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(10),
            ..Default::default()
        });

        assert!(limiter.check_rate_limit().await);
        assert!(limiter.check_rate_limit().await);
        assert!(!limiter.check_rate_limit().await); // Should be blocked
    }

    #[tokio::test]
    async fn test_rate_limiter_window_expiry() {
        let limiter = ApiRateLimiter::with_config(RateLimiterConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100),
            ..Default::default()
        });

        assert!(limiter.check_rate_limit().await);
        assert!(limiter.check_rate_limit().await);
        assert!(!limiter.check_rate_limit().await);

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be allowed again
        assert!(limiter.check_rate_limit().await);
    }

    #[tokio::test]
    async fn test_record_success_resets_failures() {
        let limiter = ApiRateLimiter::new();

        limiter.record_failure().await;
        limiter.record_failure().await;
        assert_eq!(*limiter.consecutive_failures.lock().await, 2);

        limiter.record_success().await;
        assert_eq!(*limiter.consecutive_failures.lock().await, 0);
    }

    #[tokio::test]
    async fn test_record_failure_increments() {
        let limiter = ApiRateLimiter::new();

        limiter.record_failure().await;
        assert_eq!(*limiter.consecutive_failures.lock().await, 1);

        limiter.record_failure().await;
        assert_eq!(*limiter.consecutive_failures.lock().await, 2);
    }

    #[tokio::test]
    async fn test_current_request_count() {
        let limiter = ApiRateLimiter::with_config(RateLimiterConfig {
            max_requests: 5,
            window_duration: Duration::from_secs(1),
            ..Default::default()
        });

        assert_eq!(limiter.current_request_count().await, 0);

        limiter.check_rate_limit().await;
        assert_eq!(limiter.current_request_count().await, 1);

        limiter.check_rate_limit().await;
        assert_eq!(limiter.current_request_count().await, 2);
    }

    #[tokio::test]
    async fn test_reset_clears_state() {
        let limiter = ApiRateLimiter::with_config(RateLimiterConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(10),
            ..Default::default()
        });

        limiter.check_rate_limit().await;
        limiter.check_rate_limit().await;
        limiter.record_failure().await;

        assert!(!limiter.check_rate_limit().await); // Over limit
        assert_eq!(*limiter.consecutive_failures.lock().await, 1);

        limiter.reset().await;

        assert_eq!(limiter.current_request_count().await, 0);
        assert_eq!(*limiter.consecutive_failures.lock().await, 0);
        assert!(limiter.check_rate_limit().await); // Now allowed
    }

    #[tokio::test]
    async fn test_perplexity_config() {
        let limiter = ApiRateLimiter::for_perplexity();
        assert_eq!(limiter.config.max_requests, 60);
        assert_eq!(limiter.config.window_duration, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_twitter_config() {
        let limiter = ApiRateLimiter::for_twitter();
        assert_eq!(limiter.config.max_requests, 10);
        assert_eq!(limiter.config.window_duration, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_ctrader_config() {
        let limiter = ApiRateLimiter::for_ctrader();
        assert_eq!(limiter.config.max_requests, 100);
        assert_eq!(limiter.config.window_duration, Duration::from_secs(1));
    }
}
