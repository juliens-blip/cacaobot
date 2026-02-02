use palm_oil_bot::modules::trading::{
    BrokerPosition, OrderSide, Position, PositionDatabase, ReconciliationEngine,
};
use std::collections::HashMap;
use tempfile::TempDir;

fn create_db(temp_dir: &TempDir) -> PositionDatabase {
    let path = temp_dir.path().join("positions.db");
    PositionDatabase::new(path).expect("Failed to create PositionDatabase")
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

fn load_local_map(db: &PositionDatabase) -> HashMap<String, Position> {
    db.get_open_positions()
        .unwrap()
        .into_iter()
        .map(|pos| (pos.id.clone(), pos))
        .collect()
}

fn apply_reconciliation(
    db: &PositionDatabase,
    engine: &ReconciliationEngine,
    broker_positions: &[BrokerPosition],
) {
    let local_map = load_local_map(db);
    let result = engine.reconcile(&local_map, broker_positions);
    let (to_add, to_remove) = engine.auto_heal(&result, broker_positions);

    for pos in to_add {
        db.upsert_position(&pos).unwrap();
    }
    for id in to_remove {
        db.delete_position(&id).unwrap();
    }
}

#[tokio::test]
async fn test_full_stack_clean_recovery_and_resume() {
    let temp_dir = TempDir::new().expect("temp dir");

    {
        let db = create_db(&temp_dir);
        db.upsert_position(&Position::new(
            "1001".to_string(),
            "FCPO".to_string(),
            OrderSide::Buy,
            4850.0,
            1.0,
        ))
        .unwrap();
    }

    let db = create_db(&temp_dir);
    let engine = ReconciliationEngine::new();
    let broker = vec![broker_position(1001, "FCPO", OrderSide::Buy, 4850.0, 1.0)];

    apply_reconciliation(&db, &engine, &broker);
    assert_eq!(db.count_open_positions().unwrap(), 1);

    db.upsert_position(&Position::new(
        "1002".to_string(),
        "FCPO".to_string(),
        OrderSide::Sell,
        4900.0,
        1.0,
    ))
    .unwrap();

    assert_eq!(db.count_open_positions().unwrap(), 2);
}

#[tokio::test]
async fn test_full_stack_orphaned_cleanup_then_resume() {
    let temp_dir = TempDir::new().expect("temp dir");

    {
        let db = create_db(&temp_dir);
        db.upsert_position(&Position::new(
            "2001".to_string(),
            "FCPO".to_string(),
            OrderSide::Buy,
            4850.0,
            1.0,
        ))
        .unwrap();
    }

    let db = create_db(&temp_dir);
    let engine = ReconciliationEngine::new();
    let broker = Vec::new();

    apply_reconciliation(&db, &engine, &broker);
    assert_eq!(db.count_open_positions().unwrap(), 0);

    db.upsert_position(&Position::new(
        "2002".to_string(),
        "FCPO".to_string(),
        OrderSide::Buy,
        4860.0,
        1.0,
    ))
    .unwrap();

    assert_eq!(db.count_open_positions().unwrap(), 1);
}

#[tokio::test]
async fn test_full_stack_missing_local_added_then_resume() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db = create_db(&temp_dir);
    let engine = ReconciliationEngine::new();

    let broker = vec![broker_position(3001, "FCPO", OrderSide::Sell, 4920.0, 1.0)];

    apply_reconciliation(&db, &engine, &broker);
    assert_eq!(db.count_open_positions().unwrap(), 1);
    assert!(db.get_position("3001").unwrap().is_some());

    db.upsert_position(&Position::new(
        "3002".to_string(),
        "FCPO".to_string(),
        OrderSide::Buy,
        4880.0,
        1.0,
    ))
    .unwrap();

    assert_eq!(db.count_open_positions().unwrap(), 2);
}
