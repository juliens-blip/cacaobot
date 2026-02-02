//! SQLite persistence layer for positions and trades
//!
//! Provides durable storage with ACID guarantees for:
//! - Open positions
//! - Closed trades (audit trail)
//! - Daily statistics
//!
//! Complements JSON persistence with stronger consistency.

use crate::error::{BotError, Result};
use crate::modules::trading::{CloseReason, OrderSide, Position};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// SQLite database for positions
pub struct PositionDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl PositionDatabase {
    /// Create a new database connection and initialize schema
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(db_path.as_ref()).map_err(|e| {
            BotError::Config(format!("Failed to open SQLite database: {}", e))
        })?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.init_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Positions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS positions (
                id TEXT PRIMARY KEY,
                broker_id INTEGER,
                symbol TEXT NOT NULL,
                side TEXT NOT NULL,
                entry_price REAL NOT NULL,
                volume REAL NOT NULL,
                take_profit REAL,
                stop_loss REAL,
                opened_at TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'open'
            )",
            [],
        )
        .map_err(|e| BotError::Config(format!("Failed to create positions table: {}", e)))?;

        // Closed trades table (audit trail)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS closed_trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                position_id TEXT NOT NULL,
                broker_id INTEGER,
                symbol TEXT NOT NULL,
                side TEXT NOT NULL,
                entry_price REAL NOT NULL,
                exit_price REAL NOT NULL,
                volume REAL NOT NULL,
                realized_pnl REAL NOT NULL,
                opened_at TEXT NOT NULL,
                closed_at TEXT NOT NULL,
                close_reason TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| BotError::Config(format!("Failed to create closed_trades table: {}", e)))?;

        // Daily stats table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS daily_stats (
                date TEXT PRIMARY KEY,
                total_pnl REAL NOT NULL,
                total_trades INTEGER NOT NULL,
                winning_trades INTEGER NOT NULL,
                losing_trades INTEGER NOT NULL,
                largest_win REAL NOT NULL,
                largest_loss REAL NOT NULL
            )",
            [],
        )
        .map_err(|e| BotError::Config(format!("Failed to create daily_stats table: {}", e)))?;

        // Indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_positions_status ON positions(status)",
            [],
        )
        .ok();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_closed_trades_date ON closed_trades(closed_at)",
            [],
        )
        .ok();

        info!("SQLite database schema initialized");
        Ok(())
    }

    /// Insert or update a position
    pub fn upsert_position(&self, position: &Position) -> Result<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let broker_id = position.id.parse::<i64>().ok();
        let side_str = format!("{:?}", position.side);
        let opened_at = position.opened_at.to_rfc3339();
        let updated_at = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO positions 
             (id, broker_id, symbol, side, entry_price, volume, take_profit, stop_loss, opened_at, last_updated, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'open')",
            params![
                &position.id,
                broker_id,
                &position.symbol,
                side_str,
                position.entry_price,
                position.volume,
                position.take_profit,
                position.stop_loss,
                opened_at,
                updated_at,
            ],
        )
        .map_err(|e| BotError::Config(format!("Failed to upsert position: {}", e)))?;

        debug!("Position {} upserted to SQLite", position.id);
        Ok(())
    }

    /// Get a position by ID
    pub fn get_position(&self, id: &str) -> Result<Option<Position>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let result = conn
            .query_row(
                "SELECT id, symbol, side, entry_price, volume, take_profit, stop_loss, opened_at
                 FROM positions
                 WHERE id = ?1 AND status = 'open'",
                params![id],
                |row| {
                    let id: String = row.get(0)?;
                    let symbol: String = row.get(1)?;
                    let side_str: String = row.get(2)?;
                    let entry_price: f64 = row.get(3)?;
                    let volume: f64 = row.get(4)?;
                    let take_profit: Option<f64> = row.get(5)?;
                    let stop_loss: Option<f64> = row.get(6)?;

                    let side = match side_str.as_str() {
                        "Buy" => OrderSide::Buy,
                        "Sell" => OrderSide::Sell,
                        _ => OrderSide::Buy,
                    };

                    let mut pos = Position::new(id, symbol, side, entry_price, volume);
                    if let Some(tp) = take_profit {
                        pos = pos.with_take_profit(tp);
                    }
                    if let Some(sl) = stop_loss {
                        pos = pos.with_stop_loss(sl);
                    }

                    Ok(pos)
                },
            )
            .optional()
            .map_err(|e| BotError::Config(format!("Failed to get position: {}", e)))?;

        Ok(result)
    }

    /// Get all open positions
    pub fn get_open_positions(&self) -> Result<Vec<Position>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let mut stmt = conn
            .prepare(
                "SELECT id, symbol, side, entry_price, volume, take_profit, stop_loss, opened_at
                 FROM positions
                 WHERE status = 'open'
                 ORDER BY opened_at DESC",
            )
            .map_err(|e| BotError::Config(format!("Failed to prepare statement: {}", e)))?;

        let positions = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let symbol: String = row.get(1)?;
                let side_str: String = row.get(2)?;
                let entry_price: f64 = row.get(3)?;
                let volume: f64 = row.get(4)?;
                let take_profit: Option<f64> = row.get(5)?;
                let stop_loss: Option<f64> = row.get(6)?;

                let side = match side_str.as_str() {
                    "Buy" => OrderSide::Buy,
                    "Sell" => OrderSide::Sell,
                    _ => OrderSide::Buy,
                };

                let mut pos = Position::new(id, symbol, side, entry_price, volume);
                if let Some(tp) = take_profit {
                    pos = pos.with_take_profit(tp);
                }
                if let Some(sl) = stop_loss {
                    pos = pos.with_stop_loss(sl);
                }

                Ok(pos)
            })
            .map_err(|e| BotError::Config(format!("Failed to query positions: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| BotError::Config(format!("Failed to collect positions: {}", e)))?;

        Ok(positions)
    }

    /// Close a position and record trade
    pub fn close_position(
        &self,
        position_id: &str,
        exit_price: f64,
        close_reason: CloseReason,
    ) -> Result<f64> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Get position data
        let (symbol, side_str, entry_price, volume, opened_at, broker_id): (
            String,
            String,
            f64,
            f64,
            String,
            Option<i64>,
        ) = conn
            .query_row(
                "SELECT symbol, side, entry_price, volume, opened_at, broker_id
                 FROM positions
                 WHERE id = ?1 AND status = 'open'",
                params![position_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .map_err(|e| BotError::Config(format!("Failed to get position for closing: {}", e)))?;

        // Calculate P&L
        let side = match side_str.as_str() {
            "Buy" => OrderSide::Buy,
            "Sell" => OrderSide::Sell,
            _ => OrderSide::Buy,
        };

        let pnl = match side {
            OrderSide::Buy => (exit_price - entry_price) * volume,
            OrderSide::Sell => (entry_price - exit_price) * volume,
        };

        // Mark position as closed
        conn.execute(
            "UPDATE positions SET status = 'closed', last_updated = ?1
             WHERE id = ?2",
            params![Utc::now().to_rfc3339(), position_id],
        )
        .map_err(|e| BotError::Config(format!("Failed to update position status: {}", e)))?;

        // Insert into closed_trades
        conn.execute(
            "INSERT INTO closed_trades 
             (position_id, broker_id, symbol, side, entry_price, exit_price, volume, realized_pnl, opened_at, closed_at, close_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                position_id,
                broker_id,
                symbol,
                side_str,
                entry_price,
                exit_price,
                volume,
                pnl,
                opened_at,
                Utc::now().to_rfc3339(),
                format!("{:?}", close_reason),
            ],
        )
        .map_err(|e| BotError::Config(format!("Failed to insert closed trade: {}", e)))?;

        debug!(
            "Position {} closed in SQLite with P&L: {:.2}",
            position_id, pnl
        );
        Ok(pnl)
    }

    /// Delete a position (for reconciliation cleanup)
    pub fn delete_position(&self, position_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        conn.execute(
            "DELETE FROM positions WHERE id = ?1",
            params![position_id],
        )
        .map_err(|e| BotError::Config(format!("Failed to delete position: {}", e)))?;

        debug!("Position {} deleted from SQLite", position_id);
        Ok(())
    }

    /// Get count of open positions
    pub fn count_open_positions(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM positions WHERE status = 'open'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| BotError::Config(format!("Failed to count positions: {}", e)))?;

        Ok(count as usize)
    }

    /// Get today's closed trades
    pub fn get_today_trades(&self) -> Result<Vec<(f64, DateTime<Utc>)>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let today = Utc::now().date_naive().to_string();

        let mut stmt = conn
            .prepare(
                "SELECT realized_pnl, closed_at
                 FROM closed_trades
                 WHERE DATE(closed_at) = DATE(?1)
                 ORDER BY closed_at",
            )
            .map_err(|e| BotError::Config(format!("Failed to prepare statement: {}", e)))?;

        let trades = stmt
            .query_map(params![today], |row| {
                let pnl: f64 = row.get(0)?;
                let closed_at_str: String = row.get(1)?;
                let closed_at = DateTime::parse_from_rfc3339(&closed_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok((pnl, closed_at))
            })
            .map_err(|e| BotError::Config(format!("Failed to query trades: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| BotError::Config(format!("Failed to collect trades: {}", e)))?;

        Ok(trades)
    }

    /// Update daily statistics
    pub fn update_daily_stats(&self, date: &str, pnl: f64, is_win: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Get existing stats or create new
        let existing = conn
            .query_row(
                "SELECT total_pnl, total_trades, winning_trades, losing_trades, largest_win, largest_loss
                 FROM daily_stats
                 WHERE date = ?1",
                params![date],
                |row| {
                    Ok((
                        row.get::<_, f64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, f64>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| BotError::Config(format!("Failed to get daily stats: {}", e)))?;

        let (total_pnl, total_trades, winning, losing, largest_win, largest_loss) = match existing {
            Some((tp, tt, w, l, lw, ll)) => {
                let new_pnl = tp + pnl;
                let new_total = tt + 1;
                let (new_win, new_lose) = if is_win { (w + 1, l) } else { (w, l + 1) };
                let new_largest_win = if is_win { lw.max(pnl) } else { lw };
                let new_largest_loss = if !is_win { ll.min(pnl) } else { ll };
                (
                    new_pnl,
                    new_total,
                    new_win,
                    new_lose,
                    new_largest_win,
                    new_largest_loss,
                )
            }
            None => {
                let (win, lose) = if is_win { (1, 0) } else { (0, 1) };
                let largest_win = if is_win { pnl } else { 0.0 };
                let largest_loss = if !is_win { pnl } else { 0.0 };
                (pnl, 1, win, lose, largest_win, largest_loss)
            }
        };

        conn.execute(
            "INSERT OR REPLACE INTO daily_stats
             (date, total_pnl, total_trades, winning_trades, losing_trades, largest_win, largest_loss)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                date,
                total_pnl,
                total_trades,
                winning,
                losing,
                largest_win,
                largest_loss
            ],
        )
        .map_err(|e| BotError::Config(format!("Failed to update daily stats: {}", e)))?;

        Ok(())
    }

    /// Get statistics for a date
    pub fn get_daily_stats(&self, date: &str) -> Result<Option<DailyStats>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let result = conn
            .query_row(
                "SELECT total_pnl, total_trades, winning_trades, losing_trades, largest_win, largest_loss
                 FROM daily_stats
                 WHERE date = ?1",
                params![date],
                |row| {
                    Ok(DailyStats {
                        date: date.to_string(),
                        total_pnl: row.get(0)?,
                        total_trades: row.get(1)?,
                        winning_trades: row.get(2)?,
                        losing_trades: row.get(3)?,
                        largest_win: row.get(4)?,
                        largest_loss: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(|e| BotError::Config(format!("Failed to get daily stats: {}", e)))?;

        Ok(result)
    }

    /// Fetch all closed trades (for audit/export)
    pub fn get_closed_trades(&self) -> Result<Vec<ClosedTradeRecord>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let mut stmt = conn
            .prepare(
                "SELECT position_id, broker_id, symbol, side, entry_price, exit_price, volume, realized_pnl, opened_at, closed_at, close_reason
                 FROM closed_trades
                 ORDER BY closed_at",
            )
            .map_err(|e| BotError::Config(format!("Failed to prepare closed trades: {}", e)))?;

        let records = stmt
            .query_map([], |row| {
                Ok(ClosedTradeRecord {
                    position_id: row.get(0)?,
                    broker_id: row.get(1)?,
                    symbol: row.get(2)?,
                    side: row.get(3)?,
                    entry_price: row.get(4)?,
                    exit_price: row.get(5)?,
                    volume: row.get(6)?,
                    realized_pnl: row.get(7)?,
                    opened_at: row.get(8)?,
                    closed_at: row.get(9)?,
                    close_reason: row.get(10)?,
                })
            })
            .map_err(|e| BotError::Config(format!("Failed to query closed trades: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| BotError::Config(format!("Failed to collect closed trades: {}", e)))?;

        Ok(records)
    }

    /// Export closed trades to CSV file
    pub fn export_closed_trades_csv(&self, path: impl AsRef<Path>) -> Result<()> {
        let records = self.get_closed_trades()?;
        let mut file = File::create(path.as_ref())
            .map_err(|e| BotError::Config(format!("Failed to create export file: {}", e)))?;

        writeln!(
            file,
            "position_id,broker_id,symbol,side,entry_price,exit_price,volume,realized_pnl,opened_at,closed_at,close_reason"
        )
        .map_err(|e| BotError::Config(format!("Failed to write CSV header: {}", e)))?;

        for record in records {
            writeln!(
                file,
                "{},{},{},{},{:.5},{:.5},{:.4},{:.4},{},{},{}",
                record.position_id,
                record
                    .broker_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "".to_string()),
                record.symbol,
                record.side,
                record.entry_price,
                record.exit_price,
                record.volume,
                record.realized_pnl,
                record.opened_at,
                record.closed_at,
                record.close_reason
            )
            .map_err(|e| BotError::Config(format!("Failed to write CSV row: {}", e)))?;
        }

        Ok(())
    }

    /// Export closed trades to JSON file
    pub fn export_closed_trades_json(&self, path: impl AsRef<Path>) -> Result<()> {
        let records = self.get_closed_trades()?;
        let payload = serde_json::to_string_pretty(&records)
            .map_err(|e| BotError::Config(format!("Failed to serialize JSON: {}", e)))?;
        std::fs::write(path.as_ref(), payload)
            .map_err(|e| BotError::Config(format!("Failed to write JSON file: {}", e)))?;
        Ok(())
    }

    /// Export daily stats to CSV file
    pub fn export_daily_stats_csv(&self, path: impl AsRef<Path>) -> Result<()> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT date, total_pnl, total_trades, winning_trades, losing_trades, largest_win, largest_loss
                 FROM daily_stats
                 ORDER BY date",
            )
            .map_err(|e| BotError::Config(format!("Failed to prepare daily stats: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(DailyStats {
                    date: row.get(0)?,
                    total_pnl: row.get(1)?,
                    total_trades: row.get(2)?,
                    winning_trades: row.get(3)?,
                    losing_trades: row.get(4)?,
                    largest_win: row.get(5)?,
                    largest_loss: row.get(6)?,
                })
            })
            .map_err(|e| BotError::Config(format!("Failed to query daily stats: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| BotError::Config(format!("Failed to collect daily stats: {}", e)))?;

        let mut file = File::create(path.as_ref())
            .map_err(|e| BotError::Config(format!("Failed to create export file: {}", e)))?;
        writeln!(
            file,
            "date,total_pnl,total_trades,winning_trades,losing_trades,largest_win,largest_loss"
        )
        .map_err(|e| BotError::Config(format!("Failed to write CSV header: {}", e)))?;

        for row in rows {
            writeln!(
                file,
                "{},{:.4},{},{},{},{:.4},{:.4}",
                row.date,
                row.total_pnl,
                row.total_trades,
                row.winning_trades,
                row.losing_trades,
                row.largest_win,
                row.largest_loss
            )
            .map_err(|e| BotError::Config(format!("Failed to write CSV row: {}", e)))?;
        }

        Ok(())
    }
}

/// Daily statistics record
#[derive(Debug, Clone)]
pub struct DailyStats {
    pub date: String,
    pub total_pnl: f64,
    pub total_trades: i64,
    pub winning_trades: i64,
    pub losing_trades: i64,
    pub largest_win: f64,
    pub largest_loss: f64,
}

impl DailyStats {
    pub fn win_rate(&self) -> f64 {
        if self.total_trades == 0 {
            0.0
        } else {
            (self.winning_trades as f64 / self.total_trades as f64) * 100.0
        }
    }
}

/// Closed trade record for export
#[derive(Debug, Clone, Serialize)]
pub struct ClosedTradeRecord {
    pub position_id: String,
    pub broker_id: Option<i64>,
    pub symbol: String,
    pub side: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub volume: f64,
    pub realized_pnl: f64,
    pub opened_at: String,
    pub closed_at: String,
    pub close_reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};

    fn create_test_db() -> (PositionDatabase, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = PositionDatabase::new(&db_path).unwrap();
        (db, temp_dir)
    }

    fn create_test_position(id: &str, symbol: &str, side: OrderSide, entry: f64) -> Position {
        Position::new(id.to_string(), symbol.to_string(), side, entry, 1.0)
    }

    #[test]
    fn test_init_schema() {
        let (db, _dir) = create_test_db();
        assert_eq!(db.count_open_positions().unwrap(), 0);
    }

    #[test]
    fn test_upsert_and_get_position() {
        let (db, _dir) = create_test_db();

        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        db.upsert_position(&pos).unwrap();

        let retrieved = db.get_position("123").unwrap().unwrap();
        assert_eq!(retrieved.id, "123");
        assert_eq!(retrieved.entry_price, 4850.0);
        assert_eq!(retrieved.side, OrderSide::Buy);
    }

    #[test]
    fn test_get_open_positions() {
        let (db, _dir) = create_test_db();

        db.upsert_position(&create_test_position("1", "FCPO", OrderSide::Buy, 4850.0))
            .unwrap();
        db.upsert_position(&create_test_position("2", "FCPO", OrderSide::Sell, 4900.0))
            .unwrap();

        let positions = db.get_open_positions().unwrap();
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_close_position() {
        let (db, _dir) = create_test_db();

        let pos = create_test_position("123", "FCPO", OrderSide::Buy, 4850.0);
        db.upsert_position(&pos).unwrap();

        let pnl = db
            .close_position("123", 4900.0, CloseReason::TakeProfit)
            .unwrap();

        assert!((pnl - 50.0).abs() < 0.01);
        assert_eq!(db.count_open_positions().unwrap(), 0);
        assert!(db.get_position("123").unwrap().is_none());
    }

    #[test]
    fn test_delete_position() {
        let (db, _dir) = create_test_db();

        db.upsert_position(&create_test_position("123", "FCPO", OrderSide::Buy, 4850.0))
            .unwrap();

        db.delete_position("123").unwrap();
        assert_eq!(db.count_open_positions().unwrap(), 0);
    }

    #[test]
    fn test_daily_stats() {
        let (db, _dir) = create_test_db();

        let today = Utc::now().date_naive().to_string();

        db.update_daily_stats(&today, 50.0, true).unwrap();
        db.update_daily_stats(&today, -30.0, false).unwrap();

        let stats = db.get_daily_stats(&today).unwrap().unwrap();
        assert_eq!(stats.total_trades, 2);
        assert_eq!(stats.winning_trades, 1);
        assert_eq!(stats.losing_trades, 1);
        assert!((stats.total_pnl - 20.0).abs() < 0.01);
        assert!((stats.win_rate() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_export_closed_trades_csv() {
        let (db, _dir) = create_test_db();

        let pos = create_test_position("export-1", "FCPO", OrderSide::Buy, 4850.0);
        db.upsert_position(&pos).unwrap();
        db.close_position("export-1", 4870.0, CloseReason::TakeProfit)
            .unwrap();

        let export_path = NamedTempFile::new().unwrap();
        db.export_closed_trades_csv(export_path.path()).unwrap();

        let content = std::fs::read_to_string(export_path.path()).unwrap();
        assert!(content.contains("position_id,broker_id"));
        assert!(content.contains("export-1"));
    }

    #[test]
    fn test_export_closed_trades_json() {
        let (db, _dir) = create_test_db();

        let pos = create_test_position("export-2", "FCPO", OrderSide::Sell, 4900.0);
        db.upsert_position(&pos).unwrap();
        db.close_position("export-2", 4880.0, CloseReason::TakeProfit)
            .unwrap();

        let export_path = NamedTempFile::new().unwrap();
        db.export_closed_trades_json(export_path.path()).unwrap();

        let content = std::fs::read_to_string(export_path.path()).unwrap();
        assert!(content.contains("\"position_id\""));
        assert!(content.contains("export-2"));
    }

    #[test]
    fn test_export_daily_stats_csv() {
        let (db, _dir) = create_test_db();

        let today = Utc::now().date_naive().to_string();
        db.update_daily_stats(&today, 10.0, true).unwrap();

        let export_path = NamedTempFile::new().unwrap();
        db.export_daily_stats_csv(export_path.path()).unwrap();

        let content = std::fs::read_to_string(export_path.path()).unwrap();
        assert!(content.contains("date,total_pnl"));
        assert!(content.contains(&today));
    }
}
