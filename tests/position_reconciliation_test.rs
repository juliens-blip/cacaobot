//! Position Reconciliation Integration Tests
//!
//! Tests for intermittent connection handling, cache behavior, and audit trail.

use palm_oil_bot::modules::trading::{
    BrokerPositionData, ConnectionState, OrderSide, Position, PositionReconciliationSystem,
    ReconciliationConfig, AuditEventType,
};
use chrono::Utc;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_position(id: &str, symbol: &str, side: OrderSide, entry: f64) -> Position {
    Position::new(id, symbol, side, entry, 1.0)
}

fn create_broker_position(id: i64, symbol: &str, side: OrderSide, entry: f64) -> BrokerPositionData {
    BrokerPositionData {
        position_id: id,
        symbol: symbol.to_string(),
        side,
        entry_price: entry,
        volume: 1.0,
        current_pnl: 0.0,
        received_at: Utc::now(),
    }
}

fn create_test_system() -> PositionReconciliationSystem {
    let config = ReconciliationConfig {
        min_reconciliation_interval_secs: 0, // Disable throttling for tests
        max_stale_duration_secs: 300,
        auto_remove_orphaned: true,
        auto_add_missing: true,
        max_audit_entries: 100,
    };
    PositionReconciliationSystem::with_config(config)
}

// ============================================================================
// Connection State Tests
// ============================================================================

#[tokio::test]
async fn test_initial_state_is_disconnected() {
    let system = create_test_system();
    assert_eq!(system.get_connection_state().await, ConnectionState::Disconnected);
    assert!(!system.is_connected().await);
}

#[tokio::test]
async fn test_connection_state_connected() {
    let system = create_test_system();

    system.set_connection_state(ConnectionState::Connected).await;

    assert_eq!(system.get_connection_state().await, ConnectionState::Connected);
    assert!(system.is_connected().await);
}

#[tokio::test]
async fn test_connection_state_reconnecting() {
    let system = create_test_system();

    system.set_connection_state(ConnectionState::Reconnecting).await;

    assert_eq!(system.get_connection_state().await, ConnectionState::Reconnecting);
    assert!(!system.is_connected().await);
}

#[tokio::test]
async fn test_connection_state_failed() {
    let system = create_test_system();

    system.set_connection_state(ConnectionState::Failed).await;

    assert_eq!(system.get_connection_state().await, ConnectionState::Failed);
    assert!(!system.is_connected().await);
}

// ============================================================================
// Intermittent Connection Tests
// ============================================================================

#[tokio::test]
async fn test_resync_triggered_after_disconnect_reconnect() {
    let system = create_test_system();

    // Initial connect from Disconnected triggers resync (safety behavior)
    system.set_connection_state(ConnectionState::Connected).await;
    assert!(system.is_resync_pending().await);

    // Clear the resync flag by reconciling
    system.reconcile(vec![]).await.unwrap();
    assert!(!system.is_resync_pending().await);

    // Disconnect
    system.set_connection_state(ConnectionState::Disconnected).await;

    // Reconnect should trigger resync again
    system.set_connection_state(ConnectionState::Connected).await;
    assert!(system.is_resync_pending().await);
}

#[tokio::test]
async fn test_resync_triggered_after_reconnecting_state() {
    let system = create_test_system();

    system.set_connection_state(ConnectionState::Connected).await;
    system.set_connection_state(ConnectionState::Reconnecting).await;
    system.set_connection_state(ConnectionState::Connected).await;

    assert!(system.is_resync_pending().await);
}

#[tokio::test]
async fn test_multiple_disconnect_reconnect_cycles() {
    let system = create_test_system();

    for _ in 0..5 {
        system.set_connection_state(ConnectionState::Connected).await;
        system.set_connection_state(ConnectionState::Disconnected).await;
    }

    system.set_connection_state(ConnectionState::Connected).await;
    assert!(system.is_resync_pending().await);
}

#[tokio::test]
async fn test_resync_cleared_after_reconciliation() {
    let system = create_test_system();

    // Trigger resync
    system.set_connection_state(ConnectionState::Connected).await;
    system.set_connection_state(ConnectionState::Disconnected).await;
    system.set_connection_state(ConnectionState::Connected).await;
    assert!(system.is_resync_pending().await);

    // Reconcile clears resync flag
    system.reconcile(vec![]).await.unwrap();
    assert!(!system.is_resync_pending().await);
}

// ============================================================================
// Cache Tests
// ============================================================================

#[tokio::test]
async fn test_cache_add_position() {
    let system = create_test_system();

    let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
    system.cache_position(pos).await;

    assert_eq!(system.position_count().await, 1);

    let retrieved = system.get_position("123").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().entry_price, 4850.0);
}

#[tokio::test]
async fn test_cache_update_position() {
    let system = create_test_system();

    // Add initial position
    let pos1 = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
    system.cache_position(pos1).await;

    // Update with new price
    let pos2 = create_test_position("123", "FCPO", OrderSide::Buy, 4860.0);
    system.cache_position(pos2).await;

    assert_eq!(system.position_count().await, 1);

    let retrieved = system.get_position("123").await.unwrap();
    assert_eq!(retrieved.entry_price, 4860.0);
}

#[tokio::test]
async fn test_cache_multiple_positions() {
    let system = create_test_system();

    system.cache_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0)).await;
    system.cache_position(create_test_position("2", "FCPO", OrderSide::Sell, 4860.0)).await;
    system.cache_position(create_test_position("3", "GOLD", OrderSide::Buy, 2000.0)).await;

    assert_eq!(system.position_count().await, 3);

    let all = system.get_all_positions().await;
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn test_cache_remove_position() {
    let system = create_test_system();

    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    let removed = system.remove_position("123", "test removal").await;
    assert!(removed.is_some());
    assert_eq!(system.position_count().await, 0);
}

#[tokio::test]
async fn test_cache_remove_nonexistent_position() {
    let system = create_test_system();

    let removed = system.remove_position("nonexistent", "test").await;
    assert!(removed.is_none());
}

#[tokio::test]
async fn test_cache_clear() {
    let system = create_test_system();

    system.cache_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0)).await;
    system.cache_position(create_test_position("2", "FCPO", OrderSide::Sell, 4860.0)).await;

    system.clear_cache().await;

    assert_eq!(system.position_count().await, 0);
}

// ============================================================================
// Reconciliation Tests
// ============================================================================

#[tokio::test]
async fn test_reconcile_empty_both_sides() {
    let system = create_test_system();

    let report = system.reconcile(vec![]).await.unwrap();

    assert!(report.is_clean());
    assert_eq!(report.synced.len(), 0);
    assert_eq!(report.orphaned.len(), 0);
    assert_eq!(report.missing.len(), 0);
}

#[tokio::test]
async fn test_reconcile_perfect_match() {
    let system = create_test_system();

    // Add local position
    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    // Matching broker position
    let broker_positions = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4850.0)];

    let report = system.reconcile(broker_positions).await.unwrap();

    assert!(report.is_clean());
    assert_eq!(report.synced.len(), 1);
}

#[tokio::test]
async fn test_reconcile_orphaned_position() {
    let system = create_test_system();

    // Local position with no broker match
    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    let report = system.reconcile(vec![]).await.unwrap();

    assert!(!report.is_clean());
    assert_eq!(report.orphaned.len(), 1);
    assert_eq!(system.position_count().await, 0); // Auto-removed
}

#[tokio::test]
async fn test_reconcile_missing_position() {
    let system = create_test_system();

    // Broker has position we don't have
    let broker_positions = vec![create_broker_position(456, "FCPO", OrderSide::Sell, 4900.0)];

    let report = system.reconcile(broker_positions).await.unwrap();

    assert!(!report.is_clean());
    assert_eq!(report.missing.len(), 1);
    assert_eq!(system.position_count().await, 1); // Auto-added

    let pos = system.get_position("456").await.unwrap();
    assert_eq!(pos.side, OrderSide::Sell);
    assert_eq!(pos.entry_price, 4900.0);
}

#[tokio::test]
async fn test_reconcile_entry_price_mismatch() {
    let system = create_test_system();

    // Local position
    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    // Broker position with different entry price
    let broker_positions = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4860.0)];

    let report = system.reconcile(broker_positions).await.unwrap();

    assert!(!report.is_clean());
    assert_eq!(report.mismatches.len(), 1);
    assert_eq!(report.mismatches[0].field, "entry_price");
    assert!(report.mismatches[0].local_value.contains("4850"));
    assert!(report.mismatches[0].broker_value.contains("4860"));
}

#[tokio::test]
async fn test_reconcile_volume_mismatch() {
    let system = create_test_system();

    // Local position with volume 1.0
    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    // Broker position with different volume
    let mut broker_pos = create_broker_position(123, "FCPO", OrderSide::Buy, 4850.0);
    broker_pos.volume = 2.0;

    let report = system.reconcile(vec![broker_pos]).await.unwrap();

    assert!(!report.is_clean());
    assert_eq!(report.mismatches.len(), 1);
    assert_eq!(report.mismatches[0].field, "volume");
}

#[tokio::test]
async fn test_reconcile_multiple_issues() {
    let system = create_test_system();

    // Local positions: 123 (orphaned), 456 (synced), 789 (mismatched)
    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;
    system.cache_position(create_test_position("456", "FCPO", OrderSide::Sell, 4860.0)).await;
    system.cache_position(create_test_position("789", "FCPO", OrderSide::Buy, 4870.0)).await;

    // Broker positions: 456 (synced), 789 (mismatched), 999 (missing)
    let broker_positions = vec![
        create_broker_position(456, "FCPO", OrderSide::Sell, 4860.0),
        create_broker_position(789, "FCPO", OrderSide::Buy, 4880.0), // Different price
        create_broker_position(999, "GOLD", OrderSide::Buy, 2000.0),
    ];

    let report = system.reconcile(broker_positions).await.unwrap();

    assert!(!report.is_clean());
    assert_eq!(report.orphaned.len(), 1); // 123
    assert_eq!(report.synced.len(), 2); // 456, 789
    assert_eq!(report.missing.len(), 1); // 999
    assert_eq!(report.mismatches.len(), 1); // 789 price mismatch

    // Position count: 456 + 789 + 999 = 3 (123 removed)
    assert_eq!(system.position_count().await, 3);
}

#[tokio::test]
async fn test_reconcile_preserves_non_broker_positions() {
    let system = create_test_system();

    // Non-numeric ID (dry-run position)
    system.cache_position(create_test_position("dry_run_001", "FCPO", OrderSide::Buy, 4850.0)).await;

    let report = system.reconcile(vec![]).await.unwrap();

    // Should not be considered orphaned
    assert_eq!(report.orphaned.len(), 0);
    assert_eq!(system.position_count().await, 1);
}

// ============================================================================
// Audit Trail Tests
// ============================================================================

#[tokio::test]
async fn test_audit_log_connection_changes() {
    let system = create_test_system();

    system.set_connection_state(ConnectionState::Connected).await;
    system.set_connection_state(ConnectionState::Disconnected).await;

    let log = system.get_audit_log().await;

    let connection_events: Vec<_> = log.iter()
        .filter(|e| matches!(e.event, AuditEventType::ConnectionStateChanged { .. }))
        .collect();

    assert!(connection_events.len() >= 2);
}

#[tokio::test]
async fn test_audit_log_position_added() {
    let system = create_test_system();

    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    let log = system.get_audit_log().await;

    let add_events: Vec<_> = log.iter()
        .filter(|e| matches!(e.event, AuditEventType::PositionAddedFromBroker { .. }))
        .collect();

    assert_eq!(add_events.len(), 1);
}

#[tokio::test]
async fn test_audit_log_position_removed() {
    let system = create_test_system();

    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;
    system.remove_position("123", "test removal").await;

    let log = system.get_audit_log().await;

    let remove_events: Vec<_> = log.iter()
        .filter(|e| matches!(e.event, AuditEventType::PositionRemoved { .. }))
        .collect();

    assert_eq!(remove_events.len(), 1);
}

#[tokio::test]
async fn test_audit_log_reconciliation_events() {
    let system = create_test_system();

    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    let broker_positions = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4850.0)];
    system.reconcile(broker_positions).await.unwrap();

    let log = system.get_audit_log().await;

    let start_events: Vec<_> = log.iter()
        .filter(|e| matches!(e.event, AuditEventType::ReconciliationStarted { .. }))
        .collect();

    let complete_events: Vec<_> = log.iter()
        .filter(|e| matches!(e.event, AuditEventType::ReconciliationCompleted { .. }))
        .collect();

    assert_eq!(start_events.len(), 1);
    assert_eq!(complete_events.len(), 1);
}

#[tokio::test]
async fn test_audit_log_mismatch_events() {
    let system = create_test_system();

    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    let broker_positions = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4860.0)];
    system.reconcile(broker_positions).await.unwrap();

    let log = system.get_audit_log().await;

    let mismatch_events: Vec<_> = log.iter()
        .filter(|e| matches!(e.event, AuditEventType::MismatchDetected { .. }))
        .collect();

    assert_eq!(mismatch_events.len(), 1);
}

#[tokio::test]
async fn test_audit_log_resync_triggered() {
    let system = create_test_system();

    system.trigger_resync("manual test").await;

    let log = system.get_audit_log().await;

    let resync_events: Vec<_> = log.iter()
        .filter(|e| matches!(e.event, AuditEventType::ResyncTriggered { .. }))
        .collect();

    assert_eq!(resync_events.len(), 1);
}

#[tokio::test]
async fn test_audit_log_since_timestamp() {
    let system = create_test_system();

    system.set_connection_state(ConnectionState::Connected).await;

    let checkpoint = Utc::now();

    // Wait a tiny bit to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    system.set_connection_state(ConnectionState::Disconnected).await;

    let log_since = system.get_audit_log_since(checkpoint).await;

    // Should only include events after checkpoint
    assert!(log_since.len() >= 1);
}

#[tokio::test]
async fn test_audit_log_cache_cleared() {
    let system = create_test_system();

    system.cache_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0)).await;
    system.cache_position(create_test_position("2", "FCPO", OrderSide::Sell, 4860.0)).await;

    system.clear_cache().await;

    let log = system.get_audit_log().await;

    let clear_events: Vec<_> = log.iter()
        .filter(|e| matches!(e.event, AuditEventType::CacheCleared { count: 2 }))
        .collect();

    assert_eq!(clear_events.len(), 1);
}

// ============================================================================
// State Export Tests
// ============================================================================

#[tokio::test]
async fn test_export_state() {
    let system = create_test_system();

    system.set_connection_state(ConnectionState::Connected).await;
    system.cache_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0)).await;
    system.cache_position(create_test_position("2", "FCPO", OrderSide::Sell, 4860.0)).await;

    let state = system.export_state().await;

    assert_eq!(state.connection_state, ConnectionState::Connected);
    assert_eq!(state.position_count, 2);
    assert!(state.last_connected.is_some());
    assert!(state.audit_log_count > 0);
}

#[tokio::test]
async fn test_connection_uptime() {
    let system = create_test_system();

    // Not connected - no uptime
    assert!(system.get_connection_uptime().await.is_none());

    system.set_connection_state(ConnectionState::Connected).await;

    // Connected - should have uptime
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let uptime = system.get_connection_uptime().await;
    assert!(uptime.is_some());
    assert!(uptime.unwrap().num_milliseconds() >= 10);
}

#[tokio::test]
async fn test_time_since_disconnect() {
    let system = create_test_system();

    // Never disconnected
    assert!(system.get_time_since_disconnect().await.is_none());

    system.set_connection_state(ConnectionState::Connected).await;
    system.set_connection_state(ConnectionState::Disconnected).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let time_since = system.get_time_since_disconnect().await;
    assert!(time_since.is_some());
    assert!(time_since.unwrap().num_milliseconds() >= 10);
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_config_auto_remove_orphaned_disabled() {
    let config = ReconciliationConfig {
        min_reconciliation_interval_secs: 0,
        auto_remove_orphaned: false, // Disabled
        auto_add_missing: true,
        ..Default::default()
    };
    let system = PositionReconciliationSystem::with_config(config);

    system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

    system.reconcile(vec![]).await.unwrap();

    // Position should NOT be removed
    assert_eq!(system.position_count().await, 1);
}

#[tokio::test]
async fn test_config_auto_add_missing_disabled() {
    let config = ReconciliationConfig {
        min_reconciliation_interval_secs: 0,
        auto_remove_orphaned: true,
        auto_add_missing: false, // Disabled
        ..Default::default()
    };
    let system = PositionReconciliationSystem::with_config(config);

    let broker_positions = vec![create_broker_position(456, "FCPO", OrderSide::Sell, 4900.0)];

    system.reconcile(broker_positions).await.unwrap();

    // Position should NOT be added
    assert_eq!(system.position_count().await, 0);
}

#[tokio::test]
async fn test_reconciliation_throttling() {
    let config = ReconciliationConfig {
        min_reconciliation_interval_secs: 60, // 60 seconds
        ..Default::default()
    };
    let system = PositionReconciliationSystem::with_config(config);

    // First reconciliation should succeed
    let result1 = system.reconcile(vec![]).await;
    assert!(result1.is_ok());

    // Second immediate reconciliation should fail
    let result2 = system.reconcile(vec![]).await;
    assert!(result2.is_err());
}

// ============================================================================
// Stress Tests
// ============================================================================

#[tokio::test]
async fn test_rapid_connection_state_changes() {
    let system = create_test_system();

    for _ in 0..100 {
        system.set_connection_state(ConnectionState::Connected).await;
        system.set_connection_state(ConnectionState::Disconnected).await;
    }

    // Should handle rapid changes without panic
    let log = system.get_audit_log().await;
    assert!(log.len() > 0);
}

#[tokio::test]
async fn test_large_position_cache() {
    let system = create_test_system();

    // Add 100 positions
    for i in 0..100 {
        let pos = create_test_position(
            &i.to_string(),
            "FCPO",
            if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell },
            4850.0 + i as f64,
        );
        system.cache_position(pos).await;
    }

    assert_eq!(system.position_count().await, 100);

    let all = system.get_all_positions().await;
    assert_eq!(all.len(), 100);
}

#[tokio::test]
async fn test_large_reconciliation() {
    let system = create_test_system();

    // Add 50 local positions
    for i in 0..50 {
        let pos = create_test_position(&i.to_string(), "FCPO", OrderSide::Buy, 4850.0);
        system.cache_position(pos).await;
    }

    // Create 50 broker positions (some overlap, some new)
    let broker_positions: Vec<_> = (25..75)
        .map(|i| create_broker_position(i, "FCPO", OrderSide::Buy, 4850.0))
        .collect();

    let report = system.reconcile(broker_positions).await.unwrap();

    // 0-24: orphaned (25)
    // 25-49: synced (25)
    // 50-74: missing (25)
    assert_eq!(report.orphaned.len(), 25);
    assert_eq!(report.synced.len(), 25);
    assert_eq!(report.missing.len(), 25);
}
