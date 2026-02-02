//! Position reconciliation between local state and cTrader broker
//!
//! Handles:
//! - Startup reconciliation (crash recovery)
//! - Periodic reconciliation (drift detection)
//! - Auto-healing (sync discrepancies)

use crate::modules::trading::position_manager::{BrokerPosition, ReconciliationResult};
use crate::modules::trading::{OrderSide, Position};

use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Reconciliation engine
pub struct ReconciliationEngine {
    /// Tolerance for price differences (in percentage)
    price_tolerance_percent: f64,
    /// Tolerance for volume differences
    volume_tolerance: f64,
    /// Auto-sync missing positions
    auto_sync_missing: bool,
    /// Auto-close orphaned positions
    auto_close_orphaned: bool,
}

impl ReconciliationEngine {
    /// Create a new reconciliation engine
    pub fn new() -> Self {
        Self {
            price_tolerance_percent: 0.1, // 0.1% price difference allowed
            volume_tolerance: 0.001,       // 0.001 lots volume difference allowed
            auto_sync_missing: true,
            auto_close_orphaned: true,
        }
    }

    /// Create with custom settings
    pub fn with_settings(
        price_tolerance_percent: f64,
        volume_tolerance: f64,
        auto_sync: bool,
        auto_close: bool,
    ) -> Self {
        Self {
            price_tolerance_percent,
            volume_tolerance,
            auto_sync_missing: auto_sync,
            auto_close_orphaned: auto_close,
        }
    }

    /// Reconcile local positions with broker state
    ///
    /// Returns ReconciliationResult with details about:
    /// - Orphaned: Local positions not on broker (likely closed externally)
    /// - Missing: Broker positions not tracked locally (manual trades or crash recovery)
    /// - Synced: Positions that match
    /// - Mismatched: Positions with different data
    pub fn reconcile(
        &self,
        local_positions: &HashMap<String, Position>,
        broker_positions: &[BrokerPosition],
    ) -> ReconciliationResult {
        let mut result = ReconciliationResult::new();

        // Build broker position map by ID
        let broker_map: HashMap<i64, &BrokerPosition> = broker_positions
            .iter()
            .map(|bp| (bp.position_id, bp))
            .collect();

        // Check each local position
        for (local_id, local_pos) in local_positions {
            // Try to parse local ID as broker ID
            if let Ok(broker_id) = local_id.parse::<i64>() {
                if let Some(broker_pos) = broker_map.get(&broker_id) {
                    // Position exists on both sides - check for mismatches
                    if let Some(mismatch) = self.check_mismatch(local_pos, broker_pos) {
                        result
                            .mismatched
                            .push((local_id.clone(), mismatch));
                    } else {
                        result.synced.push(local_id.clone());
                    }
                } else {
                    // Local position not found on broker - orphaned
                    warn!(
                        "Orphaned local position detected: {} (not on broker)",
                        local_id
                    );
                    result.orphaned_local.push(local_id.clone());
                }
            } else {
                // Local ID not parseable as broker ID - treat as orphaned
                warn!(
                    "Local position {} has invalid broker ID format",
                    local_id
                );
                result.orphaned_local.push(local_id.clone());
            }
        }

        // Check for broker positions not tracked locally
        for broker_pos in broker_positions {
            let local_id = broker_pos.position_id.to_string();
            if !local_positions.contains_key(&local_id) {
                warn!(
                    "Missing local position for broker ID: {} (symbol: {})",
                    broker_pos.position_id, broker_pos.symbol
                );
                result.missing_local.push(broker_pos.position_id);
            }
        }

        info!(
            "Reconciliation complete: {} synced, {} orphaned, {} missing, {} mismatched",
            result.synced.len(),
            result.orphaned_local.len(),
            result.missing_local.len(),
            result.mismatched.len()
        );

        result
    }

    /// Check for mismatches between local and broker positions
    fn check_mismatch(&self, local: &Position, broker: &BrokerPosition) -> Option<String> {
        // Check symbol match
        if local.symbol != broker.symbol {
            return Some(format!(
                "Symbol mismatch: local={}, broker={}",
                local.symbol, broker.symbol
            ));
        }

        // Check side match
        if local.side != broker.side {
            return Some(format!(
                "Side mismatch: local={:?}, broker={:?}",
                local.side, broker.side
            ));
        }

        // Check entry price within tolerance
        let price_diff_percent =
            ((local.entry_price - broker.entry_price).abs() / broker.entry_price) * 100.0;
        if price_diff_percent > self.price_tolerance_percent {
            return Some(format!(
                "Entry price mismatch: local={:.2}, broker={:.2} (diff: {:.3}%)",
                local.entry_price, broker.entry_price, price_diff_percent
            ));
        }

        // Check volume within tolerance
        let volume_diff = (local.volume - broker.volume).abs();
        if volume_diff > self.volume_tolerance {
            return Some(format!(
                "Volume mismatch: local={:.4}, broker={:.4} (diff: {:.4})",
                local.volume, broker.volume, volume_diff
            ));
        }

        None // No mismatch
    }

    /// Generate positions to add from broker (for missing local positions)
    pub fn generate_missing_positions(&self, broker_positions: &[BrokerPosition]) -> Vec<Position> {
        broker_positions
            .iter()
            .map(|bp| {
                let mut pos = Position::new(
                    bp.position_id.to_string(),
                    bp.symbol.clone(),
                    bp.side,
                    bp.entry_price,
                    bp.volume,
                );

                // Set default TP/SL if not present
                // (Could be extended to fetch from broker if available)
                let tp = match bp.side {
                    OrderSide::Buy => bp.entry_price * 1.02,
                    OrderSide::Sell => bp.entry_price * 0.98,
                };
                let sl = match bp.side {
                    OrderSide::Buy => bp.entry_price * 0.985,
                    OrderSide::Sell => bp.entry_price * 1.015,
                };

                pos = pos.with_take_profit(tp).with_stop_loss(sl);

                debug!(
                    "Generated position from broker: {} {} {} @ {:.2}",
                    pos.id, pos.side, pos.symbol, pos.entry_price
                );

                pos
            })
            .collect()
    }

    /// Auto-heal: Apply reconciliation actions
    ///
    /// Returns tuple: (positions_to_add, position_ids_to_remove)
    pub fn auto_heal(
        &self,
        result: &ReconciliationResult,
        broker_positions: &[BrokerPosition],
    ) -> (Vec<Position>, Vec<String>) {
        let mut to_add = Vec::new();
        let mut to_remove = Vec::new();

        // Add missing positions if auto-sync enabled
        if self.auto_sync_missing && !result.missing_local.is_empty() {
            let missing_brokers: Vec<_> = broker_positions
                .iter()
                .filter(|bp| result.missing_local.contains(&bp.position_id))
                .collect();

            for broker_pos in missing_brokers {
                let pos = Position::new(
                    broker_pos.position_id.to_string(),
                    broker_pos.symbol.clone(),
                    broker_pos.side,
                    broker_pos.entry_price,
                    broker_pos.volume,
                );
                info!(
                    "Auto-healing: Adding missing position {} from broker",
                    pos.id
                );
                to_add.push(pos);
            }
        }

        // Remove orphaned positions if auto-close enabled
        if self.auto_close_orphaned {
            for orphaned_id in &result.orphaned_local {
                info!(
                    "Auto-healing: Removing orphaned position {} (not on broker)",
                    orphaned_id
                );
                to_remove.push(orphaned_id.clone());
            }
        }

        (to_add, to_remove)
    }

    /// Check if reconciliation result is clean (no issues)
    pub fn is_clean(&self, result: &ReconciliationResult) -> bool {
        result.is_clean()
    }

    /// Generate reconciliation report
    pub fn generate_report(&self, result: &ReconciliationResult) -> String {
        let mut report = String::new();

        report.push_str("=== RECONCILIATION REPORT ===\n\n");

        report.push_str(&format!("✅ Synced positions: {}\n", result.synced.len()));
        for id in &result.synced {
            report.push_str(&format!("  - {}\n", id));
        }

        if !result.orphaned_local.is_empty() {
            report.push_str(&format!(
                "\n⚠️  Orphaned local positions: {}\n",
                result.orphaned_local.len()
            ));
            for id in &result.orphaned_local {
                report.push_str(&format!("  - {} (not on broker)\n", id));
            }
        }

        if !result.missing_local.is_empty() {
            report.push_str(&format!(
                "\n⚠️  Missing local positions: {}\n",
                result.missing_local.len()
            ));
            for id in &result.missing_local {
                report.push_str(&format!("  - {} (on broker but not tracked)\n", id));
            }
        }

        if !result.mismatched.is_empty() {
            report.push_str(&format!(
                "\n❌ Mismatched positions: {}\n",
                result.mismatched.len()
            ));
            for (id, reason) in &result.mismatched {
                report.push_str(&format!("  - {}: {}\n", id, reason));
            }
        }

        report.push('\n');
        if result.is_clean() {
            report.push_str("✅ Status: CLEAN (all positions synced)\n");
        } else {
            report.push_str("⚠️  Status: REQUIRES ATTENTION\n");
        }

        report
    }
}

impl Default for ReconciliationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_position(id: &str, symbol: &str, side: OrderSide, entry: f64) -> Position {
        Position::new(id.to_string(), symbol.to_string(), side, entry, 1.0)
    }

    fn create_broker_position(
        id: i64,
        symbol: &str,
        side: OrderSide,
        entry: f64,
    ) -> BrokerPosition {
        BrokerPosition {
            position_id: id,
            symbol_id: 1,
            symbol: symbol.to_string(),
            side,
            entry_price: entry,
            volume: 1.0,
            current_pnl: 0.0,
        }
    }

    #[test]
    fn test_reconcile_clean() {
        let engine = ReconciliationEngine::new();

        let mut local = HashMap::new();
        local.insert(
            "123".to_string(),
            create_test_position("123", "FCPO", OrderSide::Buy, 4850.0),
        );

        let broker = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4850.0)];

        let result = engine.reconcile(&local, &broker);

        assert!(result.is_clean());
        assert_eq!(result.synced.len(), 1);
        assert_eq!(result.orphaned_local.len(), 0);
        assert_eq!(result.missing_local.len(), 0);
    }

    #[test]
    fn test_reconcile_orphaned() {
        let engine = ReconciliationEngine::new();

        let mut local = HashMap::new();
        local.insert(
            "123".to_string(),
            create_test_position("123", "FCPO", OrderSide::Buy, 4850.0),
        );

        let broker = vec![]; // Empty broker state

        let result = engine.reconcile(&local, &broker);

        assert!(!result.is_clean());
        assert_eq!(result.orphaned_local.len(), 1);
        assert_eq!(result.orphaned_local[0], "123");
    }

    #[test]
    fn test_reconcile_missing() {
        let engine = ReconciliationEngine::new();

        let local = HashMap::new(); // Empty local state

        let broker = vec![create_broker_position(456, "FCPO", OrderSide::Sell, 4900.0)];

        let result = engine.reconcile(&local, &broker);

        assert!(!result.is_clean());
        assert_eq!(result.missing_local.len(), 1);
        assert_eq!(result.missing_local[0], 456);
    }

    #[test]
    fn test_reconcile_price_mismatch() {
        let engine = ReconciliationEngine::new();

        let mut local = HashMap::new();
        local.insert(
            "123".to_string(),
            create_test_position("123", "FCPO", OrderSide::Buy, 4850.0),
        );

        let broker = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4860.0)]; // Different price

        let result = engine.reconcile(&local, &broker);

        assert_eq!(result.mismatched.len(), 1);
        assert!(result.mismatched[0].1.contains("Entry price mismatch"));
    }

    #[test]
    fn test_reconcile_side_mismatch() {
        let engine = ReconciliationEngine::new();

        let mut local = HashMap::new();
        local.insert(
            "123".to_string(),
            create_test_position("123", "FCPO", OrderSide::Buy, 4850.0),
        );

        let broker = vec![create_broker_position(123, "FCPO", OrderSide::Sell, 4850.0)]; // Different side

        let result = engine.reconcile(&local, &broker);

        assert_eq!(result.mismatched.len(), 1);
        assert!(result.mismatched[0].1.contains("Side mismatch"));
    }

    #[test]
    fn test_auto_heal() {
        let engine = ReconciliationEngine::new();

        let mut local = HashMap::new();
        local.insert(
            "123".to_string(),
            create_test_position("123", "FCPO", OrderSide::Buy, 4850.0),
        );

        let broker = vec![
            // 123 exists (synced)
            create_broker_position(123, "FCPO", OrderSide::Buy, 4850.0),
            // 456 missing locally
            create_broker_position(456, "GOLD", OrderSide::Sell, 2000.0),
        ];

        let result = engine.reconcile(&local, &broker);
        let (to_add, to_remove) = engine.auto_heal(&result, &broker);

        // Should add missing position 456
        assert_eq!(to_add.len(), 1);
        assert_eq!(to_add[0].id, "456");

        // No orphaned positions to remove
        assert_eq!(to_remove.len(), 0);
    }

    #[test]
    fn test_auto_heal_orphaned() {
        let engine = ReconciliationEngine::new();

        let mut local = HashMap::new();
        local.insert(
            "999".to_string(),
            create_test_position("999", "FCPO", OrderSide::Buy, 4850.0),
        );

        let broker = vec![]; // Empty broker

        let result = engine.reconcile(&local, &broker);
        let (to_add, to_remove) = engine.auto_heal(&result, &broker);

        // Should remove orphaned position
        assert_eq!(to_remove.len(), 1);
        assert_eq!(to_remove[0], "999");

        // Nothing to add
        assert_eq!(to_add.len(), 0);
    }

    #[test]
    fn test_generate_report() {
        let engine = ReconciliationEngine::new();

        let mut result = ReconciliationResult::new();
        result.synced.push("123".to_string());
        result.orphaned_local.push("456".to_string());
        result.missing_local.push(789);
        result.mismatched.push(("111".to_string(), "Price mismatch".to_string()));

        let report = engine.generate_report(&result);

        assert!(report.contains("Synced positions: 1"));
        assert!(report.contains("Orphaned local positions: 1"));
        assert!(report.contains("Missing local positions: 1"));
        assert!(report.contains("Mismatched positions: 1"));
        assert!(report.contains("REQUIRES ATTENTION"));
    }

    #[test]
    fn test_price_tolerance() {
        let engine = ReconciliationEngine::with_settings(0.5, 0.001, true, true);

        let mut local = HashMap::new();
        local.insert(
            "123".to_string(),
            create_test_position("123", "FCPO", OrderSide::Buy, 4850.0),
        );

        // Price difference: 4850 vs 4870 = 0.41% (within 0.5% tolerance)
        let broker = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4870.0)];

        let result = engine.reconcile(&local, &broker);

        // Should be synced (within tolerance)
        assert_eq!(result.synced.len(), 1);
        assert_eq!(result.mismatched.len(), 0);
    }

    #[test]
    fn test_generate_missing_positions() {
        let engine = ReconciliationEngine::new();

        let broker = vec![
            create_broker_position(123, "FCPO", OrderSide::Buy, 4850.0),
            create_broker_position(456, "GOLD", OrderSide::Sell, 2000.0),
        ];

        let positions = engine.generate_missing_positions(&broker);

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].id, "123");
        assert_eq!(positions[1].id, "456");

        // Check TP/SL are set
        assert!(positions[0].take_profit.is_some());
        assert!(positions[0].stop_loss.is_some());
    }
}
