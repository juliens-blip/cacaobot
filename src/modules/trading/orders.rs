//! Order and Position management module
//!
//! Provides structures for managing trading orders and positions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Order side (direction)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

impl OrderSide {
    /// Returns the opposite side
    pub fn opposite(&self) -> Self {
        match self {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        }
    }
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is pending execution
    Pending,
    /// Order has been filled
    Filled,
    /// Order was cancelled
    Cancelled,
    /// Order was rejected by the broker
    Rejected,
    /// Order partially filled
    PartiallyFilled,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderStatus::Pending => write!(f, "PENDING"),
            OrderStatus::Filled => write!(f, "FILLED"),
            OrderStatus::Cancelled => write!(f, "CANCELLED"),
            OrderStatus::Rejected => write!(f, "REJECTED"),
            OrderStatus::PartiallyFilled => write!(f, "PARTIALLY_FILLED"),
        }
    }
}

/// Trading order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Unique order identifier
    pub id: String,
    /// Trading symbol (e.g., "FCPO")
    pub symbol: String,
    /// Order direction
    pub side: OrderSide,
    /// Volume in lots
    pub volume: f64,
    /// Order price (for limit orders)
    pub price: Option<f64>,
    /// Take profit price
    pub take_profit: Option<f64>,
    /// Stop loss price
    pub stop_loss: Option<f64>,
    /// Order status
    pub status: OrderStatus,
    /// Order creation timestamp
    pub created_at: DateTime<Utc>,
    /// Order fill timestamp
    pub filled_at: Option<DateTime<Utc>>,
    /// Fill price (may differ from order price)
    pub fill_price: Option<f64>,
    /// Rejection reason (if rejected)
    pub rejection_reason: Option<String>,
}

impl Order {
    /// Create a new market order
    pub fn market(
        id: impl Into<String>,
        symbol: impl Into<String>,
        side: OrderSide,
        volume: f64,
    ) -> Self {
        Self {
            id: id.into(),
            symbol: symbol.into(),
            side,
            volume,
            price: None,
            take_profit: None,
            stop_loss: None,
            status: OrderStatus::Pending,
            created_at: Utc::now(),
            filled_at: None,
            fill_price: None,
            rejection_reason: None,
        }
    }

    /// Create a new limit order
    pub fn limit(
        id: impl Into<String>,
        symbol: impl Into<String>,
        side: OrderSide,
        volume: f64,
        price: f64,
    ) -> Self {
        Self {
            id: id.into(),
            symbol: symbol.into(),
            side,
            volume,
            price: Some(price),
            take_profit: None,
            stop_loss: None,
            status: OrderStatus::Pending,
            created_at: Utc::now(),
            filled_at: None,
            fill_price: None,
            rejection_reason: None,
        }
    }

    /// Set take profit price
    pub fn with_take_profit(mut self, tp: f64) -> Self {
        self.take_profit = Some(tp);
        self
    }

    /// Set stop loss price
    pub fn with_stop_loss(mut self, sl: f64) -> Self {
        self.stop_loss = Some(sl);
        self
    }

    /// Mark order as filled
    pub fn fill(&mut self, fill_price: f64) {
        self.status = OrderStatus::Filled;
        self.filled_at = Some(Utc::now());
        self.fill_price = Some(fill_price);
    }

    /// Mark order as cancelled
    pub fn cancel(&mut self) {
        self.status = OrderStatus::Cancelled;
    }

    /// Mark order as rejected
    pub fn reject(&mut self, reason: impl Into<String>) {
        self.status = OrderStatus::Rejected;
        self.rejection_reason = Some(reason.into());
    }

    /// Check if order is active (can be cancelled)
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Pending | OrderStatus::PartiallyFilled)
    }

    /// Check if order is terminal (final state)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected
        )
    }
}

/// Trailing stop configuration
#[derive(Debug, Clone, Copy)]
pub struct TrailingStopConfig {
    /// Activation threshold (% profit before trailing starts)
    pub activation_percent: f64,
    /// Trail distance (% behind highest profit)
    pub trail_percent: f64,
}

impl Default for TrailingStopConfig {
    fn default() -> Self {
        Self {
            activation_percent: 1.0, // Activate at +1%
            trail_percent: 0.5,      // Trail 0.5% behind peak
        }
    }
}

/// Open trading position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Unique position identifier
    pub id: String,
    /// Trading symbol
    pub symbol: String,
    /// Position direction
    pub side: OrderSide,
    /// Entry price
    pub entry_price: f64,
    /// Position volume in lots
    pub volume: f64,
    /// Current unrealized P&L
    pub current_pnl: f64,
    /// Current price (for P&L calculation)
    pub current_price: f64,
    /// Take profit price
    pub take_profit: Option<f64>,
    /// Stop loss price
    pub stop_loss: Option<f64>,
    /// Position open timestamp
    pub opened_at: DateTime<Utc>,
    /// Associated order ID
    pub order_id: String,
    /// Trailing stop configuration
    #[serde(skip)]
    trailing_config: Option<TrailingStopConfig>,
    /// Highest price reached (for BUY positions)
    highest_price: f64,
    /// Lowest price reached (for SELL positions)
    lowest_price: f64,
    /// Whether trailing stop is active
    trailing_active: bool,
    /// Current trailing stop price
    trailing_stop_price: Option<f64>,
}

impl Position {
    /// Create a new position from a filled order
    pub fn from_order(order: &Order, fill_price: f64) -> Self {
        Self {
            id: format!("pos_{}", order.id),
            symbol: order.symbol.clone(),
            side: order.side,
            entry_price: fill_price,
            volume: order.volume,
            current_pnl: 0.0,
            current_price: fill_price,
            take_profit: order.take_profit,
            stop_loss: order.stop_loss,
            opened_at: Utc::now(),
            order_id: order.id.clone(),
            trailing_config: None,
            highest_price: fill_price,
            lowest_price: fill_price,
            trailing_active: false,
            trailing_stop_price: None,
        }
    }

    /// Create a new position directly
    pub fn new(
        id: impl Into<String>,
        symbol: impl Into<String>,
        side: OrderSide,
        entry_price: f64,
        volume: f64,
    ) -> Self {
        Self {
            id: id.into(),
            symbol: symbol.into(),
            side,
            entry_price,
            volume,
            current_pnl: 0.0,
            current_price: entry_price,
            take_profit: None,
            stop_loss: None,
            opened_at: Utc::now(),
            order_id: String::new(),
            trailing_config: None,
            highest_price: entry_price,
            lowest_price: entry_price,
            trailing_active: false,
            trailing_stop_price: None,
        }
    }

    /// Set take profit price
    pub fn with_take_profit(mut self, tp: f64) -> Self {
        self.take_profit = Some(tp);
        self
    }

    /// Set stop loss price
    pub fn with_stop_loss(mut self, sl: f64) -> Self {
        self.stop_loss = Some(sl);
        self
    }

    /// Enable trailing stop for this position
    pub fn with_trailing_stop(mut self, config: TrailingStopConfig) -> Self {
        self.trailing_config = Some(config);
        self
    }

    /// Update trailing stop with current price
    /// Returns Some(stop_price) if trailing stop is triggered
    pub fn update_trailing_stop(&mut self, current_price: f64) -> Option<f64> {
        let config = self.trailing_config?;

        let pnl_percent = self.calculate_pnl_percent(current_price);

        match self.side {
            OrderSide::Buy => {
                // Track highest price
                if current_price > self.highest_price {
                    self.highest_price = current_price;
                }

                // Check activation
                if !self.trailing_active && pnl_percent >= config.activation_percent {
                    self.trailing_active = true;
                }

                // Update trailing stop price
                if self.trailing_active {
                    let new_stop = self.highest_price * (1.0 - config.trail_percent / 100.0);

                    // Only move stop up, never down
                    if self.trailing_stop_price.is_none_or(|stop| new_stop > stop) {
                        self.trailing_stop_price = Some(new_stop);
                    }

                    // Check if stop is hit
                    if let Some(stop_price) = self.trailing_stop_price {
                        if current_price <= stop_price {
                            return self.trailing_stop_price;
                        }
                    }
                }
            }
            OrderSide::Sell => {
                // Track lowest price
                if current_price < self.lowest_price {
                    self.lowest_price = current_price;
                }

                // Check activation
                if !self.trailing_active && pnl_percent >= config.activation_percent {
                    self.trailing_active = true;
                }

                // Update trailing stop price
                if self.trailing_active {
                    let new_stop = self.lowest_price * (1.0 + config.trail_percent / 100.0);

                    // Only move stop down, never up
                    if self.trailing_stop_price.is_none_or(|stop| new_stop < stop) {
                        self.trailing_stop_price = Some(new_stop);
                    }

                    // Check if stop is hit
                    if let Some(stop_price) = self.trailing_stop_price {
                        if current_price >= stop_price {
                            return self.trailing_stop_price;
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if trailing stop is active
    pub fn is_trailing_active(&self) -> bool {
        self.trailing_active
    }

    /// Get current trailing stop price
    pub fn trailing_stop_price(&self) -> Option<f64> {
        self.trailing_stop_price
    }

    /// Update position with current market price
    pub fn update_price(&mut self, current_price: f64) {
        self.current_price = current_price;
        self.current_pnl = self.calculate_pnl(current_price);
    }

    /// Calculate P&L for a given price
    pub fn calculate_pnl(&self, price: f64) -> f64 {
        let price_diff = match self.side {
            OrderSide::Buy => price - self.entry_price,
            OrderSide::Sell => self.entry_price - price,
        };
        price_diff * self.volume
    }

    /// Calculate P&L as percentage
    pub fn calculate_pnl_percent(&self, price: f64) -> f64 {
        let price_diff = match self.side {
            OrderSide::Buy => price - self.entry_price,
            OrderSide::Sell => self.entry_price - price,
        };
        (price_diff / self.entry_price) * 100.0
    }

    /// Check if take profit is hit
    pub fn is_take_profit_hit(&self, price: f64) -> bool {
        if let Some(tp) = self.take_profit {
            match self.side {
                OrderSide::Buy => price >= tp,
                OrderSide::Sell => price <= tp,
            }
        } else {
            false
        }
    }

    /// Check if stop loss is hit
    pub fn is_stop_loss_hit(&self, price: f64) -> bool {
        if let Some(sl) = self.stop_loss {
            match self.side {
                OrderSide::Buy => price <= sl,
                OrderSide::Sell => price >= sl,
            }
        } else {
            false
        }
    }

    /// Get position duration
    pub fn duration(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.opened_at)
    }
}

/// Position manager for tracking multiple positions
#[derive(Debug, Default)]
pub struct PositionManager {
    positions: Vec<Position>,
    closed_positions: Vec<ClosedPosition>,
}

/// Closed position record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedPosition {
    pub position: Position,
    pub close_price: f64,
    pub realized_pnl: f64,
    pub closed_at: DateTime<Utc>,
    pub close_reason: CloseReason,
}

/// Reason for closing a position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloseReason {
    TakeProfit,
    StopLoss,
    TrailingStop,
    Manual,
    Signal,
    RiskLimit,
}

impl fmt::Display for CloseReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloseReason::TakeProfit => write!(f, "Take Profit"),
            CloseReason::StopLoss => write!(f, "Stop Loss"),
            CloseReason::TrailingStop => write!(f, "Trailing Stop"),
            CloseReason::Manual => write!(f, "Manual Close"),
            CloseReason::Signal => write!(f, "Exit Signal"),
            CloseReason::RiskLimit => write!(f, "Risk Limit"),
        }
    }
}

impl PositionManager {
    /// Create a new position manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new position
    pub fn add(&mut self, position: Position) {
        self.positions.push(position);
    }

    /// Get all open positions
    pub fn open_positions(&self) -> &[Position] {
        &self.positions
    }

    /// Get positions for a specific symbol
    pub fn positions_for_symbol(&self, symbol: &str) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| p.symbol == symbol)
            .collect()
    }

    /// Get total number of open positions
    pub fn count(&self) -> usize {
        self.positions.len()
    }

    /// Check if any positions are open for a symbol
    pub fn has_position(&self, symbol: &str) -> bool {
        self.positions.iter().any(|p| p.symbol == symbol)
    }

    /// Update all positions with current prices
    pub fn update_prices(&mut self, symbol: &str, price: f64) {
        for position in &mut self.positions {
            if position.symbol == symbol {
                position.update_price(price);
            }
        }
    }

    /// Close a position
    pub fn close(&mut self, position_id: &str, close_price: f64, reason: CloseReason) -> Option<ClosedPosition> {
        if let Some(idx) = self.positions.iter().position(|p| p.id == position_id) {
            let mut position = self.positions.remove(idx);
            position.update_price(close_price);

            let closed = ClosedPosition {
                realized_pnl: position.current_pnl,
                close_price,
                closed_at: Utc::now(),
                close_reason: reason,
                position,
            };

            self.closed_positions.push(closed.clone());
            Some(closed)
        } else {
            None
        }
    }

    /// Get total unrealized P&L
    pub fn total_unrealized_pnl(&self) -> f64 {
        self.positions.iter().map(|p| p.current_pnl).sum()
    }

    /// Get total realized P&L (from closed positions)
    pub fn total_realized_pnl(&self) -> f64 {
        self.closed_positions.iter().map(|p| p.realized_pnl).sum()
    }

    /// Get closed positions
    pub fn closed_positions(&self) -> &[ClosedPosition] {
        &self.closed_positions
    }

    /// Replace all open positions (used for reconciliation)
    pub fn replace_positions(&mut self, positions: Vec<Position>) {
        self.positions = positions;
    }

    /// Check positions for TP/SL hits
    pub fn check_exits(&self, symbol: &str, price: f64) -> Vec<(&Position, CloseReason)> {
        self.positions
            .iter()
            .filter(|p| p.symbol == symbol)
            .filter_map(|p| {
                if p.is_take_profit_hit(price) {
                    Some((p, CloseReason::TakeProfit))
                } else if p.is_stop_loss_hit(price) {
                    Some((p, CloseReason::StopLoss))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_creation() {
        let order = Order::market("123", "FCPO", OrderSide::Buy, 0.1)
            .with_take_profit(5000.0)
            .with_stop_loss(4800.0);

        assert_eq!(order.id, "123");
        assert_eq!(order.symbol, "FCPO");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.volume, 0.1);
        assert_eq!(order.take_profit, Some(5000.0));
        assert_eq!(order.stop_loss, Some(4800.0));
        assert_eq!(order.status, OrderStatus::Pending);
        assert!(order.is_active());
    }

    #[test]
    fn test_order_fill() {
        let mut order = Order::market("123", "FCPO", OrderSide::Buy, 0.1);
        assert!(order.is_active());

        order.fill(4850.0);

        assert_eq!(order.status, OrderStatus::Filled);
        assert_eq!(order.fill_price, Some(4850.0));
        assert!(order.filled_at.is_some());
        assert!(order.is_terminal());
    }

    #[test]
    fn test_order_reject() {
        let mut order = Order::market("123", "FCPO", OrderSide::Sell, 0.1);

        order.reject("Insufficient margin");

        assert_eq!(order.status, OrderStatus::Rejected);
        assert_eq!(order.rejection_reason, Some("Insufficient margin".to_string()));
    }

    #[test]
    fn test_position_pnl_buy() {
        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);

        // Price goes up - profit
        assert!((position.calculate_pnl(4900.0) - 50.0).abs() < 0.001);
        assert!((position.calculate_pnl_percent(4900.0) - 1.0309).abs() < 0.01);

        // Price goes down - loss
        assert!((position.calculate_pnl(4800.0) - (-50.0)).abs() < 0.001);
    }

    #[test]
    fn test_position_pnl_sell() {
        let position = Position::new("pos_1", "FCPO", OrderSide::Sell, 4850.0, 1.0);

        // Price goes down - profit for short
        assert!((position.calculate_pnl(4800.0) - 50.0).abs() < 0.001);

        // Price goes up - loss for short
        assert!((position.calculate_pnl(4900.0) - (-50.0)).abs() < 0.001);
    }

    #[test]
    fn test_position_take_profit() {
        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0)
            .with_take_profit(4950.0);

        assert!(!position.is_take_profit_hit(4900.0));
        assert!(position.is_take_profit_hit(4950.0));
        assert!(position.is_take_profit_hit(5000.0));
    }

    #[test]
    fn test_position_stop_loss() {
        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0)
            .with_stop_loss(4800.0);

        assert!(!position.is_stop_loss_hit(4820.0));
        assert!(position.is_stop_loss_hit(4800.0));
        assert!(position.is_stop_loss_hit(4750.0));
    }

    #[test]
    fn test_sell_position_tp_sl() {
        let position = Position::new("pos_1", "FCPO", OrderSide::Sell, 4850.0, 1.0)
            .with_take_profit(4750.0)  // Lower = profit for short
            .with_stop_loss(4950.0);   // Higher = loss for short

        // TP hit when price goes below target
        assert!(!position.is_take_profit_hit(4800.0));
        assert!(position.is_take_profit_hit(4750.0));
        assert!(position.is_take_profit_hit(4700.0));

        // SL hit when price goes above target
        assert!(!position.is_stop_loss_hit(4900.0));
        assert!(position.is_stop_loss_hit(4950.0));
        assert!(position.is_stop_loss_hit(5000.0));
    }

    #[test]
    fn test_position_manager() {
        let mut manager = PositionManager::new();

        let position1 = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 0.1)
            .with_take_profit(4950.0)
            .with_stop_loss(4800.0);

        manager.add(position1);

        assert_eq!(manager.count(), 1);
        assert!(manager.has_position("FCPO"));
        assert!(!manager.has_position("GOLD"));

        // Update price
        manager.update_prices("FCPO", 4900.0);

        let positions = manager.open_positions();
        assert!((positions[0].current_pnl - 5.0).abs() < 0.001);

        // Check for exits
        let exits = manager.check_exits("FCPO", 4950.0);
        assert_eq!(exits.len(), 1);
        assert_eq!(exits[0].1, CloseReason::TakeProfit);
    }

    #[test]
    fn test_position_manager_close() {
        let mut manager = PositionManager::new();

        let position = Position::new("pos_1", "FCPO", OrderSide::Buy, 4850.0, 1.0);
        manager.add(position);

        let closed = manager.close("pos_1", 4900.0, CloseReason::TakeProfit);

        assert!(closed.is_some());
        let closed = closed.unwrap();
        assert!((closed.realized_pnl - 50.0).abs() < 0.001);
        assert_eq!(closed.close_reason, CloseReason::TakeProfit);

        assert_eq!(manager.count(), 0);
        assert_eq!(manager.closed_positions().len(), 1);
    }

    #[test]
    fn test_order_side_opposite() {
        assert_eq!(OrderSide::Buy.opposite(), OrderSide::Sell);
        assert_eq!(OrderSide::Sell.opposite(), OrderSide::Buy);
    }

    #[test]
    fn test_trailing_stop_buy() {
        let config = TrailingStopConfig {
            activation_percent: 1.0,  // Activate at +1%
            trail_percent: 0.5,       // Trail 0.5% behind
        };

        let mut position = Position::new("test", "FCPO", OrderSide::Buy, 4850.0, 1.0)
            .with_trailing_stop(config);

        // Price moves up but not enough to activate
        assert!(position.update_trailing_stop(4880.0).is_none()); // +0.62%
        assert!(!position.is_trailing_active());

        // Price moves to +1% - activates trailing
        assert!(position.update_trailing_stop(4898.5).is_none()); // +1.0%
        assert!(position.is_trailing_active());

        // Price continues up
        assert!(position.update_trailing_stop(4950.0).is_none()); // +2.06%

        // Trailing stop should be at 4950 * 0.995 = 4925.25
        assert!((position.trailing_stop_price().unwrap() - 4925.25).abs() < 0.1);

        // Price retraces - trailing stop hit
        let hit = position.update_trailing_stop(4920.0);
        assert!(hit.is_some());
    }

    #[test]
    fn test_trailing_stop_sell() {
        let config = TrailingStopConfig {
            activation_percent: 1.0,
            trail_percent: 0.5,
        };

        let mut position = Position::new("test", "FCPO", OrderSide::Sell, 4850.0, 1.0)
            .with_trailing_stop(config);

        // Price moves down to +1% profit for short
        assert!(position.update_trailing_stop(4801.5).is_none()); // +1.0%
        assert!(position.is_trailing_active());

        // Price continues down
        assert!(position.update_trailing_stop(4750.0).is_none()); // +2.06%

        // Trailing stop should be at 4750 * 1.005 = 4773.75
        assert!((position.trailing_stop_price().unwrap() - 4773.75).abs() < 0.1);

        // Price retraces up - trailing stop hit
        let hit = position.update_trailing_stop(4780.0);
        assert!(hit.is_some());
    }
}
