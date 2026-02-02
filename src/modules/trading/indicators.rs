//! Technical indicators module
//!
//! Provides technical indicators: RSI, EMA, MACD, Bollinger Bands, ATR

use std::collections::VecDeque;
use tracing::debug;

/// Price data point
#[derive(Debug, Clone, Copy)]
pub struct PricePoint {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl PricePoint {
    pub fn new(close: f64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            timestamp: now,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
        }
    }
}

/// RSI (Relative Strength Index) calculator
///
/// RSI = 100 - (100 / (1 + RS))
/// RS = Average Gain / Average Loss
pub struct RsiCalculator {
    period: usize,
    prices: VecDeque<f64>,
    avg_gain: Option<f64>,
    avg_loss: Option<f64>,
}

impl RsiCalculator {
    /// Create a new RSI calculator with the specified period
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prices: VecDeque::with_capacity(period + 1),
            avg_gain: None,
            avg_loss: None,
        }
    }

    /// Add a new price and calculate RSI
    pub fn add_price(&mut self, price: f64) -> Option<f64> {
        self.prices.push_back(price);

        // Keep only period + 1 prices
        while self.prices.len() > self.period + 1 {
            self.prices.pop_front();
        }

        self.calculate()
    }

    /// Calculate RSI from current prices
    fn calculate(&mut self) -> Option<f64> {
        if self.prices.len() < self.period + 1 {
            debug!(
                "RSI: Not enough data ({}/{})",
                self.prices.len(),
                self.period + 1
            );
            return None;
        }

        let prices: Vec<f64> = self.prices.iter().copied().collect();

        // Calculate price changes
        let changes: Vec<f64> = prices.windows(2).map(|w| w[1] - w[0]).collect();

        // Separate gains and losses
        let gains: Vec<f64> = changes.iter().map(|&c| if c > 0.0 { c } else { 0.0 }).collect();
        let losses: Vec<f64> = changes
            .iter()
            .map(|&c| if c < 0.0 { -c } else { 0.0 })
            .collect();

        // Calculate average gain/loss using Wilder's smoothing
        let (avg_gain, avg_loss) = if let (Some(prev_avg_gain), Some(prev_avg_loss)) =
            (self.avg_gain, self.avg_loss)
        {
            // Subsequent calculations - Wilder's smoothing
            let last_gain = *gains.last().unwrap_or(&0.0);
            let last_loss = *losses.last().unwrap_or(&0.0);

            let avg_gain =
                (prev_avg_gain * (self.period - 1) as f64 + last_gain) / self.period as f64;
            let avg_loss =
                (prev_avg_loss * (self.period - 1) as f64 + last_loss) / self.period as f64;
            (avg_gain, avg_loss)
        } else {
            // First calculation - simple average
            let avg_gain = gains.iter().sum::<f64>() / self.period as f64;
            let avg_loss = losses.iter().sum::<f64>() / self.period as f64;
            (avg_gain, avg_loss)
        };

        self.avg_gain = Some(avg_gain);
        self.avg_loss = Some(avg_loss);

        // Calculate RSI
        let rsi = if avg_loss == 0.0 {
            100.0 // No losses = max RSI
        } else if avg_gain == 0.0 {
            0.0 // No gains = min RSI
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - (100.0 / (1.0 + rs))
        };

        debug!(
            "RSI calculation: avg_gain={:.4}, avg_loss={:.4}, RSI={:.2}",
            avg_gain, avg_loss, rsi
        );

        Some(rsi)
    }

    /// Get the current RSI value without adding a new price
    pub fn current(&self) -> Option<f64> {
        if self.avg_gain.is_none() || self.avg_loss.is_none() {
            return None;
        }

        let (avg_gain, avg_loss) = match (self.avg_gain, self.avg_loss) {
            (Some(gain), Some(loss)) => (gain, loss),
            _ => return None,
        };

        if avg_loss == 0.0 {
            Some(100.0)
        } else if avg_gain == 0.0 {
            Some(0.0)
        } else {
            let rs = avg_gain / avg_loss;
            Some(100.0 - (100.0 / (1.0 + rs)))
        }
    }

    /// Reset the calculator
    pub fn reset(&mut self) {
        self.prices.clear();
        self.avg_gain = None;
        self.avg_loss = None;
    }

    /// Check if we have enough data for RSI calculation
    pub fn is_ready(&self) -> bool {
        self.prices.len() > self.period
    }

    /// Get the number of prices stored
    pub fn len(&self) -> usize {
        self.prices.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }
}

/// Determines if the RSI indicates oversold condition
pub fn is_oversold(rsi: f64, threshold: f64) -> bool {
    rsi < threshold
}

/// Determines if the RSI indicates overbought condition
pub fn is_overbought(rsi: f64, threshold: f64) -> bool {
    rsi > threshold
}

/// EMA (Exponential Moving Average) calculator
///
/// EMA = Price(t) * k + EMA(y) * (1 - k)
/// where k = 2 / (N + 1), N = period
#[derive(Debug)]
pub struct EmaCalculator {
    period: usize,
    multiplier: f64,
    current_ema: Option<f64>,
    prices_count: usize,
    initial_sum: f64,
}

impl EmaCalculator {
    /// Create a new EMA calculator with the specified period
    pub fn new(period: usize) -> Self {
        Self {
            period,
            multiplier: 2.0 / (period as f64 + 1.0),
            current_ema: None,
            prices_count: 0,
            initial_sum: 0.0,
        }
    }

    /// Add a new price and calculate EMA
    pub fn add_price(&mut self, price: f64) -> Option<f64> {
        self.prices_count += 1;

        if self.current_ema.is_none() {
            // Accumulate prices for initial SMA
            self.initial_sum += price;

            if self.prices_count >= self.period {
                // First EMA = SMA of first N prices
                let sma = self.initial_sum / self.period as f64;
                self.current_ema = Some(sma);
            }
        } else {
            // EMA = Price * k + EMA(prev) * (1 - k)
            if let Some(prev_ema) = self.current_ema {
                let new_ema = price * self.multiplier + prev_ema * (1.0 - self.multiplier);
                self.current_ema = Some(new_ema);
            }
        }

        self.current_ema
    }

    /// Get current EMA value
    pub fn current(&self) -> Option<f64> {
        self.current_ema
    }

    /// Check if EMA is ready
    pub fn is_ready(&self) -> bool {
        self.current_ema.is_some()
    }

    /// Reset the calculator
    pub fn reset(&mut self) {
        self.current_ema = None;
        self.prices_count = 0;
        self.initial_sum = 0.0;
    }

    /// Alias for add_price (compatibility with MACD/ATR)
    pub fn update(&mut self, price: f64) -> Option<f64> {
        self.add_price(price)
    }
}

/// Trend direction based on price vs EMA
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// Price above EMA - uptrend
    Up,
    /// Price below EMA - downtrend
    Down,
    /// Price at EMA or EMA not ready
    Neutral,
}

impl Trend {
    /// Determine trend from price and EMA
    pub fn from_price_ema(price: f64, ema: Option<f64>) -> Self {
        match ema {
            Some(ema_val) if price > ema_val * 1.001 => Trend::Up,   // 0.1% buffer
            Some(ema_val) if price < ema_val * 0.999 => Trend::Down, // 0.1% buffer
            _ => Trend::Neutral,
        }
    }

    /// Check if trend allows buying
    pub fn allows_buy(&self) -> bool {
        matches!(self, Trend::Up | Trend::Neutral)
    }

    /// Check if trend allows selling
    pub fn allows_sell(&self) -> bool {
        matches!(self, Trend::Down | Trend::Neutral)
    }
}

/// MACD values
#[derive(Debug, Clone, Copy)]
pub struct MacdValues {
    pub macd_line: f64,
    pub signal_line: f64,
    pub histogram: f64,
}

/// MACD (Moving Average Convergence Divergence) calculator
pub struct MacdCalculator {
    fast_ema: EmaCalculator,
    slow_ema: EmaCalculator,
    signal_ema: EmaCalculator,
}

impl MacdCalculator {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            fast_ema: EmaCalculator::new(fast),
            slow_ema: EmaCalculator::new(slow),
            signal_ema: EmaCalculator::new(signal),
        }
    }

    pub fn update(&mut self, price: f64) -> Option<MacdValues> {
        let fast = self.fast_ema.update(price);
        let slow = self.slow_ema.update(price);
        let (fast, slow) = match (fast, slow) {
            (Some(fast), Some(slow)) => (fast, slow),
            _ => return None,
        };
        let macd_line = fast - slow;
        let signal_line = self.signal_ema.update(macd_line)?;
        let histogram = macd_line - signal_line;

        Some(MacdValues {
            macd_line,
            signal_line,
            histogram,
        })
    }
}

/// Bollinger Bands values
#[derive(Debug, Clone, Copy)]
pub struct BbValues {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

/// Bollinger Bands calculator
pub struct BollingerBands {
    period: usize,
    std_dev: f64,
    prices: VecDeque<f64>,
}

impl BollingerBands {
    pub fn new(period: usize, std_dev: f64) -> Self {
        Self {
            period,
            std_dev,
            prices: VecDeque::with_capacity(period),
        }
    }

    pub fn update(&mut self, price: f64) -> Option<BbValues> {
        self.prices.push_back(price);
        if self.prices.len() > self.period {
            self.prices.pop_front();
        }

        if self.prices.len() < self.period {
            return None;
        }

        let middle = self.prices.iter().sum::<f64>() / self.period as f64;
        let variance = self.calculate_variance(middle);
        let std = variance.sqrt();

        Some(BbValues {
            upper: middle + (self.std_dev * std),
            middle,
            lower: middle - (self.std_dev * std),
        })
    }

    fn calculate_variance(&self, mean: f64) -> f64 {
        self.prices
            .iter()
            .map(|price| {
                let diff = price - mean;
                diff * diff
            })
            .sum::<f64>()
            / self.period as f64
    }
}

/// ATR (Average True Range) calculator
pub struct AtrCalculator {
    prev_close: Option<f64>,
    atr_ema: EmaCalculator,
}

impl AtrCalculator {
    pub fn new(period: usize) -> Self {
        Self {
            prev_close: None,
            atr_ema: EmaCalculator::new(period),
        }
    }

    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        let true_range = if let Some(prev) = self.prev_close {
            (high - low)
                .max((high - prev).abs())
                .max((low - prev).abs())
        } else {
            high - low
        };

        self.prev_close = Some(close);
        self.atr_ema.update(true_range)
    }

    pub fn current(&self) -> Option<f64> {
        self.atr_ema.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_calculation() {
        let mut rsi = RsiCalculator::new(14);

        // Add 15 prices (need 14+1 for first calculation)
        let prices = vec![
            44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.5, 44.75, 45.0, 45.5, 45.25,
            45.75,
        ];

        let mut last_rsi = None;
        for price in prices {
            last_rsi = rsi.add_price(price);
        }

        assert!(last_rsi.is_some());
        let rsi_value = last_rsi.unwrap();
        assert!((0.0..=100.0).contains(&rsi_value));
        println!("RSI: {:.2}", rsi_value);
    }

    #[test]
    fn test_rsi_all_gains() {
        let mut rsi = RsiCalculator::new(5);

        // All increasing prices = RSI should be 100
        for i in 1..=7 {
            rsi.add_price(100.0 + i as f64);
        }

        let rsi_value = rsi.current().unwrap();
        assert!((rsi_value - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_rsi_all_losses() {
        let mut rsi = RsiCalculator::new(5);

        // All decreasing prices = RSI should be 0
        for i in (1..=7).rev() {
            rsi.add_price(100.0 + i as f64);
        }

        let rsi_value = rsi.current().unwrap();
        assert!(rsi_value < 1.0);
    }

    #[test]
    fn test_oversold_overbought() {
        assert!(is_oversold(25.0, 30.0));
        assert!(!is_oversold(35.0, 30.0));
        assert!(is_overbought(75.0, 70.0));
        assert!(!is_overbought(65.0, 70.0));
    }

    #[test]
    fn test_ema_calculation() {
        let mut ema = EmaCalculator::new(5);

        // Add 5 prices for initial SMA
        let prices = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let mut result = None;
        for price in prices {
            result = ema.add_price(price);
        }

        // First EMA = SMA = (10+11+12+13+14)/5 = 12.0
        assert!(result.is_some());
        assert!((result.unwrap() - 12.0).abs() < 0.01);

        // Add another price
        let next = ema.add_price(15.0);
        // EMA = 15 * (2/6) + 12 * (4/6) = 5 + 8 = 13.0
        assert!((next.unwrap() - 13.0).abs() < 0.01);
    }

    #[test]
    fn test_trend_detection() {
        assert_eq!(Trend::from_price_ema(100.0, Some(95.0)), Trend::Up);
        assert_eq!(Trend::from_price_ema(90.0, Some(95.0)), Trend::Down);
        assert_eq!(Trend::from_price_ema(95.0, Some(95.0)), Trend::Neutral);
        assert_eq!(Trend::from_price_ema(100.0, None), Trend::Neutral);
    }

    #[test]
    fn test_trend_allows() {
        assert!(Trend::Up.allows_buy());
        assert!(!Trend::Up.allows_sell());
        assert!(!Trend::Down.allows_buy());
        assert!(Trend::Down.allows_sell());
        assert!(Trend::Neutral.allows_buy());
        assert!(Trend::Neutral.allows_sell());
    }

    #[test]
    fn test_ema_update_alias() {
        let mut ema = EmaCalculator::new(10);
        // First update initializes sum
        let first = ema.update(100.0);
        assert!(first.is_none()); // Need 10 prices for EMA

        // Add more prices
        for i in 1..10 {
            ema.update(100.0 + i as f64);
        }
        assert!(ema.current().is_some());
    }

    #[test]
    fn test_macd_calculation() {
        let mut macd = MacdCalculator::new(12, 26, 9);
        let mut last = None;
        for price in 100..140 {
            last = macd.update(price as f64);
        }
        let values = last.unwrap();
        assert!(values.histogram.abs() < 10.0);
    }

    #[test]
    fn test_bollinger_bands() {
        let mut bb = BollingerBands::new(20, 2.0);
        for i in 0..20 {
            bb.update(100.0 + (i % 5) as f64);
        }
        let bands = bb.update(100.0).unwrap();
        assert!(bands.middle > 95.0 && bands.middle < 105.0);
        assert!(bands.upper > bands.middle);
        assert!(bands.lower < bands.middle);
    }

    #[test]
    fn test_atr_calculation() {
        let mut atr = AtrCalculator::new(14);
        let mut result = None;
        for i in 0..14 {
            let offset = i as f64;
            result = atr.update(102.0 + offset, 98.0 + offset, 100.0 + offset);
        }
        let atr_value = result.unwrap();
        assert!(atr_value > 0.0 && atr_value < 10.0);
    }
}
