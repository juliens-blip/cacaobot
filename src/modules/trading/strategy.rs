//! Trading strategy module
//!
//! Implements the trading logic combining RSI and sentiment analysis.
//! Includes risk management with position limits and daily loss circuit breaker.

use crate::config::{StrategyConfig, TradingConfig};
use crate::error::Result;
use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};

use super::circuit_breakers::{CircuitBreakers, CircuitBreakerConfig};
use super::indicators::{EmaCalculator, Trend};
use super::orders::{CloseReason, OrderSide, Position, PositionManager};

/// Trading signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Buy signal
    Buy,
    /// Sell signal
    Sell,
    /// No signal - hold
    Hold,
}

impl std::fmt::Display for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Signal::Buy => write!(f, "BUY"),
            Signal::Sell => write!(f, "SELL"),
            Signal::Hold => write!(f, "HOLD"),
        }
    }
}

/// Risk management state
#[derive(Debug, Clone)]
pub struct RiskState {
    /// Daily realized P&L
    pub daily_pnl: f64,
    /// Daily P&L start timestamp
    pub day_start: DateTime<Utc>,
    /// Circuit breaker triggered
    pub circuit_breaker: bool,
    /// Consecutive losses count
    pub consecutive_losses: u32,
    /// Today's trade count
    pub daily_trades: u32,
}

impl Default for RiskState {
    fn default() -> Self {
        Self {
            daily_pnl: 0.0,
            day_start: Utc::now().date_naive().and_hms_opt(0, 0, 0)
                .map(|dt| dt.and_utc())
                .unwrap_or_else(Utc::now),
            circuit_breaker: false,
            consecutive_losses: 0,
            daily_trades: 0,
        }
    }
}

impl RiskState {
    /// Check if a new trading day has started and reset if needed
    pub fn check_new_day(&mut self) {
        let today = Utc::now().date_naive().and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc())
            .unwrap_or_else(Utc::now);

        if today > self.day_start {
            info!("New trading day started. Resetting risk state.");
            self.daily_pnl = 0.0;
            self.day_start = today;
            self.circuit_breaker = false;
            self.daily_trades = 0;
            // Keep consecutive_losses across days as a risk indicator
        }
    }

    /// Update P&L after closing a position
    pub fn record_trade(&mut self, pnl: f64) {
        self.daily_pnl += pnl;
        self.daily_trades += 1;

        if pnl < 0.0 {
            self.consecutive_losses += 1;
        } else {
            self.consecutive_losses = 0;
        }

        debug!(
            "Trade recorded: P&L={:.2}, Daily P&L={:.2}, Consecutive losses={}",
            pnl, self.daily_pnl, self.consecutive_losses
        );
    }

    /// Check if circuit breaker should be triggered
    pub fn check_circuit_breaker(&mut self, max_daily_loss_percent: f64, account_balance: f64) -> bool {
        let loss_percent = (-self.daily_pnl / account_balance) * 100.0;

        if loss_percent >= max_daily_loss_percent {
            warn!(
                "Circuit breaker triggered! Daily loss: {:.2}% >= {:.2}%",
                loss_percent, max_daily_loss_percent
            );
            self.circuit_breaker = true;
        }

        self.circuit_breaker
    }
}

/// Trading strategy combining RSI and sentiment analysis
#[derive(Debug)]
pub struct TradingStrategy {
    /// Strategy configuration
    strategy_config: StrategyConfig,
    /// Trading configuration
    trading_config: TradingConfig,
    /// Position manager
    position_manager: PositionManager,
    /// Risk management state
    risk_state: RiskState,
    /// Account balance for risk calculations
    account_balance: f64,
    /// EMA calculator for trend filter (50-period)
    ema: EmaCalculator,
    /// Current trend based on EMA
    current_trend: Trend,
    /// Enable/disable trend filter
    use_trend_filter: bool,
    /// Circuit breakers for risk management
    circuit_breakers: CircuitBreakers,
}

impl TradingStrategy {
    /// Create a new trading strategy
    pub fn new(
        strategy_config: StrategyConfig,
        trading_config: TradingConfig,
        account_balance: f64,
    ) -> Self {
        let circuit_breaker_config = CircuitBreakerConfig {
            daily_loss_limit: -0.05,  // -5%
            max_consecutive_losses: 3,
            volatility_threshold: 2.0,
        };

        Self {
            strategy_config,
            trading_config,
            position_manager: PositionManager::new(),
            risk_state: RiskState::default(),
            account_balance,
            ema: EmaCalculator::new(50), // 50-period EMA for trend
            current_trend: Trend::Neutral,
            use_trend_filter: true,
            circuit_breakers: CircuitBreakers::new(circuit_breaker_config),
        }
    }

    /// Update price data and recalculate EMA/trend
    pub fn update_price(&mut self, price: f64) {
        if let Some(ema_val) = self.ema.add_price(price) {
            self.current_trend = Trend::from_price_ema(price, Some(ema_val));
            debug!(
                "EMA update: price={:.2}, EMA={:.2}, trend={:?}",
                price, ema_val, self.current_trend
            );
        }
    }

    /// Check if conditions indicate a BUY signal
    ///
    /// Buy when:
    /// - RSI < 30 (oversold)
    /// - Sentiment > 30 (bullish)
    /// - Trend is UP or Neutral (if trend filter enabled)
    pub fn should_buy(&self, rsi: f64, sentiment: i32) -> bool {
        let oversold = rsi < self.strategy_config.rsi_oversold;
        let bullish = sentiment > self.strategy_config.sentiment_threshold;
        let trend_ok = !self.use_trend_filter || self.current_trend.allows_buy();

        debug!(
            "Buy check: RSI={:.2} (<{:.2}? {}), Sentiment={} (>{}? {}), Trend={:?} (ok={})",
            rsi,
            self.strategy_config.rsi_oversold,
            oversold,
            sentiment,
            self.strategy_config.sentiment_threshold,
            bullish,
            self.current_trend,
            trend_ok
        );

        oversold && bullish && trend_ok
    }

    /// Check if conditions indicate a SELL signal
    ///
    /// Sell when:
    /// - RSI > 70 (overbought)
    /// - Sentiment < -30 (bearish)
    /// - Trend is DOWN or Neutral (if trend filter enabled)
    pub fn should_sell(&self, rsi: f64, sentiment: i32) -> bool {
        let overbought = rsi > self.strategy_config.rsi_overbought;
        let bearish = sentiment < -self.strategy_config.sentiment_threshold;
        let trend_ok = !self.use_trend_filter || self.current_trend.allows_sell();

        debug!(
            "Sell check: RSI={:.2} (>{:.2}? {}), Sentiment={} (<-{}? {}), Trend={:?} (ok={})",
            rsi,
            self.strategy_config.rsi_overbought,
            overbought,
            sentiment,
            self.strategy_config.sentiment_threshold,
            bearish,
            self.current_trend,
            trend_ok
        );

        overbought && bearish && trend_ok
    }

    /// Generate trading signal based on RSI and sentiment
    pub fn generate_signal(&self, rsi: f64, sentiment: i32) -> Signal {
        if self.should_buy(rsi, sentiment) {
            Signal::Buy
        } else if self.should_sell(rsi, sentiment) {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }

    /// Check if take profit is hit for a position
    ///
    /// Take profit at +2% (configurable)
    pub fn check_take_profit(&self, position: &Position, current_price: f64) -> bool {
        let pnl_percent = position.calculate_pnl_percent(current_price);
        let tp_threshold = self.trading_config.take_profit_percent;

        debug!(
            "TP check for {}: P&L={:.2}% (>= {:.2}%? {})",
            position.id,
            pnl_percent,
            tp_threshold,
            pnl_percent >= tp_threshold
        );

        pnl_percent >= tp_threshold
    }

    /// Check if stop loss is hit for a position
    ///
    /// Stop loss at -1.5% (configurable)
    pub fn check_stop_loss(&self, position: &Position, current_price: f64) -> bool {
        let pnl_percent = position.calculate_pnl_percent(current_price);
        let sl_threshold = -self.trading_config.stop_loss_percent;

        debug!(
            "SL check for {}: P&L={:.2}% (<= {:.2}%? {})",
            position.id,
            pnl_percent,
            sl_threshold,
            pnl_percent <= sl_threshold
        );

        pnl_percent <= sl_threshold
    }

    /// Check position for exit conditions
    pub fn check_position_exit(&self, position: &Position, current_price: f64) -> Option<CloseReason> {
        if self.check_take_profit(position, current_price) {
            Some(CloseReason::TakeProfit)
        } else if self.check_stop_loss(position, current_price) {
            Some(CloseReason::StopLoss)
        } else {
            None
        }
    }

    /// Check if we can open a new position (risk management)
    pub fn can_open_position(&mut self) -> Result<bool> {
        // Check for new trading day and reset circuit breakers if needed
        self.risk_state.check_new_day();
        
        // Sync circuit breakers daily reset
        if self.risk_state.day_start > chrono::Utc::now() - chrono::Duration::hours(24) {
            self.circuit_breakers.reset_daily();
        }

        // Check circuit breakers first (highest priority)
        if !self.circuit_breakers.is_trading_allowed() {
            warn!("Circuit breakers triggered - no new positions allowed");
            return Ok(false);
        }

        // Check old circuit breaker state (for backward compatibility)
        if self.risk_state.circuit_breaker {
            warn!("Circuit breaker active - no new positions allowed");
            return Ok(false);
        }

        // Check daily loss limit
        let daily_loss_pct = self.risk_state.daily_pnl / self.account_balance;
        self.circuit_breakers.check_daily_loss(daily_loss_pct);
        
        if self.risk_state.check_circuit_breaker(
            self.trading_config.max_daily_loss_percent,
            self.account_balance,
        ) {
            return Ok(false);
        }

        // Check max positions limit
        let current_positions = self.position_manager.count();
        if current_positions >= self.trading_config.max_positions {
            debug!(
                "Max positions reached: {}/{}",
                current_positions, self.trading_config.max_positions
            );
            return Ok(false);
        }

        // Check consecutive losses (cool down after 3 consecutive losses)
        if self.risk_state.consecutive_losses >= 3 {
            warn!(
                "Consecutive losses cool down: {} losses in a row",
                self.risk_state.consecutive_losses
            );
            return Ok(false);
        }

        Ok(true)
    }

    /// Calculate take profit price for a given entry price and side
    pub fn calculate_take_profit(&self, entry_price: f64, side: OrderSide) -> f64 {
        let tp_percent = self.trading_config.take_profit_percent / 100.0;
        match side {
            OrderSide::Buy => entry_price * (1.0 + tp_percent),
            OrderSide::Sell => entry_price * (1.0 - tp_percent),
        }
    }

    /// Calculate stop loss price for a given entry price and side
    pub fn calculate_stop_loss(&self, entry_price: f64, side: OrderSide) -> f64 {
        let sl_percent = self.trading_config.stop_loss_percent / 100.0;
        match side {
            OrderSide::Buy => entry_price * (1.0 - sl_percent),
            OrderSide::Sell => entry_price * (1.0 + sl_percent),
        }
    }

    /// Calculate position size based on risk (returns base currency units)
    ///
    /// Formula: volume = risk_amount / risk_per_unit
    /// The result is in base currency units. normalize_volume() in bot.rs
    /// handles conversion to cTrader volume units and broker min/max/step.
    pub fn calculate_position_size(&self, entry_price: f64, stop_loss: f64) -> f64 {
        let risk_amount = self.account_balance * (self.trading_config.risk_per_trade / 100.0);
        let risk_per_unit = (entry_price - stop_loss).abs();

        if risk_per_unit > 0.0 {
            risk_amount / risk_per_unit
        } else {
            0.0
        }
    }

    /// Add a position to the manager
    pub fn add_position(&mut self, position: Position) {
        self.position_manager.add(position);
    }

    /// Close a position and record the trade
    pub fn close_position(
        &mut self,
        position_id: &str,
        close_price: f64,
        reason: CloseReason,
    ) -> Option<f64> {
        if let Some(closed) = self.position_manager.close(position_id, close_price, reason) {
            // Record trade in risk state
            self.risk_state.record_trade(closed.realized_pnl);
            
            // Also record in circuit breakers
            let won = closed.realized_pnl > 0.0;
            self.circuit_breakers.record_trade_result(won);
            
            info!(
                "Position {} closed: P&L={:.2}, Reason={}, Won={}",
                position_id, closed.realized_pnl, reason, won
            );
            Some(closed.realized_pnl)
        } else {
            None
        }
    }

    /// Update account balance
    pub fn update_balance(&mut self, balance: f64) {
        self.account_balance = balance;
    }

    /// Get position manager reference
    pub fn position_manager(&self) -> &PositionManager {
        &self.position_manager
    }

    /// Get all open positions (compatibility alias)
    pub fn get_open_positions(&self) -> &[Position] {
        self.position_manager.open_positions()
    }

    /// Get mutable position manager reference
    pub fn position_manager_mut(&mut self) -> &mut PositionManager {
        &mut self.position_manager
    }

    /// Replace open positions after broker reconciliation
    pub fn reconcile_positions(&mut self, positions: Vec<Position>) {
        self.position_manager.replace_positions(positions);
    }

    /// Get risk state
    pub fn risk_state(&self) -> &RiskState {
        &self.risk_state
    }

    /// Get trading configuration
    pub fn trading_config(&self) -> &TradingConfig {
        &self.trading_config
    }

    /// Get strategy configuration
    pub fn strategy_config(&self) -> &StrategyConfig {
        &self.strategy_config
    }

    /// Reset consecutive losses counter (e.g., after manual intervention)
    pub fn reset_consecutive_losses(&mut self) {
        self.risk_state.consecutive_losses = 0;
    }

    /// Manually disable circuit breaker (use with caution)
    pub fn disable_circuit_breaker(&mut self) {
        warn!("Circuit breaker manually disabled");
        self.risk_state.circuit_breaker = false;
    }

    /// Enable or disable trend filter
    pub fn set_trend_filter(&mut self, enabled: bool) {
        self.use_trend_filter = enabled;
        info!("Trend filter {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Get current trend
    pub fn current_trend(&self) -> Trend {
        self.current_trend
    }

    /// Get current EMA value
    pub fn current_ema(&self) -> Option<f64> {
        self.ema.current()
    }

    /// Check if trend filter is enabled
    pub fn is_trend_filter_enabled(&self) -> bool {
        self.use_trend_filter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_strategy() -> TradingStrategy {
        let strategy_config = StrategyConfig {
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            rsi_timeframe: "5m".to_string(),
            sentiment_threshold: 30,
        };

        let trading_config = TradingConfig {
            symbol: "FCPO".to_string(),
            risk_per_trade: 1.0,
            take_profit_percent: 2.0,
            stop_loss_percent: 1.5,
            max_positions: 1,
            max_daily_loss_percent: 5.0,
            initial_balance: 10000.0,
        };

        TradingStrategy::new(strategy_config, trading_config, 10000.0)
    }

    #[test]
    fn test_should_buy() {
        let strategy = create_test_strategy();

        // RSI < 30 && sentiment > 30 = BUY
        assert!(strategy.should_buy(25.0, 50));
        assert!(strategy.should_buy(29.9, 31));

        // RSI >= 30 = NO BUY
        assert!(!strategy.should_buy(30.0, 50));
        assert!(!strategy.should_buy(50.0, 80));

        // Sentiment <= 30 = NO BUY
        assert!(!strategy.should_buy(25.0, 30));
        assert!(!strategy.should_buy(25.0, -10));
    }

    #[test]
    fn test_should_sell() {
        let strategy = create_test_strategy();

        // RSI > 70 && sentiment < -30 = SELL
        assert!(strategy.should_sell(75.0, -50));
        assert!(strategy.should_sell(70.1, -31));

        // RSI <= 70 = NO SELL
        assert!(!strategy.should_sell(70.0, -50));
        assert!(!strategy.should_sell(50.0, -80));

        // Sentiment >= -30 = NO SELL
        assert!(!strategy.should_sell(75.0, -30));
        assert!(!strategy.should_sell(75.0, 10));
    }

    #[test]
    fn test_generate_signal() {
        let strategy = create_test_strategy();

        assert_eq!(strategy.generate_signal(25.0, 50), Signal::Buy);
        assert_eq!(strategy.generate_signal(75.0, -50), Signal::Sell);
        assert_eq!(strategy.generate_signal(50.0, 0), Signal::Hold);
        assert_eq!(strategy.generate_signal(25.0, 0), Signal::Hold);  // Oversold but neutral sentiment
        assert_eq!(strategy.generate_signal(75.0, 50), Signal::Hold); // Overbought but bullish
    }

    #[test]
    fn test_get_open_positions() {
        let mut strategy = create_test_strategy();
        assert_eq!(strategy.get_open_positions().len(), 0);

        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);
        strategy.add_position(position);
        assert_eq!(strategy.get_open_positions().len(), 1);
    }

    #[test]
    fn test_check_take_profit() {
        let strategy = create_test_strategy();

        // Buy position at 4850, TP at +2% = 4947
        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);

        assert!(!strategy.check_take_profit(&position, 4900.0));  // +1.03%
        assert!(!strategy.check_take_profit(&position, 4946.0));  // +1.98%
        assert!(strategy.check_take_profit(&position, 4947.0));   // +2.0%
        assert!(strategy.check_take_profit(&position, 5000.0));   // +3.09%
    }

    #[test]
    fn test_check_stop_loss() {
        let strategy = create_test_strategy();

        // Buy position at 4850, SL at -1.5% = 4777.25
        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);

        assert!(!strategy.check_stop_loss(&position, 4800.0));  // -1.03%
        assert!(!strategy.check_stop_loss(&position, 4778.0));  // -1.48%
        assert!(strategy.check_stop_loss(&position, 4777.0));   // -1.50%
        assert!(strategy.check_stop_loss(&position, 4700.0));   // -3.09%
    }

    #[test]
    fn test_check_take_profit_sell() {
        let strategy = create_test_strategy();

        // Sell position at 4850, TP at +2% (price going down)
        let position = Position::new("pos_1", "FCPO", OrderSide::Sell, 4850.0, 1.0);

        assert!(!strategy.check_take_profit(&position, 4800.0));  // +1.03%
        assert!(strategy.check_take_profit(&position, 4753.0));   // +2.0%
        assert!(strategy.check_take_profit(&position, 4700.0));   // +3.09%
    }

    #[test]
    fn test_check_stop_loss_sell() {
        let strategy = create_test_strategy();

        // Sell position at 4850, SL at -1.5% (price going up)
        let position = Position::new("pos_1", "FCPO", OrderSide::Sell, 4850.0, 1.0);

        assert!(!strategy.check_stop_loss(&position, 4900.0));   // -1.03%
        assert!(strategy.check_stop_loss(&position, 4923.0));    // -1.50%
        assert!(strategy.check_stop_loss(&position, 5000.0));    // -3.09%
    }

    #[test]
    fn test_calculate_tp_sl_buy() {
        let strategy = create_test_strategy();
        let entry = 4850.0;

        let tp = strategy.calculate_take_profit(entry, OrderSide::Buy);
        let sl = strategy.calculate_stop_loss(entry, OrderSide::Buy);

        assert!((tp - 4947.0).abs() < 0.01);   // +2%
        assert!((sl - 4777.25).abs() < 0.01);  // -1.5%
    }

    #[test]
    fn test_calculate_tp_sl_sell() {
        let strategy = create_test_strategy();
        let entry = 4850.0;

        let tp = strategy.calculate_take_profit(entry, OrderSide::Sell);
        let sl = strategy.calculate_stop_loss(entry, OrderSide::Sell);

        assert!((tp - 4753.0).abs() < 0.01);   // -2% (profit for short)
        assert!((sl - 4922.75).abs() < 0.01);  // +1.5% (loss for short)
    }

    #[test]
    fn test_can_open_position() {
        let mut strategy = create_test_strategy();

        // Initially should be able to open position
        assert!(strategy.can_open_position().unwrap());

        // Add max positions
        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);
        strategy.add_position(position);

        // Now should not be able to open another
        assert!(!strategy.can_open_position().unwrap());
    }

    #[test]
    fn test_consecutive_losses_cooldown() {
        let mut strategy = create_test_strategy();

        // Simulate 3 consecutive losses
        strategy.risk_state.consecutive_losses = 3;

        // Should not be able to open position due to cooldown
        assert!(!strategy.can_open_position().unwrap());

        // Reset consecutive losses
        strategy.reset_consecutive_losses();
        assert!(strategy.can_open_position().unwrap());
    }

    #[test]
    fn test_circuit_breaker() {
        let mut strategy = create_test_strategy();

        // Simulate -5% daily loss (500 on 10000 balance)
        strategy.risk_state.daily_pnl = -500.0;

        // This should trigger circuit breaker
        let triggered = strategy.risk_state.check_circuit_breaker(5.0, 10000.0);
        assert!(triggered);

        // Should not be able to open position
        assert!(!strategy.can_open_position().unwrap());
    }

    #[test]
    fn test_close_position_records_trade() {
        let mut strategy = create_test_strategy();

        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);
        strategy.add_position(position);

        // Close with profit
        let pnl = strategy.close_position("pos_1", 4900.0, CloseReason::TakeProfit);

        assert!(pnl.is_some());
        assert!((pnl.unwrap() - 50.0).abs() < 0.01);
        assert!((strategy.risk_state.daily_pnl - 50.0).abs() < 0.01);
        assert_eq!(strategy.risk_state.daily_trades, 1);
        assert_eq!(strategy.risk_state.consecutive_losses, 0);
    }

    #[test]
    fn test_close_position_loss_increments_consecutive() {
        let mut strategy = create_test_strategy();

        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);
        strategy.add_position(position);

        // Close with loss
        let pnl = strategy.close_position("pos_1", 4800.0, CloseReason::StopLoss);

        assert!(pnl.is_some());
        assert!(pnl.unwrap() < 0.0);
        assert_eq!(strategy.risk_state.consecutive_losses, 1);
    }

    #[test]
    fn test_calculate_position_size() {
        let strategy = create_test_strategy();

        // With 1% risk on 10000 balance = 100 risk
        // Entry 4850, SL 4777.25 = 72.75 risk per unit
        // Size = 100 / 72.75 ≈ 1.374 base currency units
        let entry = 4850.0;
        let sl = strategy.calculate_stop_loss(entry, OrderSide::Buy);
        let size = strategy.calculate_position_size(entry, sl);

        let expected = 100.0 / (entry - sl);
        assert!((size - expected).abs() < 0.01, "size={} expected={}", size, expected);
        assert!(size > 0.0);
    }

    #[test]
    fn test_check_position_exit() {
        let strategy = create_test_strategy();

        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);

        // No exit at neutral price
        assert!(strategy.check_position_exit(&position, 4850.0).is_none());
        assert!(strategy.check_position_exit(&position, 4900.0).is_none());

        // Take profit hit
        let exit = strategy.check_position_exit(&position, 4947.0);
        assert_eq!(exit, Some(CloseReason::TakeProfit));

        // Stop loss hit
        let exit = strategy.check_position_exit(&position, 4777.0);
        assert_eq!(exit, Some(CloseReason::StopLoss));
    }

    #[test]
    fn test_signal_display() {
        assert_eq!(format!("{}", Signal::Buy), "BUY");
        assert_eq!(format!("{}", Signal::Sell), "SELL");
        assert_eq!(format!("{}", Signal::Hold), "HOLD");
    }

    // TASK-PO-012: Additional strategy tests
    #[test]
    fn test_should_buy_oversold_bullish() {
        let mut strategy = create_test_strategy();
        strategy.set_trend_filter(false);
        let rsi = 25.0;
        let sentiment = 40;
        assert!(strategy.should_buy(rsi, sentiment));
    }

    #[test]
    fn test_should_buy_neutral() {
        let mut strategy = create_test_strategy();
        strategy.set_trend_filter(false);
        let rsi = 50.0;
        let sentiment = 0;
        assert!(!strategy.should_buy(rsi, sentiment));
    }

    #[test]
    fn test_should_sell_overbought_bearish() {
        let mut strategy = create_test_strategy();
        strategy.set_trend_filter(false);
        let rsi = 75.0;
        let sentiment = -40;
        assert!(strategy.should_sell(rsi, sentiment));
    }

    #[test]
    fn test_should_sell_neutral() {
        let mut strategy = create_test_strategy();
        strategy.set_trend_filter(false);
        let rsi = 50.0;
        let sentiment = 0;
        assert!(!strategy.should_sell(rsi, sentiment));
    }

    #[test]
    fn test_edge_cases_thresholds() {
        let mut strategy = create_test_strategy();
        strategy.set_trend_filter(false);

        // RSI exactly at 30 (threshold) should NOT trigger buy (need < 30)
        assert!(!strategy.should_buy(30.0, 50));
        // RSI just below 30 should trigger buy
        assert!(strategy.should_buy(29.9, 50));

        // RSI exactly at 70 (threshold) should NOT trigger sell (need > 70)
        assert!(!strategy.should_sell(70.0, -50));
        // RSI just above 70 should trigger sell
        assert!(strategy.should_sell(70.1, -50));

        // Sentiment exactly at 30 should NOT trigger (need > 30)
        assert!(!strategy.should_buy(25.0, 30));
        // Sentiment just above 30 should trigger
        assert!(strategy.should_buy(25.0, 31));

        // Sentiment exactly at -30 should NOT trigger (need < -30)
        assert!(!strategy.should_sell(75.0, -30));
        // Sentiment just below -30 should trigger
        assert!(strategy.should_sell(75.0, -31));
    }

    #[test]
    fn test_risk_state_record_trade() {
        let mut risk_state = RiskState::default();

        // Record winning trade
        risk_state.record_trade(50.0);
        assert!((risk_state.daily_pnl - 50.0).abs() < 0.01);
        assert_eq!(risk_state.consecutive_losses, 0);
        assert_eq!(risk_state.daily_trades, 1);

        // Record losing trade
        risk_state.record_trade(-30.0);
        assert!((risk_state.daily_pnl - 20.0).abs() < 0.01);
        assert_eq!(risk_state.consecutive_losses, 1);
        assert_eq!(risk_state.daily_trades, 2);

        // Another losing trade
        risk_state.record_trade(-25.0);
        assert_eq!(risk_state.consecutive_losses, 2);

        // Winning trade resets consecutive losses
        risk_state.record_trade(40.0);
        assert_eq!(risk_state.consecutive_losses, 0);
    }

    #[test]
    fn test_trend_filter() {
        let mut strategy = create_test_strategy();

        // Initially trend is Neutral, so should_buy works with RSI/sentiment
        assert!(strategy.should_buy(25.0, 50)); // Oversold + Bullish

        // Simulate downtrend by feeding decreasing prices
        for i in 0..60 {
            strategy.update_price(5000.0 - (i as f64 * 5.0));
        }

        // Now trend should be Down, so buying should be blocked
        assert_eq!(strategy.current_trend(), Trend::Down);
        assert!(!strategy.should_buy(25.0, 50)); // Blocked by trend

        // But selling should work
        assert!(strategy.should_sell(75.0, -50)); // Overbought + Bearish + Downtrend

        // Disable trend filter
        strategy.set_trend_filter(false);
        assert!(strategy.should_buy(25.0, 50)); // Now works again
    }

    #[test]
    fn test_ema_integration() {
        let mut strategy = create_test_strategy();

        // EMA not ready initially
        assert!(strategy.current_ema().is_none());

        // Feed 50 prices to initialize EMA
        for i in 0..50 {
            strategy.update_price(4800.0 + (i as f64));
        }

        // Now EMA should be ready
        assert!(strategy.current_ema().is_some());
    }
}
