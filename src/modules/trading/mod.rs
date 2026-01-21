//! Trading module
//!
//! This module contains:
//! - `ctrader`: cTrader Open API client (Protobuf/TCP)
//! - `protobuf`: Protobuf message definitions for cTrader
//! - `indicators`: Technical indicators (RSI)
//! - `strategy`: Trading strategy logic
//! - `orders`: Order and position management

pub mod ctrader;
pub mod protobuf;
pub mod indicators;
pub mod orders;
pub mod strategy;
pub mod circuit_breakers;
pub mod event_system;
pub mod candles;

pub use ctrader::{CTraderClient, Price, OrderTicket};
pub use indicators::{RsiCalculator, PricePoint};
pub use orders::{Order, OrderSide, OrderStatus, Position, PositionManager, ClosedPosition, CloseReason};
pub use strategy::{TradingStrategy, Signal, RiskState};
pub use circuit_breakers::CircuitBreakers;
pub use event_system::{MarketEvent, EventChannel, EventChannelHandle, EventFilter, EventType, AlertLevel, SubscriberId};
pub use candles::{Candle, CandleBuilder, TimeFrame, Tick};
