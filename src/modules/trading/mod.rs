//! Trading module
//!
//! This module contains:
//! - `ctrader`: cTrader Open API client (Protobuf/TCP)
//! - `protobuf`: Protobuf message definitions for cTrader
//! - `indicators`: Technical indicators (RSI)
//! - `strategy`: Trading strategy logic
//! - `orders`: Order and position management

pub mod candles;
pub mod circuit_breakers;
pub mod ctrader;
pub mod event_system;
pub mod indicators;
pub mod oauth;
pub mod orders;
pub mod persistence;
pub mod position_manager;
pub mod position_reconciliation;
pub mod protobuf;
pub mod reconciliation;
pub mod strategy;

pub use candles::{Candle, CandleBuilder, TimeFrame, Tick};
pub use circuit_breakers::CircuitBreakers;
pub use ctrader::{CTraderClient, CTraderEnvironment, Price, OrderTicket, SymbolMeta};
pub use event_system::{MarketEvent, EventChannel, EventChannelHandle, EventFilter, EventType, AlertLevel, SubscriberId};
pub use indicators::{RsiCalculator, PricePoint};
pub use oauth::OAuthClient;
pub use orders::{Order, OrderSide, OrderStatus, Position, PositionManager, ClosedPosition, CloseReason};
pub use persistence::{PositionDatabase, DailyStats, ClosedTradeRecord};
pub use position_manager::{PersistentPositionManager, BrokerPosition, ReconciliationResult};
pub use position_reconciliation::{
    PositionReconciliationSystem, ConnectionState, ReconciliationConfig,
    ReconciliationReport, ReconciliationMismatch, AuditEntry, AuditEventType,
    BrokerPositionData, CachedPosition, ReconciliationState,
};
pub use reconciliation::ReconciliationEngine;
pub use strategy::{TradingStrategy, Signal, RiskState};
