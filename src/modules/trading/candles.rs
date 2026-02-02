//! Candle aggregation module
//!
//! Aggregates price ticks into OHLCV candles for different timeframes.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Timeframe for candle aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeFrame {
    /// 1 minute
    M1,
    /// 5 minutes
    M5,
    /// 15 minutes
    M15,
    /// 30 minutes
    M30,
    /// 1 hour
    H1,
    /// 4 hours
    H4,
    /// 1 day
    D1,
}

impl TimeFrame {
    /// Get the duration in seconds
    pub fn duration_secs(&self) -> i64 {
        match self {
            TimeFrame::M1 => 60,
            TimeFrame::M5 => 300,
            TimeFrame::M15 => 900,
            TimeFrame::M30 => 1800,
            TimeFrame::H1 => 3600,
            TimeFrame::H4 => 14400,
            TimeFrame::D1 => 86400,
        }
    }

    /// Get chrono Duration
    pub fn to_duration(&self) -> Duration {
        Duration::seconds(self.duration_secs())
    }

    /// Calculate the start time of the candle for a given timestamp
    pub fn candle_start(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        let duration_secs = self.duration_secs();
        let unix_timestamp = timestamp.timestamp();
        let aligned_timestamp = (unix_timestamp / duration_secs) * duration_secs;
        DateTime::from_timestamp(aligned_timestamp, 0).unwrap_or(timestamp)
    }
}

impl std::fmt::Display for TimeFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeFrame::M1 => write!(f, "1m"),
            TimeFrame::M5 => write!(f, "5m"),
            TimeFrame::M15 => write!(f, "15m"),
            TimeFrame::M30 => write!(f, "30m"),
            TimeFrame::H1 => write!(f, "1h"),
            TimeFrame::H4 => write!(f, "4h"),
            TimeFrame::D1 => write!(f, "1d"),
        }
    }
}

/// OHLCV Candle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    /// Candle start timestamp
    pub timestamp: DateTime<Utc>,
    /// Timeframe
    pub timeframe: TimeFrame,
    /// Open price
    pub open: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Close price
    pub close: f64,
    /// Volume (number of ticks aggregated)
    pub volume: u64,
}

impl Candle {
    /// Get candle end timestamp
    pub fn end_time(&self) -> DateTime<Utc> {
        self.timestamp + self.timeframe.to_duration()
    }

    /// Check if this is a bullish candle (close > open)
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Check if this is a bearish candle (close < open)
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Get candle body size
    pub fn body_size(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Get candle range (high - low)
    pub fn range(&self) -> f64 {
        self.high - self.low
    }
}

/// Price tick for aggregation
#[derive(Debug, Clone, Copy)]
pub struct Tick {
    /// Timestamp of the tick
    pub timestamp: DateTime<Utc>,
    /// Price
    pub price: f64,
}

impl Tick {
    pub fn new(timestamp: DateTime<Utc>, price: f64) -> Self {
        Self { timestamp, price }
    }
}

/// Builder for aggregating ticks into candles
#[derive(Debug)]
pub struct CandleBuilder {
    /// Timeframe for candle aggregation
    timeframe: TimeFrame,
    /// Current candle being built (if any)
    current_candle: Option<CandleInProgress>,
}

/// Candle in progress (internal state)
#[derive(Debug, Clone)]
struct CandleInProgress {
    timestamp: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: u64,
}

impl CandleInProgress {
    fn new(timestamp: DateTime<Utc>, price: f64) -> Self {
        Self {
            timestamp,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 1,
        }
    }

    fn update(&mut self, price: f64) {
        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
        self.volume += 1;
    }

    fn into_candle(self, timeframe: TimeFrame) -> Candle {
        Candle {
            timestamp: self.timestamp,
            timeframe,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
        }
    }
}

impl CandleBuilder {
    /// Create a new candle builder
    pub fn new(timeframe: TimeFrame) -> Self {
        Self {
            timeframe,
            current_candle: None,
        }
    }

    /// Add a tick and potentially complete a candle
    ///
    /// Returns `Some(Candle)` when a candle is completed, `None` otherwise
    pub fn add_tick(&mut self, tick: Tick) -> Option<Candle> {
        let candle_start = self.timeframe.candle_start(tick.timestamp);

        match &mut self.current_candle {
            None => {
                // Start new candle
                self.current_candle = Some(CandleInProgress::new(candle_start, tick.price));
                None
            }
            Some(candle) => {
                if candle.timestamp == candle_start {
                    // Same candle, update it
                    candle.update(tick.price);
                    None
                } else {
                    // New candle period started, complete the current one
                    let completed = candle.clone().into_candle(self.timeframe);
                    self.current_candle = Some(CandleInProgress::new(candle_start, tick.price));
                    Some(completed)
                }
            }
        }
    }

    /// Force complete the current candle (useful for finalization)
    pub fn flush(&mut self) -> Option<Candle> {
        self.current_candle
            .take()
            .map(|candle| candle.into_candle(self.timeframe))
    }

    /// Check if a candle is currently being built
    pub fn has_current(&self) -> bool {
        self.current_candle.is_some()
    }

    /// Get timeframe
    pub fn timeframe(&self) -> TimeFrame {
        self.timeframe
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_timeframe_duration() {
        assert_eq!(TimeFrame::M1.duration_secs(), 60);
        assert_eq!(TimeFrame::M5.duration_secs(), 300);
        assert_eq!(TimeFrame::M15.duration_secs(), 900);
        assert_eq!(TimeFrame::H1.duration_secs(), 3600);
    }

    #[test]
    fn test_timeframe_candle_start() {
        let tf = TimeFrame::M5;
        
        // 10:03:45 -> should align to 10:00:00
        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 3, 45).unwrap();
        let start = tf.candle_start(ts);
        assert_eq!(start, Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap());

        // 10:07:30 -> should align to 10:05:00
        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 7, 30).unwrap();
        let start = tf.candle_start(ts);
        assert_eq!(start, Utc.with_ymd_and_hms(2024, 1, 1, 10, 5, 0).unwrap());
    }

    #[test]
    fn test_candle_builder_single_tick() {
        let mut builder = CandleBuilder::new(TimeFrame::M1);
        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 30).unwrap();
        
        let result = builder.add_tick(Tick::new(ts, 100.0));
        assert!(result.is_none()); // No candle completed yet
        
        // Current candle should exist
        assert!(builder.has_current());
    }

    #[test]
    fn test_candle_builder_multiple_ticks_same_period() {
        let mut builder = CandleBuilder::new(TimeFrame::M1);
        let base_ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();

        // All ticks in same minute
        builder.add_tick(Tick::new(base_ts, 100.0));
        builder.add_tick(Tick::new(base_ts + Duration::seconds(10), 105.0));
        builder.add_tick(Tick::new(base_ts + Duration::seconds(20), 98.0));
        builder.add_tick(Tick::new(base_ts + Duration::seconds(30), 102.0));

        // Flush to get the candle
        let candle = builder.flush().unwrap();
        
        assert_eq!(candle.open, 100.0);
        assert_eq!(candle.high, 105.0);
        assert_eq!(candle.low, 98.0);
        assert_eq!(candle.close, 102.0);
        assert_eq!(candle.volume, 4);
    }

    #[test]
    fn test_candle_builder_completes_on_new_period() {
        let mut builder = CandleBuilder::new(TimeFrame::M1);
        let base_ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();

        // Ticks in first minute
        builder.add_tick(Tick::new(base_ts, 100.0));
        builder.add_tick(Tick::new(base_ts + Duration::seconds(30), 102.0));

        // Tick in next minute triggers candle completion
        let completed = builder.add_tick(Tick::new(base_ts + Duration::seconds(60), 105.0));

        assert!(completed.is_some());
        let candle = completed.unwrap();
        
        assert_eq!(candle.open, 100.0);
        assert_eq!(candle.close, 102.0);
        assert_eq!(candle.volume, 2);
        assert_eq!(candle.timeframe, TimeFrame::M1);
    }

    #[test]
    fn test_candle_builder_m5_timeframe() {
        let mut builder = CandleBuilder::new(TimeFrame::M5);
        let base_ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();

        // Ticks in first 5-minute period
        builder.add_tick(Tick::new(base_ts, 100.0));
        builder.add_tick(Tick::new(base_ts + Duration::minutes(2), 105.0));
        builder.add_tick(Tick::new(base_ts + Duration::minutes(4), 102.0));

        // Still in same 5-min period
        let result = builder.add_tick(Tick::new(base_ts + Duration::minutes(4) + Duration::seconds(30), 103.0));
        assert!(result.is_none());

        // New 5-min period triggers completion
        let completed = builder.add_tick(Tick::new(base_ts + Duration::minutes(5), 110.0));
        assert!(completed.is_some());
        
        let candle = completed.unwrap();
        assert_eq!(candle.open, 100.0);
        assert_eq!(candle.high, 105.0);
        assert_eq!(candle.close, 103.0);
        assert_eq!(candle.volume, 4);
    }

    #[test]
    fn test_candle_is_bullish_bearish() {
        let bullish = Candle {
            timestamp: Utc::now(),
            timeframe: TimeFrame::M1,
            open: 100.0,
            high: 105.0,
            low: 98.0,
            close: 103.0,
            volume: 10,
        };
        assert!(bullish.is_bullish());
        assert!(!bullish.is_bearish());

        let bearish = Candle {
            timestamp: Utc::now(),
            timeframe: TimeFrame::M1,
            open: 100.0,
            high: 105.0,
            low: 95.0,
            close: 97.0,
            volume: 10,
        };
        assert!(bearish.is_bearish());
        assert!(!bearish.is_bullish());
    }

    #[test]
    fn test_candle_body_and_range() {
        let candle = Candle {
            timestamp: Utc::now(),
            timeframe: TimeFrame::M1,
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
            volume: 10,
        };

        assert_eq!(candle.body_size(), 5.0);
        assert_eq!(candle.range(), 15.0);
    }

    #[test]
    fn test_timeframe_display() {
        assert_eq!(format!("{}", TimeFrame::M1), "1m");
        assert_eq!(format!("{}", TimeFrame::M5), "5m");
        assert_eq!(format!("{}", TimeFrame::H1), "1h");
        assert_eq!(format!("{}", TimeFrame::D1), "1d");
    }

    #[test]
    fn test_multiple_candles_sequence() {
        let mut builder = CandleBuilder::new(TimeFrame::M1);
        let base_ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();

        let mut completed_candles = Vec::new();

        // Generate ticks across 3 minutes
        for minute in 0..3 {
            for second in 0..60 {
                let ts = base_ts + Duration::minutes(minute) + Duration::seconds(second);
                let price = 100.0 + (minute as f64) * 5.0;
                
                if let Some(candle) = builder.add_tick(Tick::new(ts, price)) {
                    completed_candles.push(candle);
                }
            }
        }

        // Should have completed 2 candles (minute 0->1, minute 1->2)
        // Minute 2 is still in progress
        assert_eq!(completed_candles.len(), 2);
        assert_eq!(completed_candles[0].open, 100.0);
        assert_eq!(completed_candles[1].open, 105.0);
    }

    #[test]
    fn test_flush_current_candle() {
        let mut builder = CandleBuilder::new(TimeFrame::M1);
        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();

        builder.add_tick(Tick::new(ts, 100.0));
        builder.add_tick(Tick::new(ts + Duration::seconds(30), 105.0));

        // Flush should return current candle
        let candle = builder.flush();
        assert!(candle.is_some());
        
        let candle = candle.unwrap();
        assert_eq!(candle.open, 100.0);
        assert_eq!(candle.close, 105.0);
        
        // After flush, no current candle
        assert!(!builder.has_current());
    }

    #[test]
    fn test_candle_end_time() {
        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
        let candle = Candle {
            timestamp: ts,
            timeframe: TimeFrame::M5,
            open: 100.0,
            high: 105.0,
            low: 98.0,
            close: 102.0,
            volume: 10,
        };

        let expected_end = ts + Duration::minutes(5);
        assert_eq!(candle.end_time(), expected_end);
    }
}
