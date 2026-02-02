use palm_oil_bot::modules::trading::{BrokerPosition, OrderSide, Position, ReconciliationEngine};
use std::collections::HashMap;

fn local_position(id: &str, symbol: &str, side: OrderSide, entry: f64, volume: f64) -> Position {
    Position::new(id.to_string(), symbol.to_string(), side, entry, volume)
}

fn broker_position(
    position_id: i64,
    symbol: &str,
    side: OrderSide,
    entry: f64,
    volume: f64,
) -> BrokerPosition {
    BrokerPosition {
        position_id,
        symbol_id: 1,
        symbol: symbol.to_string(),
        side,
        entry_price: entry,
        volume,
        current_pnl: 0.0,
    }
}

fn map_positions(positions: Vec<Position>) -> HashMap<String, Position> {
    let mut map = HashMap::new();
    for pos in positions {
        map.insert(pos.id.clone(), pos);
    }
    map
}

#[tokio::test]
async fn test_reconcile_clean() {
    let engine = ReconciliationEngine::new();
    let local = map_positions(vec![local_position("100", "FCPO", OrderSide::Buy, 4850.0, 1.0)]);
    let broker = vec![broker_position(100, "FCPO", OrderSide::Buy, 4850.0, 1.0)];

    let result = engine.reconcile(&local, &broker);

    assert!(result.is_clean());
    assert_eq!(result.synced.len(), 1);
}

#[tokio::test]
async fn test_reconcile_orphaned_local() {
    let engine = ReconciliationEngine::new();
    let local = map_positions(vec![local_position("200", "FCPO", OrderSide::Buy, 4850.0, 1.0)]);
    let broker = Vec::new();

    let result = engine.reconcile(&local, &broker);

    assert!(!result.is_clean());
    assert_eq!(result.orphaned_local, vec!["200".to_string()]);
}

#[tokio::test]
async fn test_reconcile_missing_local() {
    let engine = ReconciliationEngine::new();
    let local = HashMap::new();
    let broker = vec![broker_position(300, "FCPO", OrderSide::Sell, 4920.0, 1.0)];

    let result = engine.reconcile(&local, &broker);

    assert!(!result.is_clean());
    assert_eq!(result.missing_local, vec![300]);
}

#[tokio::test]
async fn test_reconcile_symbol_mismatch() {
    let engine = ReconciliationEngine::new();
    let local = map_positions(vec![local_position("400", "FCPO", OrderSide::Buy, 4850.0, 1.0)]);
    let broker = vec![broker_position(400, "GOLD", OrderSide::Buy, 4850.0, 1.0)];

    let result = engine.reconcile(&local, &broker);

    assert_eq!(result.mismatched.len(), 1);
    assert!(result.mismatched[0].1.contains("Symbol mismatch"));
}

#[tokio::test]
async fn test_reconcile_side_mismatch() {
    let engine = ReconciliationEngine::new();
    let local = map_positions(vec![local_position("500", "FCPO", OrderSide::Buy, 4850.0, 1.0)]);
    let broker = vec![broker_position(500, "FCPO", OrderSide::Sell, 4850.0, 1.0)];

    let result = engine.reconcile(&local, &broker);

    assert_eq!(result.mismatched.len(), 1);
    assert!(result.mismatched[0].1.contains("Side mismatch"));
}

#[tokio::test]
async fn test_reconcile_price_tolerance_within_limit() {
    let engine = ReconciliationEngine::with_settings(0.5, 0.001, true, true);
    let local = map_positions(vec![local_position("600", "FCPO", OrderSide::Buy, 4850.0, 1.0)]);
    let broker = vec![broker_position(600, "FCPO", OrderSide::Buy, 4870.0, 1.0)];

    let result = engine.reconcile(&local, &broker);

    assert_eq!(result.mismatched.len(), 0);
    assert_eq!(result.synced.len(), 1);
}

#[tokio::test]
async fn test_reconcile_volume_mismatch() {
    let engine = ReconciliationEngine::new();
    let local = map_positions(vec![local_position("700", "FCPO", OrderSide::Buy, 4850.0, 1.0)]);
    let broker = vec![broker_position(700, "FCPO", OrderSide::Buy, 4850.0, 1.2)];

    let result = engine.reconcile(&local, &broker);

    assert_eq!(result.mismatched.len(), 1);
    assert!(result.mismatched[0].1.contains("Volume mismatch"));
}

#[tokio::test]
async fn test_auto_heal_adds_and_removes() {
    let engine = ReconciliationEngine::new();
    let local = map_positions(vec![local_position("800", "FCPO", OrderSide::Buy, 4850.0, 1.0)]);
    let broker = vec![broker_position(900, "GOLD", OrderSide::Sell, 2000.0, 1.0)];

    let result = engine.reconcile(&local, &broker);
    let (to_add, to_remove) = engine.auto_heal(&result, &broker);

    assert_eq!(to_add.len(), 1);
    assert_eq!(to_add[0].id, "900");
    assert_eq!(to_remove.len(), 1);
    assert_eq!(to_remove[0], "800");
}

#[tokio::test]
async fn test_generate_missing_positions_sets_tp_sl() {
    let engine = ReconciliationEngine::new();
    let broker = vec![broker_position(1000, "FCPO", OrderSide::Buy, 4850.0, 1.0)];

    let positions = engine.generate_missing_positions(&broker);

    assert_eq!(positions.len(), 1);
    assert!(positions[0].take_profit.is_some());
    assert!(positions[0].stop_loss.is_some());
}
