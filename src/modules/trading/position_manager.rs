//! Position Manager with persistence and cTrader reconciliation
//!
//! Provides a thread-safe position manager with:
//! - JSON persistence for crash recovery
//! - Reconciliation with cTrader broker state
//! - Position lifecycle management

use crate::error::{BotError, Result};
use crate::modules::trading::{CloseReason, OrderSide, Position};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Position state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedPosition {
    pub id: String,
    pub broker_id: Option<i64>,
    pub symbol: String,
    pub side: OrderSide,
    pub entry_price: f64,
    pub volume: f64,
    pub take_profit: Option<f64>,
    pub stop_loss: Option<f64>,
    pub opened_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl From<&Position> for PersistedPosition {
    fn from(pos: &Position) -> Self {
        Self {
            id: pos.id.clone(),
            broker_id: pos.id.parse().ok(),
            symbol: pos.symbol.clone(),
            side: pos.side,
            entry_price: pos.entry_price,
            volume: pos.volume,
            take_profit: pos.take_profit,
            stop_loss: pos.stop_loss,
            opened_at: pos.opened_at,
            last_updated: Utc::now(),
        }
    }
}

impl From<PersistedPosition> for Position {
    fn from(pp: PersistedPosition) -> Self {
        let mut pos = Position::new(pp.id, pp.symbol, pp.side, pp.entry_price, pp.volume);
        if let Some(tp) = pp.take_profit {
            pos = pos.with_take_profit(tp);
        }
        if let Some(sl) = pp.stop_loss {
            pos = pos.with_stop_loss(sl);
        }
        pos
    }
}

/// Closed position record for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedClosedPosition {
    pub position: PersistedPosition,
    pub close_price: f64,
    pub realized_pnl: f64,
    pub closed_at: DateTime<Utc>,
    pub close_reason: CloseReason,
}

/// Persistence state file format
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistenceState {
    pub version: u32,
    pub last_saved: DateTime<Utc>,
    pub open_positions: Vec<PersistedPosition>,
    pub closed_positions: Vec<PersistedClosedPosition>,
    pub daily_pnl: f64,
    pub total_trades: u32,
}

impl PersistenceState {
    pub fn new() -> Self {
        Self {
            version: 1,
            last_saved: Utc::now(),
            open_positions: Vec::new(),
            closed_positions: Vec::new(),
            daily_pnl: 0.0,
            total_trades: 0,
        }
    }
}

/// Broker position for reconciliation
#[derive(Debug, Clone)]
pub struct BrokerPosition {
    pub position_id: i64,
    pub symbol_id: i64,
    pub symbol: String,
    pub side: OrderSide,
    pub entry_price: f64,
    pub volume: f64,
    pub current_pnl: f64,
}

/// Reconciliation result
#[derive(Debug, Clone)]
pub struct ReconciliationResult {
    /// Positions that exist locally but not on broker (orphaned)
    pub orphaned_local: Vec<String>,
    /// Positions that exist on broker but not locally (missing)
    pub missing_local: Vec<i64>,
    /// Positions that were synced successfully
    pub synced: Vec<String>,
    /// Positions with mismatched data
    pub mismatched: Vec<(String, String)>, // (position_id, mismatch_reason)
}

impl ReconciliationResult {
    pub fn new() -> Self {
        Self {
            orphaned_local: Vec::new(),
            missing_local: Vec::new(),
            synced: Vec::new(),
            mismatched: Vec::new(),
        }
    }

    pub fn is_clean(&self) -> bool {
        self.orphaned_local.is_empty()
            && self.missing_local.is_empty()
            && self.mismatched.is_empty()
    }
}

impl Default for ReconciliationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe Position Manager with persistence
pub struct PersistentPositionManager {
    /// Open positions indexed by ID
    positions: Arc<RwLock<HashMap<String, Position>>>,
    /// Closed positions history
    closed_positions: Arc<RwLock<Vec<PersistedClosedPosition>>>,
    /// Persistence file path
    persistence_path: Option<PathBuf>,
    /// Daily P&L tracking
    daily_pnl: Arc<RwLock<f64>>,
    /// Total trades count
    total_trades: Arc<RwLock<u32>>,
    /// Auto-save enabled
    auto_save: bool,
}

impl PersistentPositionManager {
    /// Create a new position manager without persistence
    pub fn new() -> Self {
        Self {
            positions: Arc::new(RwLock::new(HashMap::new())),
            closed_positions: Arc::new(RwLock::new(Vec::new())),
            persistence_path: None,
            daily_pnl: Arc::new(RwLock::new(0.0)),
            total_trades: Arc::new(RwLock::new(0)),
            auto_save: false,
        }
    }

    /// Create a position manager with JSON persistence
    pub fn with_persistence(path: impl AsRef<Path>) -> Self {
        Self {
            positions: Arc::new(RwLock::new(HashMap::new())),
            closed_positions: Arc::new(RwLock::new(Vec::new())),
            persistence_path: Some(path.as_ref().to_path_buf()),
            daily_pnl: Arc::new(RwLock::new(0.0)),
            total_trades: Arc::new(RwLock::new(0)),
            auto_save: true,
        }
    }

    /// Load state from persistence file
    pub async fn load(&self) -> Result<()> {
        let path = match &self.persistence_path {
            Some(p) => p,
            None => return Ok(()),
        };

        if !path.exists() {
            info!("No persistence file found, starting fresh");
            return Ok(());
        }

        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            BotError::Config(format!("Failed to read persistence file: {}", e))
        })?;

        let state: PersistenceState = serde_json::from_str(&content).map_err(|e| {
            BotError::Config(format!("Failed to parse persistence file: {}", e))
        })?;

        // Restore positions
        {
            let mut positions = self.positions.write().await;
            for pp in state.open_positions {
                let pos: Position = pp.into();
                positions.insert(pos.id.clone(), pos);
            }
        }

        // Restore closed positions
        {
            let mut closed = self.closed_positions.write().await;
            *closed = state.closed_positions;
        }

        // Restore stats
        *self.daily_pnl.write().await = state.daily_pnl;
        *self.total_trades.write().await = state.total_trades;

        info!(
            "Loaded {} open positions, {} closed from persistence",
            self.positions.read().await.len(),
            self.closed_positions.read().await.len()
        );

        Ok(())
    }

    /// Save current state to persistence file
    pub async fn save(&self) -> Result<()> {
        let path = match &self.persistence_path {
            Some(p) => p,
            None => return Ok(()),
        };

        let state = PersistenceState {
            version: 1,
            last_saved: Utc::now(),
            open_positions: self
                .positions
                .read()
                .await
                .values()
                .map(PersistedPosition::from)
                .collect(),
            closed_positions: self.closed_positions.read().await.clone(),
            daily_pnl: *self.daily_pnl.read().await,
            total_trades: *self.total_trades.read().await,
        };

        let content = serde_json::to_string_pretty(&state).map_err(|e| {
            BotError::Config(format!("Failed to serialize state: {}", e))
        })?;

        tokio::fs::write(path, content).await.map_err(|e| {
            BotError::Config(format!("Failed to write persistence file: {}", e))
        })?;

        debug!("Saved {} positions to {:?}", state.open_positions.len(), path);

        Ok(())
    }

    /// Open a new position
    pub async fn open_position(&self, position: Position) -> Result<String> {
        let position_id = position.id.clone();

        {
            let mut positions = self.positions.write().await;
            if positions.contains_key(&position_id) {
                return Err(BotError::Trading(format!(
                    "Position {} already exists",
                    position_id
                )));
            }
            positions.insert(position_id.clone(), position);
        }

        info!("Opened position: {}", position_id);

        if self.auto_save {
            self.save().await?;
        }

        Ok(position_id)
    }

    /// Close a position
    pub async fn close_position(
        &self,
        position_id: &str,
        close_price: f64,
        reason: CloseReason,
    ) -> Result<f64> {
        let position = {
            let mut positions = self.positions.write().await;
            positions
                .remove(position_id)
                .ok_or_else(|| BotError::Trading(format!("Position {} not found", position_id)))?
        };

        let realized_pnl = position.calculate_pnl(close_price);

        let closed = PersistedClosedPosition {
            position: PersistedPosition::from(&position),
            close_price,
            realized_pnl,
            closed_at: Utc::now(),
            close_reason: reason,
        };

        {
            let mut closed_positions = self.closed_positions.write().await;
            closed_positions.push(closed);
        }

        // Update stats
        *self.daily_pnl.write().await += realized_pnl;
        *self.total_trades.write().await += 1;

        info!(
            "Closed position {}: P&L={:.2}, reason={:?}",
            position_id, realized_pnl, reason
        );

        if self.auto_save {
            self.save().await?;
        }

        Ok(realized_pnl)
    }

    /// Get all open positions
    pub async fn get_all(&self) -> Vec<Position> {
        self.positions.read().await.values().cloned().collect()
    }

    /// Get a specific position by ID
    pub async fn get(&self, position_id: &str) -> Option<Position> {
        self.positions.read().await.get(position_id).cloned()
    }

    /// Get open position count
    pub async fn count(&self) -> usize {
        self.positions.read().await.len()
    }

    /// Check if any position is open for a symbol
    pub async fn has_position_for_symbol(&self, symbol: &str) -> bool {
        self.positions
            .read()
            .await
            .values()
            .any(|p| p.symbol == symbol)
    }

    /// Update all positions with current price
    pub async fn update_prices(&self, symbol: &str, price: f64) {
        let mut positions = self.positions.write().await;
        for position in positions.values_mut() {
            if position.symbol == symbol {
                position.update_price(price);
            }
        }
    }

    /// Get total unrealized P&L
    pub async fn total_unrealized_pnl(&self) -> f64 {
        self.positions
            .read()
            .await
            .values()
            .map(|p| p.current_pnl)
            .sum()
    }

    /// Get daily realized P&L
    pub async fn get_daily_pnl(&self) -> f64 {
        *self.daily_pnl.read().await
    }

    /// Get total trades count
    pub async fn get_total_trades(&self) -> u32 {
        *self.total_trades.read().await
    }

    /// Get closed positions
    pub async fn get_closed_positions(&self) -> Vec<PersistedClosedPosition> {
        self.closed_positions.read().await.clone()
    }

    /// Reset daily stats (call at midnight)
    pub async fn reset_daily(&self) {
        *self.daily_pnl.write().await = 0.0;
        info!("Daily P&L reset");
    }

    /// Reconcile local state with cTrader broker positions
    ///
    /// This method compares local positions with broker state and:
    /// - Identifies orphaned local positions (closed on broker)
    /// - Identifies missing local positions (opened on broker but not tracked)
    /// - Syncs position data (prices, P&L)
    pub async fn reconcile_with_ctrader(
        &self,
        broker_positions: Vec<BrokerPosition>,
    ) -> Result<ReconciliationResult> {
        let mut result = ReconciliationResult::new();
        let mut positions = self.positions.write().await;

        // Create lookup map for broker positions
        let broker_map: HashMap<i64, &BrokerPosition> = broker_positions
            .iter()
            .map(|bp| (bp.position_id, bp))
            .collect();

        // Check local positions against broker
        let local_ids: Vec<String> = positions.keys().cloned().collect();

        for local_id in local_ids {
            let broker_id: Option<i64> = local_id.parse().ok();

            match broker_id {
                Some(bid) if broker_map.contains_key(&bid) => {
                    // Position exists on both sides - sync data
                    let broker_pos = broker_map.get(&bid).unwrap();

                    if let Some(local_pos) = positions.get_mut(&local_id) {
                        // Check for mismatches
                        if (local_pos.entry_price - broker_pos.entry_price).abs() > 0.01 {
                            result.mismatched.push((
                                local_id.clone(),
                                format!(
                                    "Entry price mismatch: local={:.2}, broker={:.2}",
                                    local_pos.entry_price, broker_pos.entry_price
                                ),
                            ));
                        }

                        if (local_pos.volume - broker_pos.volume).abs() > 0.001 {
                            result.mismatched.push((
                                local_id.clone(),
                                format!(
                                    "Volume mismatch: local={:.3}, broker={:.3}",
                                    local_pos.volume, broker_pos.volume
                                ),
                            ));
                        }

                        // Sync current P&L from broker
                        local_pos.current_pnl = broker_pos.current_pnl;

                        result.synced.push(local_id.clone());
                    }
                }
                Some(_) => {
                    // Local position not found on broker - orphaned
                    warn!("Orphaned local position: {}", local_id);
                    result.orphaned_local.push(local_id);
                }
                None => {
                    // Non-numeric ID (e.g., dry_run positions) - skip
                    debug!("Skipping non-broker position: {}", local_id);
                }
            }
        }

        // Check for broker positions missing locally
        let local_broker_ids: std::collections::HashSet<i64> = positions
            .keys()
            .filter_map(|id| id.parse().ok())
            .collect();

        for broker_pos in &broker_positions {
            if !local_broker_ids.contains(&broker_pos.position_id) {
                warn!("Missing local position for broker ID: {}", broker_pos.position_id);
                result.missing_local.push(broker_pos.position_id);

                // Auto-add missing position
                let new_pos = Position::new(
                    broker_pos.position_id.to_string(),
                    broker_pos.symbol.clone(),
                    broker_pos.side,
                    broker_pos.entry_price,
                    broker_pos.volume,
                );
                positions.insert(new_pos.id.clone(), new_pos);
            }
        }

        // Remove orphaned positions
        for orphan_id in &result.orphaned_local {
            positions.remove(orphan_id);
        }

        if result.is_clean() {
            info!("Reconciliation complete: all {} positions synced", result.synced.len());
        } else {
            warn!(
                "Reconciliation issues: {} orphaned, {} missing, {} mismatched",
                result.orphaned_local.len(),
                result.missing_local.len(),
                result.mismatched.len()
            );
        }

        // Save after reconciliation
        drop(positions);
        if self.auto_save {
            self.save().await?;
        }

        Ok(result)
    }

    /// Force sync a position from broker data
    pub async fn sync_from_broker(&self, broker_pos: BrokerPosition) -> Result<()> {
        let position = Position::new(
            broker_pos.position_id.to_string(),
            broker_pos.symbol,
            broker_pos.side,
            broker_pos.entry_price,
            broker_pos.volume,
        );

        let mut positions = self.positions.write().await;
        positions.insert(position.id.clone(), position);

        info!("Synced position {} from broker", broker_pos.position_id);

        Ok(())
    }

    /// Clear all positions (for testing)
    #[cfg(test)]
    pub async fn clear_all(&self) {
        self.positions.write().await.clear();
        self.closed_positions.write().await.clear();
        *self.daily_pnl.write().await = 0.0;
        *self.total_trades.write().await = 0;
    }
}

impl Default for PersistentPositionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_position(id: &str, symbol: &str, side: OrderSide, entry: f64) -> Position {
        Position::new(id, symbol, side, entry, 1.0)
            .with_take_profit(entry * 1.02)
            .with_stop_loss(entry * 0.985)
    }

    #[tokio::test]
    async fn test_open_position() {
        let manager = PersistentPositionManager::new();

        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        let id = manager.open_position(pos).await.unwrap();

        assert_eq!(id, "123");
        assert_eq!(manager.count().await, 1);
    }

    #[tokio::test]
    async fn test_open_duplicate_position_fails() {
        let manager = PersistentPositionManager::new();

        let pos1 = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        let pos2 = create_test_position("123", "FCPO", OrderSide::Sell, 4900.0);

        manager.open_position(pos1).await.unwrap();
        let result = manager.open_position(pos2).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_close_position() {
        let manager = PersistentPositionManager::new();

        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        manager.open_position(pos).await.unwrap();

        let pnl = manager
            .close_position("123", 4900.0, CloseReason::TakeProfit)
            .await
            .unwrap();

        assert!((pnl - 50.0).abs() < 0.01);
        assert_eq!(manager.count().await, 0);
        assert_eq!(manager.get_closed_positions().await.len(), 1);
    }

    #[tokio::test]
    async fn test_close_nonexistent_position_fails() {
        let manager = PersistentPositionManager::new();

        let result = manager
            .close_position("nonexistent", 4900.0, CloseReason::Manual)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all() {
        let manager = PersistentPositionManager::new();

        manager
            .open_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();
        manager
            .open_position(create_test_position("2", "FCPO", OrderSide::Sell, 4860.0))
            .await
            .unwrap();

        let positions = manager.get_all().await;
        assert_eq!(positions.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let manager = PersistentPositionManager::new();

        manager
            .open_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();

        let pos = manager.get("123").await;
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().entry_price, 4850.0);

        let missing = manager.get("456").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_has_position_for_symbol() {
        let manager = PersistentPositionManager::new();

        assert!(!manager.has_position_for_symbol("FCPO").await);

        manager
            .open_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();

        assert!(manager.has_position_for_symbol("FCPO").await);
        assert!(!manager.has_position_for_symbol("GOLD").await);
    }

    #[tokio::test]
    async fn test_update_prices() {
        let manager = PersistentPositionManager::new();

        manager
            .open_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();

        manager.update_prices("FCPO", 4900.0).await;

        let pos = manager.get("1").await.unwrap();
        assert!((pos.current_pnl - 50.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_daily_pnl_tracking() {
        let manager = PersistentPositionManager::new();

        // Open and close with profit
        manager
            .open_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();
        manager
            .close_position("1", 4900.0, CloseReason::TakeProfit)
            .await
            .unwrap();

        // Open and close with loss
        manager
            .open_position(create_test_position("2", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();
        manager
            .close_position("2", 4800.0, CloseReason::StopLoss)
            .await
            .unwrap();

        // Net P&L: +50 - 50 = 0
        assert!((manager.get_daily_pnl().await - 0.0).abs() < 0.01);
        assert_eq!(manager.get_total_trades().await, 2);
    }

    #[tokio::test]
    async fn test_reset_daily() {
        let manager = PersistentPositionManager::new();

        manager
            .open_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();
        manager
            .close_position("1", 4900.0, CloseReason::TakeProfit)
            .await
            .unwrap();

        assert!(manager.get_daily_pnl().await > 0.0);

        manager.reset_daily().await;

        assert_eq!(manager.get_daily_pnl().await, 0.0);
    }

    #[tokio::test]
    async fn test_persistence_save_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Create and save
        {
            let manager = PersistentPositionManager::with_persistence(&path);
            manager
                .open_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0))
                .await
                .unwrap();
            manager.save().await.unwrap();
        }

        // Load and verify
        {
            let manager = PersistentPositionManager::with_persistence(&path);
            manager.load().await.unwrap();

            assert_eq!(manager.count().await, 1);
            let pos = manager.get("123").await.unwrap();
            assert_eq!(pos.entry_price, 4850.0);
            assert_eq!(pos.side, OrderSide::Buy);
        }
    }

    #[tokio::test]
    async fn test_persistence_with_closed_positions() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Create, close, and save
        {
            let manager = PersistentPositionManager::with_persistence(&path);
            manager
                .open_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0))
                .await
                .unwrap();
            manager
                .close_position("1", 4900.0, CloseReason::TakeProfit)
                .await
                .unwrap();
            manager.save().await.unwrap();
        }

        // Load and verify
        {
            let manager = PersistentPositionManager::with_persistence(&path);
            manager.load().await.unwrap();

            assert_eq!(manager.count().await, 0);
            let closed = manager.get_closed_positions().await;
            assert_eq!(closed.len(), 1);
            assert!((closed[0].realized_pnl - 50.0).abs() < 0.01);
        }
    }

    #[tokio::test]
    async fn test_reconcile_clean() {
        let manager = PersistentPositionManager::new();

        manager
            .open_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();

        let broker_positions = vec![BrokerPosition {
            position_id: 123,
            symbol_id: 1,
            symbol: "FCPO".to_string(),
            side: OrderSide::Buy,
            entry_price: 4850.0,
            volume: 1.0,
            current_pnl: 25.0,
        }];

        let result = manager.reconcile_with_ctrader(broker_positions).await.unwrap();

        assert!(result.is_clean());
        assert_eq!(result.synced.len(), 1);
    }

    #[tokio::test]
    async fn test_reconcile_orphaned_local() {
        let manager = PersistentPositionManager::new();

        // Local position that doesn't exist on broker
        manager
            .open_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();

        // Empty broker state
        let broker_positions = vec![];

        let result = manager.reconcile_with_ctrader(broker_positions).await.unwrap();

        assert_eq!(result.orphaned_local.len(), 1);
        assert_eq!(result.orphaned_local[0], "123");
        // Orphaned position should be removed
        assert_eq!(manager.count().await, 0);
    }

    #[tokio::test]
    async fn test_reconcile_missing_local() {
        let manager = PersistentPositionManager::new();

        // Broker has a position we don't track
        let broker_positions = vec![BrokerPosition {
            position_id: 456,
            symbol_id: 1,
            symbol: "FCPO".to_string(),
            side: OrderSide::Sell,
            entry_price: 4900.0,
            volume: 0.5,
            current_pnl: -10.0,
        }];

        let result = manager.reconcile_with_ctrader(broker_positions).await.unwrap();

        assert_eq!(result.missing_local.len(), 1);
        assert_eq!(result.missing_local[0], 456);
        // Missing position should be auto-added
        assert_eq!(manager.count().await, 1);
        let pos = manager.get("456").await.unwrap();
        assert_eq!(pos.side, OrderSide::Sell);
    }

    #[tokio::test]
    async fn test_reconcile_mismatched() {
        let manager = PersistentPositionManager::new();

        // Local position with different entry price
        manager
            .open_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();

        let broker_positions = vec![BrokerPosition {
            position_id: 123,
            symbol_id: 1,
            symbol: "FCPO".to_string(),
            side: OrderSide::Buy,
            entry_price: 4860.0, // Different from local
            volume: 1.0,
            current_pnl: 20.0,
        }];

        let result = manager.reconcile_with_ctrader(broker_positions).await.unwrap();

        assert_eq!(result.mismatched.len(), 1);
        assert!(result.mismatched[0].1.contains("Entry price mismatch"));
    }

    #[tokio::test]
    async fn test_sync_from_broker() {
        let manager = PersistentPositionManager::new();

        let broker_pos = BrokerPosition {
            position_id: 789,
            symbol_id: 1,
            symbol: "GOLD".to_string(),
            side: OrderSide::Sell,
            entry_price: 2000.0,
            volume: 0.1,
            current_pnl: 5.0,
        };

        manager.sync_from_broker(broker_pos).await.unwrap();

        let pos = manager.get("789").await.unwrap();
        assert_eq!(pos.symbol, "GOLD");
        assert_eq!(pos.side, OrderSide::Sell);
    }

    #[tokio::test]
    async fn test_total_unrealized_pnl() {
        let manager = PersistentPositionManager::new();

        manager
            .open_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0))
            .await
            .unwrap();
        manager
            .open_position(create_test_position("2", "FCPO", OrderSide::Sell, 4860.0))
            .await
            .unwrap();

        manager.update_prices("FCPO", 4870.0).await;

        // Position 1 (Buy at 4850): +20
        // Position 2 (Sell at 4860): -10
        // Total: +10
        assert!((manager.total_unrealized_pnl().await - 10.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let manager = PersistentPositionManager::with_persistence("/nonexistent/path/file.json");
        // Should not fail, just start fresh
        let result = manager.load().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_persisted_position_conversion() {
        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        let persisted = PersistedPosition::from(&pos);

        assert_eq!(persisted.id, "123");
        assert_eq!(persisted.symbol, "FCPO");
        assert_eq!(persisted.side, OrderSide::Buy);
        assert_eq!(persisted.entry_price, 4850.0);
        assert_eq!(persisted.take_profit, Some(4850.0 * 1.02));

        let restored: Position = persisted.into();
        assert_eq!(restored.id, "123");
        assert_eq!(restored.entry_price, 4850.0);
    }

    #[tokio::test]
    async fn test_reconciliation_result_is_clean() {
        let mut result = ReconciliationResult::new();
        assert!(result.is_clean());

        result.orphaned_local.push("123".to_string());
        assert!(!result.is_clean());
    }
}
