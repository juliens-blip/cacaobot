use palm_oil_bot::modules::trading::{CloseReason, OrderSide, Position, PositionDatabase};
use tempfile::TempDir;

fn create_position(id: &str, side: OrderSide, entry: f64) -> Position {
    Position::new(id.to_string(), "FCPO".to_string(), side, entry, 1.0)
}

fn create_db(temp_dir: &TempDir) -> PositionDatabase {
    let path = temp_dir.path().join("positions.db");
    PositionDatabase::new(path).expect("Failed to create PositionDatabase")
}

#[tokio::test]
async fn test_crash_recovery_reloads_positions() {
    let temp_dir = TempDir::new().expect("temp dir");

    {
        let db = create_db(&temp_dir);
        db.upsert_position(&create_position("101", OrderSide::Buy, 4850.0))
            .unwrap();
        db.upsert_position(&create_position("202", OrderSide::Sell, 4900.0))
            .unwrap();
    }

    let db = create_db(&temp_dir);
    let positions = db.get_open_positions().unwrap();
    assert_eq!(positions.len(), 2);
    let ids: Vec<String> = positions.into_iter().map(|p| p.id).collect();
    assert!(ids.contains(&"101".to_string()));
    assert!(ids.contains(&"202".to_string()));
}

#[tokio::test]
async fn test_crash_recovery_closed_trades_persist() {
    let temp_dir = TempDir::new().expect("temp dir");

    {
        let db = create_db(&temp_dir);
        db.upsert_position(&create_position("303", OrderSide::Buy, 4800.0))
            .unwrap();
        db.close_position("303", 4850.0, CloseReason::TakeProfit)
            .unwrap();
    }

    let db = create_db(&temp_dir);
    let trades = db.get_today_trades().unwrap();
    assert_eq!(trades.len(), 1);
}

#[tokio::test]
async fn test_close_position_removes_open_entry() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db = create_db(&temp_dir);

    db.upsert_position(&create_position("404", OrderSide::Buy, 4820.0))
        .unwrap();
    db.close_position("404", 4810.0, CloseReason::StopLoss)
        .unwrap();

    assert_eq!(db.count_open_positions().unwrap(), 0);
    assert!(db.get_position("404").unwrap().is_none());
}

#[tokio::test]
async fn test_delete_position_removes_entry() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db = create_db(&temp_dir);

    db.upsert_position(&create_position("505", OrderSide::Buy, 4850.0))
        .unwrap();
    db.upsert_position(&create_position("606", OrderSide::Sell, 4900.0))
        .unwrap();

    db.delete_position("505").unwrap();

    assert_eq!(db.count_open_positions().unwrap(), 1);
    assert!(db.get_position("505").unwrap().is_none());
}

#[tokio::test]
async fn test_daily_stats_persist_after_restart() {
    let temp_dir = TempDir::new().expect("temp dir");
    let date = chrono::Utc::now().date_naive().to_string();

    {
        let db = create_db(&temp_dir);
        db.update_daily_stats(&date, 100.0, true).unwrap();
        db.update_daily_stats(&date, -40.0, false).unwrap();
    }

    let db = create_db(&temp_dir);
    let stats = db.get_daily_stats(&date).unwrap().unwrap();
    assert_eq!(stats.total_trades, 2);
    assert_eq!(stats.winning_trades, 1);
    assert_eq!(stats.losing_trades, 1);
    assert!((stats.total_pnl - 60.0).abs() < 0.01);
}

#[tokio::test]
async fn test_count_open_positions_after_multiple_inserts() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db = create_db(&temp_dir);

    db.upsert_position(&create_position("707", OrderSide::Buy, 4800.0))
        .unwrap();
    db.upsert_position(&create_position("808", OrderSide::Sell, 4950.0))
        .unwrap();
    db.upsert_position(&create_position("909", OrderSide::Buy, 4875.0))
        .unwrap();

    assert_eq!(db.count_open_positions().unwrap(), 3);
}
