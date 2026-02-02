//! Helper functions for the Palm Oil Trading Bot
//!
//! Contains utility functions for retrying, formatting, and common operations.

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Configuration for retry logic
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay between retries (in milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (in milliseconds)
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with custom values
    pub fn new(max_retries: u32, initial_delay_ms: u64) -> Self {
        Self {
            max_retries,
            initial_delay_ms,
            ..Default::default()
        }
    }

    /// Set the maximum delay
    pub fn with_max_delay(mut self, max_delay_ms: u64) -> Self {
        self.max_delay_ms = max_delay_ms;
        self
    }

    /// Set the backoff multiplier
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }
}

/// Retry a future with exponential backoff
///
/// # Arguments
/// * `config` - Retry configuration
/// * `operation` - The async operation to retry
/// * `should_retry` - Function to determine if an error should trigger a retry
///
/// # Example
/// ```ignore
/// let result = retry_with_backoff(
///     RetryConfig::default(),
///     || async { fetch_price().await },
///     |e| !e.is_permanent(),
/// ).await;
/// ```
pub async fn retry_with_backoff<T, E, F, Fut, R>(
    config: RetryConfig,
    mut operation: F,
    should_retry: R,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    R: Fn(&E) -> bool,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    let mut delay_ms = config.initial_delay_ms;

    loop {
        attempt += 1;
        debug!("Retry attempt {}/{}", attempt, config.max_retries + 1);

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!("Operation succeeded after {} attempts", attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt > config.max_retries || !should_retry(&e) {
                    warn!(
                        "Operation failed after {} attempts: {}",
                        attempt, e
                    );
                    return Err(e);
                }

                warn!(
                    "Attempt {} failed: {}. Retrying in {}ms...",
                    attempt, e, delay_ms
                );

                sleep(Duration::from_millis(delay_ms)).await;

                // Calculate next delay with exponential backoff
                delay_ms = ((delay_ms as f64 * config.backoff_multiplier) as u64)
                    .min(config.max_delay_ms);
            }
        }
    }
}

/// Format a price value for display
///
/// # Arguments
/// * `price` - The price value
/// * `decimals` - Number of decimal places
///
/// # Example
/// ```
/// use palm_oil_bot::modules::utils::format_price;
/// assert_eq!(format_price(4832.5, 2), "4,832.50");
/// ```
pub fn format_price(price: f64, decimals: usize) -> String {
    let formatted = format!("{:.prec$}", price, prec = decimals);
    add_thousands_separator(&formatted)
}

/// Format a price with currency symbol
///
/// # Arguments
/// * `price` - The price value
/// * `currency` - Currency code (e.g., "MYR", "USD")
///
/// # Example
/// ```
/// use palm_oil_bot::modules::utils::format_currency;
/// assert_eq!(format_currency(4832.5, "MYR"), "MYR 4,832.50");
/// ```
pub fn format_currency(price: f64, currency: &str) -> String {
    format!("{} {}", currency, format_price(price, 2))
}

/// Format a percentage value
///
/// # Arguments
/// * `value` - The percentage value (e.g., 0.05 for 5%)
/// * `include_sign` - Whether to include + for positive values
///
/// # Example
/// ```
/// use palm_oil_bot::modules::utils::format_percentage;
/// assert_eq!(format_percentage(0.0243, true), "+2.43%");
/// assert_eq!(format_percentage(-0.015, true), "-1.50%");
/// ```
pub fn format_percentage(value: f64, include_sign: bool) -> String {
    let percentage = value * 100.0;
    if include_sign && percentage > 0.0 {
        format!("+{:.2}%", percentage)
    } else {
        format!("{:.2}%", percentage)
    }
}

/// Format a timestamp for display
///
/// # Arguments
/// * `timestamp` - Unix timestamp in seconds
///
/// # Example
/// ```
/// use palm_oil_bot::modules::utils::format_timestamp;
/// let formatted = format_timestamp(1705680000);
/// assert!(formatted.contains("2024"));
/// ```
pub fn format_timestamp(timestamp: i64) -> String {
    use chrono::{TimeZone, Utc};

    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid timestamp".to_string())
}

/// Format current time for logging
pub fn now_formatted() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Add thousands separator to a number string
fn add_thousands_separator(s: &str) -> String {
    let parts: Vec<&str> = s.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = parts.get(1);

    let chars: Vec<char> = integer_part.chars().collect();
    let mut result = String::new();
    let len = chars.len();
    let is_negative = chars.first() == Some(&'-');
    let start = if is_negative { 1 } else { 0 };

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && i >= start && (len - i).is_multiple_of(3) && i != start {
            result.push(',');
        }
        result.push(*c);
    }

    if let Some(decimal) = decimal_part {
        result.push('.');
        result.push_str(decimal);
    }

    result
}

/// Calculate the time elapsed since a timestamp
pub fn time_elapsed_str(timestamp: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let elapsed_secs = now - timestamp;

    if elapsed_secs < 0 {
        return "in the future".to_string();
    }

    let elapsed = elapsed_secs as u64;

    if elapsed < 60 {
        format!("{}s ago", elapsed)
    } else if elapsed < 3600 {
        format!("{}m ago", elapsed / 60)
    } else if elapsed < 86400 {
        format!("{}h ago", elapsed / 3600)
    } else {
        format!("{}d ago", elapsed / 86400)
    }
}

/// Clamp a value between min and max
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Calculate profit/loss percentage
pub fn calculate_pnl_percent(entry_price: f64, current_price: f64, is_long: bool) -> f64 {
    if entry_price == 0.0 {
        return 0.0;
    }

    if is_long {
        (current_price - entry_price) / entry_price
    } else {
        (entry_price - current_price) / entry_price
    }
}

/// Generate a unique order ID
pub fn generate_order_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    format!("PO_{}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_price() {
        assert_eq!(format_price(1234.56, 2), "1,234.56");
        assert_eq!(format_price(1000000.0, 0), "1,000,000");
        assert_eq!(format_price(42.123, 3), "42.123");
        assert_eq!(format_price(-1234.56, 2), "-1,234.56");
    }

    #[test]
    fn test_format_currency() {
        assert_eq!(format_currency(4832.5, "MYR"), "MYR 4,832.50");
        assert_eq!(format_currency(1000.0, "USD"), "USD 1,000.00");
    }

    #[test]
    fn test_format_percentage() {
        assert_eq!(format_percentage(0.0243, true), "+2.43%");
        assert_eq!(format_percentage(-0.015, true), "-1.50%");
        assert_eq!(format_percentage(0.05, false), "5.00%");
        assert_eq!(format_percentage(0.0, true), "0.00%");
    }

    #[test]
    fn test_calculate_pnl_percent() {
        // Long position profit
        assert!((calculate_pnl_percent(100.0, 102.0, true) - 0.02).abs() < 0.0001);
        // Long position loss
        assert!((calculate_pnl_percent(100.0, 98.0, true) - (-0.02)).abs() < 0.0001);
        // Short position profit
        assert!((calculate_pnl_percent(100.0, 98.0, false) - 0.02).abs() < 0.0001);
        // Short position loss
        assert!((calculate_pnl_percent(100.0, 102.0, false) - (-0.02)).abs() < 0.0001);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 0, 10), 5);
        assert_eq!(clamp(-5, 0, 10), 0);
        assert_eq!(clamp(15, 0, 10), 10);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        let mut attempts = 0;
        let result: Result<i32, &str> = retry_with_backoff(
            RetryConfig::new(3, 10),
            || {
                attempts += 1;
                async move {
                    if attempts < 2 {
                        Err("temporary error")
                    } else {
                        Ok(42)
                    }
                }
            },
            |_| true,
        )
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_exhausted() {
        let mut attempts = 0;
        let result: Result<i32, &str> = retry_with_backoff(
            RetryConfig::new(2, 10),
            || {
                attempts += 1;
                async move { Err("permanent error") }
            },
            |_| true,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 3); // Initial + 2 retries
    }
}
