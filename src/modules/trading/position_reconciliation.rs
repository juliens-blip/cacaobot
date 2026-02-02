//! Position Reconciliation System
//!
//! Provides a robust position reconciliation mechanism with:
//! - Local cache using HashMap<String, Position>
//! - Re-sync mechanism after connection loss/recovery
//! - Detailed audit trail with timestamps
//! - Connection state tracking for intermittent connections

use crate::error::{BotError, Result};
use crate::modules::trading::{OrderSide, Position};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Connection state for tracking broker connectivity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ConnectionState {
    /// Connected to broker
    Connected,
    /// Disconnected from broker
    #[default]
    Disconnected,
    /// Reconnecting after connection loss
    Reconnecting,
    /// Connection failed after retries
    Failed,
}


impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Connected => write!(f, "CONNECTED"),
            ConnectionState::Disconnected => write!(f, "DISCONNECTED"),
            ConnectionState::Reconnecting => write!(f, "RECONNECTING"),
            ConnectionState::Failed => write!(f, "FAILED"),
        }
    }
}

/// Audit event types for tracking reconciliation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Connection state changed
    ConnectionStateChanged {
        from: ConnectionState,
        to: ConnectionState,
    },
    /// Reconciliation started
    ReconciliationStarted {
        local_count: usize,
        broker_count: usize,
    },
    /// Reconciliation completed
    ReconciliationCompleted {
        synced: usize,
        orphaned: usize,
        missing: usize,
        mismatched: usize,
        duration_ms: u64,
    },
    /// Position added from broker
    PositionAddedFromBroker {
        position_id: String,
        symbol: String,
        side: OrderSide,
        entry_price: f64,
    },
    /// Position removed (orphaned)
    PositionRemoved {
        position_id: String,
        reason: String,
    },
    /// Position updated
    PositionUpdated {
        position_id: String,
        field: String,
        old_value: String,
        new_value: String,
    },
    /// Mismatch detected
    MismatchDetected {
        position_id: String,
        field: String,
        local_value: String,
        broker_value: String,
    },
    /// Re-sync triggered
    ResyncTriggered {
        reason: String,
    },
    /// Cache cleared
    CacheCleared {
        count: usize,
    },
}

/// Audit trail entry with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Event type with details
    pub event: AuditEventType,
    /// Optional correlation ID for tracking related events
    pub correlation_id: Option<String>,
}

impl AuditEntry {
    pub fn new(event: AuditEventType) -> Self {
        Self {
            timestamp: Utc::now(),
            event,
            correlation_id: None,
        }
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
}

/// Cached position with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPosition {
    /// The position data
    pub position: Position,
    /// When the position was cached
    pub cached_at: DateTime<Utc>,
    /// Last sync time with broker
    pub last_synced: DateTime<Utc>,
    /// Number of sync attempts
    pub sync_count: u32,
    /// Whether position is confirmed on broker
    pub broker_confirmed: bool,
}

impl CachedPosition {
    pub fn new(position: Position) -> Self {
        let now = Utc::now();
        Self {
            position,
            cached_at: now,
            last_synced: now,
            sync_count: 0,
            broker_confirmed: false,
        }
    }

    pub fn mark_synced(&mut self) {
        self.last_synced = Utc::now();
        self.sync_count += 1;
        self.broker_confirmed = true;
    }

    /// Check if position is stale (not synced recently)
    pub fn is_stale(&self, max_age: Duration) -> bool {
        Utc::now() - self.last_synced > max_age
    }
}

/// Broker position data for reconciliation
#[derive(Debug, Clone)]
pub struct BrokerPositionData {
    pub position_id: i64,
    pub symbol: String,
    pub side: OrderSide,
    pub entry_price: f64,
    pub volume: f64,
    pub current_pnl: f64,
    pub received_at: DateTime<Utc>,
}

/// Reconciliation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationConfig {
    /// Maximum age before position is considered stale
    pub max_stale_duration_secs: u64,
    /// Auto-remove orphaned positions
    pub auto_remove_orphaned: bool,
    /// Auto-add missing positions from broker
    pub auto_add_missing: bool,
    /// Maximum audit log entries to keep
    pub max_audit_entries: usize,
    /// Minimum interval between reconciliations (seconds)
    pub min_reconciliation_interval_secs: u64,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            max_stale_duration_secs: 300, // 5 minutes
            auto_remove_orphaned: true,
            auto_add_missing: true,
            max_audit_entries: 1000,
            min_reconciliation_interval_secs: 5,
        }
    }
}

/// Position Reconciliation System
///
/// Manages position cache and synchronization with broker
pub struct PositionReconciliationSystem {
    /// Local position cache: HashMap<position_id, CachedPosition>
    cache: Arc<RwLock<HashMap<String, CachedPosition>>>,
    /// Connection state
    connection_state: Arc<RwLock<ConnectionState>>,
    /// Last successful connection time
    last_connected: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Last disconnection time
    last_disconnected: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Last reconciliation time
    last_reconciliation: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Audit trail
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
    /// Configuration
    config: ReconciliationConfig,
    /// Pending re-sync flag
    pending_resync: Arc<RwLock<bool>>,
}

impl PositionReconciliationSystem {
    /// Create a new reconciliation system with default config
    pub fn new() -> Self {
        Self::with_config(ReconciliationConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: ReconciliationConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            last_connected: Arc::new(RwLock::new(None)),
            last_disconnected: Arc::new(RwLock::new(None)),
            last_reconciliation: Arc::new(RwLock::new(None)),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            config,
            pending_resync: Arc::new(RwLock::new(false)),
        }
    }

    /// Log an audit event
    async fn log_audit(&self, event: AuditEventType) {
        let entry = AuditEntry::new(event.clone());

        // Log to tracing
        match &event {
            AuditEventType::ConnectionStateChanged { from, to } => {
                info!(
                    "[AUDIT] Connection state: {} -> {} at {}",
                    from, to, entry.timestamp
                );
            }
            AuditEventType::ReconciliationStarted { local_count, broker_count } => {
                info!(
                    "[AUDIT] Reconciliation started: {} local, {} broker at {}",
                    local_count, broker_count, entry.timestamp
                );
            }
            AuditEventType::ReconciliationCompleted { synced, orphaned, missing, mismatched, duration_ms } => {
                info!(
                    "[AUDIT] Reconciliation completed: synced={}, orphaned={}, missing={}, mismatched={}, duration={}ms at {}",
                    synced, orphaned, missing, mismatched, duration_ms, entry.timestamp
                );
            }
            AuditEventType::PositionAddedFromBroker { position_id, symbol, side, entry_price } => {
                info!(
                    "[AUDIT] Position added from broker: {} {} {} @ {:.2} at {}",
                    position_id, symbol, side, entry_price, entry.timestamp
                );
            }
            AuditEventType::PositionRemoved { position_id, reason } => {
                warn!(
                    "[AUDIT] Position removed: {} - {} at {}",
                    position_id, reason, entry.timestamp
                );
            }
            AuditEventType::PositionUpdated { position_id, field, old_value, new_value } => {
                debug!(
                    "[AUDIT] Position updated: {} {} changed from {} to {} at {}",
                    position_id, field, old_value, new_value, entry.timestamp
                );
            }
            AuditEventType::MismatchDetected { position_id, field, local_value, broker_value } => {
                warn!(
                    "[AUDIT] Mismatch detected: {} {} local={} broker={} at {}",
                    position_id, field, local_value, broker_value, entry.timestamp
                );
            }
            AuditEventType::ResyncTriggered { reason } => {
                info!("[AUDIT] Re-sync triggered: {} at {}", reason, entry.timestamp);
            }
            AuditEventType::CacheCleared { count } => {
                info!("[AUDIT] Cache cleared: {} positions at {}", count, entry.timestamp);
            }
        }

        // Add to audit log
        let mut log = self.audit_log.write().await;
        log.push(entry);

        // Trim if exceeds max entries
        if log.len() > self.config.max_audit_entries {
            let excess = log.len() - self.config.max_audit_entries;
            log.drain(0..excess);
        }
    }

    /// Update connection state
    pub async fn set_connection_state(&self, new_state: ConnectionState) {
        let mut state = self.connection_state.write().await;
        let old_state = *state;

        if old_state == new_state {
            return;
        }

        *state = new_state;
        drop(state);

        // Log state change
        self.log_audit(AuditEventType::ConnectionStateChanged {
            from: old_state,
            to: new_state,
        }).await;

        // Handle state transitions
        match new_state {
            ConnectionState::Connected => {
                *self.last_connected.write().await = Some(Utc::now());

                // Trigger re-sync if we were disconnected
                if old_state == ConnectionState::Disconnected
                    || old_state == ConnectionState::Reconnecting
                {
                    *self.pending_resync.write().await = true;
                    self.log_audit(AuditEventType::ResyncTriggered {
                        reason: format!("Connection restored from {}", old_state),
                    }).await;
                }
            }
            ConnectionState::Disconnected => {
                *self.last_disconnected.write().await = Some(Utc::now());
            }
            _ => {}
        }
    }

    /// Get current connection state
    pub async fn get_connection_state(&self) -> ConnectionState {
        *self.connection_state.read().await
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connection_state.read().await == ConnectionState::Connected
    }

    /// Check if re-sync is pending
    pub async fn is_resync_pending(&self) -> bool {
        *self.pending_resync.read().await
    }

    /// Add or update a position in cache
    pub async fn cache_position(&self, position: Position) {
        let id = position.id.clone();
        let mut cache = self.cache.write().await;

        if let Some(existing) = cache.get_mut(&id) {
            // Update existing
            let old_price = existing.position.entry_price;
            existing.position = position.clone();
            existing.mark_synced();
            let new_price = existing.position.entry_price;

            if (old_price - new_price).abs() > 0.01 {
                drop(cache);
                self.log_audit(AuditEventType::PositionUpdated {
                    position_id: id,
                    field: "entry_price".to_string(),
                    old_value: format!("{:.2}", old_price),
                    new_value: format!("{:.2}", new_price),
                }).await;
            }
        } else {
            // New position
            cache.insert(id.clone(), CachedPosition::new(position.clone()));
            drop(cache);
            self.log_audit(AuditEventType::PositionAddedFromBroker {
                position_id: id,
                symbol: position.symbol,
                side: position.side,
                entry_price: position.entry_price,
            }).await;
        }
    }

    /// Get a position from cache
    pub async fn get_position(&self, position_id: &str) -> Option<Position> {
        self.cache
            .read()
            .await
            .get(position_id)
            .map(|cp| cp.position.clone())
    }

    /// Get all cached positions
    pub async fn get_all_positions(&self) -> Vec<Position> {
        self.cache
            .read()
            .await
            .values()
            .map(|cp| cp.position.clone())
            .collect()
    }

    /// Get position count
    pub async fn position_count(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Remove a position from cache
    pub async fn remove_position(&self, position_id: &str, reason: &str) -> Option<Position> {
        let mut cache = self.cache.write().await;
        let removed = cache.remove(position_id);

        if let Some(ref cp) = removed {
            drop(cache);
            self.log_audit(AuditEventType::PositionRemoved {
                position_id: position_id.to_string(),
                reason: reason.to_string(),
            }).await;
            return Some(cp.position.clone());
        }

        None
    }

    /// Clear entire cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        let count = cache.len();
        cache.clear();
        drop(cache);

        self.log_audit(AuditEventType::CacheCleared { count }).await;
    }

    /// Reconcile local cache with broker positions
    pub async fn reconcile(&self, broker_positions: Vec<BrokerPositionData>) -> Result<ReconciliationReport> {
        let start_time = Utc::now();
        let mut report = ReconciliationReport::new();

        // Check minimum interval
        if let Some(last) = *self.last_reconciliation.read().await {
            let elapsed = (Utc::now() - last).num_seconds() as u64;
            if elapsed < self.config.min_reconciliation_interval_secs {
                return Err(BotError::Trading(format!(
                    "Reconciliation too frequent: {}s since last (min: {}s)",
                    elapsed, self.config.min_reconciliation_interval_secs
                )));
            }
        }

        let cache = self.cache.write().await;
        let local_count = cache.len();
        let broker_count = broker_positions.len();

        drop(cache);
        self.log_audit(AuditEventType::ReconciliationStarted {
            local_count,
            broker_count,
        }).await;

        let mut cache = self.cache.write().await;

        // Build broker position lookup
        let broker_map: HashMap<i64, &BrokerPositionData> = broker_positions
            .iter()
            .map(|bp| (bp.position_id, bp))
            .collect();

        // Check local positions against broker
        let local_ids: Vec<String> = cache.keys().cloned().collect();

        for local_id in &local_ids {
            let broker_id: Option<i64> = local_id.parse().ok();

            match broker_id {
                Some(bid) => {
                    if let Some(broker_pos) = broker_map.get(&bid) {
                        // Position exists on both sides
                        if let Some(cached) = cache.get_mut(local_id) {
                            // Check for mismatches
                            let local_entry = cached.position.entry_price;
                            let broker_entry = broker_pos.entry_price;

                            if (local_entry - broker_entry).abs() > 0.01 {
                                report.mismatches.push(ReconciliationMismatch {
                                    position_id: local_id.clone(),
                                    field: "entry_price".to_string(),
                                    local_value: format!("{:.2}", local_entry),
                                    broker_value: format!("{:.2}", broker_entry),
                                });
                            }

                            let local_volume = cached.position.volume;
                            let broker_volume = broker_pos.volume;

                            if (local_volume - broker_volume).abs() > 0.001 {
                                report.mismatches.push(ReconciliationMismatch {
                                    position_id: local_id.clone(),
                                    field: "volume".to_string(),
                                    local_value: format!("{:.3}", local_volume),
                                    broker_value: format!("{:.3}", broker_volume),
                                });
                            }

                            // Update P&L from broker
                            cached.position.current_pnl = broker_pos.current_pnl;
                            cached.mark_synced();
                            report.synced.push(local_id.clone());
                        }
                    } else {
                        // Local position not found on broker - orphaned
                        report.orphaned.push(local_id.clone());
                    }
                }
                None => {
                    // Non-numeric ID (dry-run position) - skip
                    debug!("Skipping non-broker position: {}", local_id);
                }
            }
        }

        // Check for broker positions missing locally
        let local_broker_ids: std::collections::HashSet<i64> = cache
            .keys()
            .filter_map(|id| id.parse().ok())
            .collect();

        for broker_pos in &broker_positions {
            if !local_broker_ids.contains(&broker_pos.position_id) {
                report.missing.push(broker_pos.position_id);

                if self.config.auto_add_missing {
                    let new_pos = Position::new(
                        broker_pos.position_id.to_string(),
                        broker_pos.symbol.clone(),
                        broker_pos.side,
                        broker_pos.entry_price,
                        broker_pos.volume,
                    );
                    cache.insert(new_pos.id.clone(), CachedPosition::new(new_pos));
                }
            }
        }

        // Remove orphaned positions if configured
        if self.config.auto_remove_orphaned {
            for orphan_id in &report.orphaned {
                cache.remove(orphan_id);
            }
        }

        drop(cache);

        // Update last reconciliation time
        *self.last_reconciliation.write().await = Some(Utc::now());
        *self.pending_resync.write().await = false;

        // Calculate duration
        let duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;
        report.duration_ms = duration_ms;
        report.timestamp = start_time;

        // Log completion
        self.log_audit(AuditEventType::ReconciliationCompleted {
            synced: report.synced.len(),
            orphaned: report.orphaned.len(),
            missing: report.missing.len(),
            mismatched: report.mismatches.len(),
            duration_ms,
        }).await;

        // Log individual mismatches
        for mismatch in &report.mismatches {
            self.log_audit(AuditEventType::MismatchDetected {
                position_id: mismatch.position_id.clone(),
                field: mismatch.field.clone(),
                local_value: mismatch.local_value.clone(),
                broker_value: mismatch.broker_value.clone(),
            }).await;
        }

        Ok(report)
    }

    /// Trigger manual re-sync
    pub async fn trigger_resync(&self, reason: &str) {
        *self.pending_resync.write().await = true;
        self.log_audit(AuditEventType::ResyncTriggered {
            reason: reason.to_string(),
        }).await;
    }

    /// Get audit log entries
    pub async fn get_audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log.read().await.clone()
    }

    /// Get audit log entries since a timestamp
    pub async fn get_audit_log_since(&self, since: DateTime<Utc>) -> Vec<AuditEntry> {
        self.audit_log
            .read()
            .await
            .iter()
            .filter(|e| e.timestamp >= since)
            .cloned()
            .collect()
    }

    /// Get stale positions
    pub async fn get_stale_positions(&self) -> Vec<String> {
        let max_age = Duration::seconds(self.config.max_stale_duration_secs as i64);
        self.cache
            .read()
            .await
            .iter()
            .filter(|(_, cp)| cp.is_stale(max_age))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get connection uptime
    pub async fn get_connection_uptime(&self) -> Option<Duration> {
        let state = *self.connection_state.read().await;
        if state == ConnectionState::Connected {
            self.last_connected.read().await.map(|t| Utc::now() - t)
        } else {
            None
        }
    }

    /// Get time since last disconnect
    pub async fn get_time_since_disconnect(&self) -> Option<Duration> {
        self.last_disconnected.read().await.map(|t| Utc::now() - t)
    }

    /// Export cache state for debugging
    pub async fn export_state(&self) -> ReconciliationState {
        ReconciliationState {
            connection_state: *self.connection_state.read().await,
            last_connected: *self.last_connected.read().await,
            last_disconnected: *self.last_disconnected.read().await,
            last_reconciliation: *self.last_reconciliation.read().await,
            pending_resync: *self.pending_resync.read().await,
            position_count: self.cache.read().await.len(),
            audit_log_count: self.audit_log.read().await.len(),
        }
    }
}

impl Default for PositionReconciliationSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Reconciliation mismatch detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationMismatch {
    pub position_id: String,
    pub field: String,
    pub local_value: String,
    pub broker_value: String,
}

/// Reconciliation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    /// Timestamp when reconciliation started
    pub timestamp: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Positions successfully synced
    pub synced: Vec<String>,
    /// Orphaned local positions (not on broker)
    pub orphaned: Vec<String>,
    /// Missing local positions (on broker but not cached)
    pub missing: Vec<i64>,
    /// Mismatches detected
    pub mismatches: Vec<ReconciliationMismatch>,
}

impl ReconciliationReport {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            duration_ms: 0,
            synced: Vec::new(),
            orphaned: Vec::new(),
            missing: Vec::new(),
            mismatches: Vec::new(),
        }
    }

    pub fn is_clean(&self) -> bool {
        self.orphaned.is_empty() && self.missing.is_empty() && self.mismatches.is_empty()
    }

    pub fn total_issues(&self) -> usize {
        self.orphaned.len() + self.missing.len() + self.mismatches.len()
    }
}

impl Default for ReconciliationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Exported reconciliation state for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationState {
    pub connection_state: ConnectionState,
    pub last_connected: Option<DateTime<Utc>>,
    pub last_disconnected: Option<DateTime<Utc>>,
    pub last_reconciliation: Option<DateTime<Utc>>,
    pub pending_resync: bool,
    pub position_count: usize,
    pub audit_log_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn test_cache_position() {
        let system = PositionReconciliationSystem::new();

        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        system.cache_position(pos).await;

        assert_eq!(system.position_count().await, 1);

        let retrieved = system.get_position("123").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().entry_price, 4850.0);
    }

    #[tokio::test]
    async fn test_remove_position() {
        let system = PositionReconciliationSystem::new();

        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        system.cache_position(pos).await;

        let removed = system.remove_position("123", "test removal").await;
        assert!(removed.is_some());
        assert_eq!(system.position_count().await, 0);
    }

    #[tokio::test]
    async fn test_connection_state_transitions() {
        let system = PositionReconciliationSystem::new();

        assert_eq!(system.get_connection_state().await, ConnectionState::Disconnected);
        assert!(!system.is_connected().await);

        system.set_connection_state(ConnectionState::Connected).await;
        assert!(system.is_connected().await);

        system.set_connection_state(ConnectionState::Disconnected).await;
        assert!(!system.is_connected().await);
    }

    #[tokio::test]
    async fn test_resync_triggered_on_reconnect() {
        let system = PositionReconciliationSystem::new();

        // Simulate connection loss and recovery
        system.set_connection_state(ConnectionState::Connected).await;
        system.set_connection_state(ConnectionState::Disconnected).await;
        system.set_connection_state(ConnectionState::Connected).await;

        assert!(system.is_resync_pending().await);
    }

    #[tokio::test]
    async fn test_reconcile_clean() {
        let config = ReconciliationConfig {
            min_reconciliation_interval_secs: 0, // Disable throttling for test
            ..Default::default()
        };
        let system = PositionReconciliationSystem::with_config(config);

        // Add local position
        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        system.cache_position(pos).await;

        // Create matching broker position
        let broker_positions = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4850.0)];

        let report = system.reconcile(broker_positions).await.unwrap();

        assert!(report.is_clean());
        assert_eq!(report.synced.len(), 1);
    }

    #[tokio::test]
    async fn test_reconcile_orphaned() {
        let config = ReconciliationConfig {
            min_reconciliation_interval_secs: 0,
            auto_remove_orphaned: true,
            ..Default::default()
        };
        let system = PositionReconciliationSystem::with_config(config);

        // Add local position with no broker match
        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        system.cache_position(pos).await;

        // Empty broker positions
        let broker_positions = vec![];

        let report = system.reconcile(broker_positions).await.unwrap();

        assert_eq!(report.orphaned.len(), 1);
        assert_eq!(system.position_count().await, 0); // Removed
    }

    #[tokio::test]
    async fn test_reconcile_missing() {
        let config = ReconciliationConfig {
            min_reconciliation_interval_secs: 0,
            auto_add_missing: true,
            ..Default::default()
        };
        let system = PositionReconciliationSystem::with_config(config);

        // Broker has position we don't have
        let broker_positions = vec![create_broker_position(456, "FCPO", OrderSide::Sell, 4900.0)];

        let report = system.reconcile(broker_positions).await.unwrap();

        assert_eq!(report.missing.len(), 1);
        assert_eq!(system.position_count().await, 1); // Added

        let pos = system.get_position("456").await.unwrap();
        assert_eq!(pos.side, OrderSide::Sell);
    }

    #[tokio::test]
    async fn test_reconcile_mismatch() {
        let config = ReconciliationConfig {
            min_reconciliation_interval_secs: 0,
            ..Default::default()
        };
        let system = PositionReconciliationSystem::with_config(config);

        // Local position with different entry price
        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        system.cache_position(pos).await;

        // Broker position with different entry price
        let broker_positions = vec![create_broker_position(123, "FCPO", OrderSide::Buy, 4860.0)];

        let report = system.reconcile(broker_positions).await.unwrap();

        assert!(!report.is_clean());
        assert_eq!(report.mismatches.len(), 1);
        assert!(report.mismatches[0].field == "entry_price");
    }

    #[tokio::test]
    async fn test_audit_log() {
        let system = PositionReconciliationSystem::new();

        system.set_connection_state(ConnectionState::Connected).await;
        system.set_connection_state(ConnectionState::Disconnected).await;

        let log = system.get_audit_log().await;
        assert!(log.len() >= 2);

        // Check that connection state changes are logged
        let connection_events: Vec<_> = log.iter()
            .filter(|e| matches!(e.event, AuditEventType::ConnectionStateChanged { .. }))
            .collect();
        assert!(connection_events.len() >= 2);
    }

    #[tokio::test]
    async fn test_get_all_positions() {
        let system = PositionReconciliationSystem::new();

        system.cache_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0)).await;
        system.cache_position(create_test_position("2", "FCPO", OrderSide::Sell, 4860.0)).await;

        let positions = system.get_all_positions().await;
        assert_eq!(positions.len(), 2);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let system = PositionReconciliationSystem::new();

        system.cache_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0)).await;
        system.cache_position(create_test_position("2", "FCPO", OrderSide::Sell, 4860.0)).await;

        system.clear_cache().await;

        assert_eq!(system.position_count().await, 0);
    }

    #[tokio::test]
    async fn test_export_state() {
        let system = PositionReconciliationSystem::new();

        system.set_connection_state(ConnectionState::Connected).await;
        system.cache_position(create_test_position("1", "FCPO", OrderSide::Buy, 4850.0)).await;

        let state = system.export_state().await;

        assert_eq!(state.connection_state, ConnectionState::Connected);
        assert_eq!(state.position_count, 1);
        assert!(state.last_connected.is_some());
    }

    #[tokio::test]
    async fn test_trigger_resync() {
        let system = PositionReconciliationSystem::new();

        assert!(!system.is_resync_pending().await);

        system.trigger_resync("manual test").await;

        assert!(system.is_resync_pending().await);
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

    #[tokio::test]
    async fn test_stale_position_detection() {
        let config = ReconciliationConfig {
            max_stale_duration_secs: 0, // Immediate staleness for test
            min_reconciliation_interval_secs: 0,
            ..Default::default()
        };
        let system = PositionReconciliationSystem::with_config(config);

        system.cache_position(create_test_position("123", "FCPO", OrderSide::Buy, 4850.0)).await;

        // Wait a tiny bit
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let stale = system.get_stale_positions().await;
        assert_eq!(stale.len(), 1);
    }
}
