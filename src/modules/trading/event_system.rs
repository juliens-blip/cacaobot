//! Real-Time Market Data Pipeline
//!
//! Event-driven system for streaming market data using tokio mpsc channels.
//! Provides publish/subscribe pattern for price updates, order fills, and system events.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};

/// Unique identifier for subscribers
pub type SubscriberId = u64;

/// Market event types for real-time data pipeline
#[derive(Debug, Clone)]
pub enum MarketEvent {
    /// Price tick update
    PriceTick {
        symbol_id: i64,
        symbol: String,
        bid: f64,
        ask: f64,
        spread: f64,
        timestamp: DateTime<Utc>,
    },
    /// OHLCV bar completed
    BarClosed {
        symbol_id: i64,
        symbol: String,
        timeframe: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        timestamp: DateTime<Utc>,
    },
    /// Order filled
    OrderFilled {
        order_id: i64,
        symbol_id: i64,
        side: OrderSide,
        volume: f64,
        price: f64,
        timestamp: DateTime<Utc>,
    },
    /// Order rejected
    OrderRejected {
        order_id: i64,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Position updated
    PositionUpdate {
        position_id: i64,
        symbol_id: i64,
        unrealized_pnl: f64,
        timestamp: DateTime<Utc>,
    },
    /// Position closed
    PositionClosed {
        position_id: i64,
        symbol_id: i64,
        realized_pnl: f64,
        close_reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Connection status
    ConnectionStatus {
        connected: bool,
        message: String,
        timestamp: DateTime<Utc>,
    },
    /// System alert
    Alert {
        level: AlertLevel,
        message: String,
        timestamp: DateTime<Utc>,
    },
    /// Heartbeat for connection health
    Heartbeat {
        timestamp: DateTime<Utc>,
    },
}

/// Order side for events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Event filter for selective subscription
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Only receive events for these symbols (empty = all)
    pub symbols: Vec<i64>,
    /// Event types to receive (empty = all)
    pub event_types: Vec<EventType>,
}

/// Event type identifiers for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    PriceTick,
    BarClosed,
    OrderFilled,
    OrderRejected,
    PositionUpdate,
    PositionClosed,
    ConnectionStatus,
    Alert,
    Heartbeat,
}

impl MarketEvent {
    /// Get the event type
    pub fn event_type(&self) -> EventType {
        match self {
            MarketEvent::PriceTick { .. } => EventType::PriceTick,
            MarketEvent::BarClosed { .. } => EventType::BarClosed,
            MarketEvent::OrderFilled { .. } => EventType::OrderFilled,
            MarketEvent::OrderRejected { .. } => EventType::OrderRejected,
            MarketEvent::PositionUpdate { .. } => EventType::PositionUpdate,
            MarketEvent::PositionClosed { .. } => EventType::PositionClosed,
            MarketEvent::ConnectionStatus { .. } => EventType::ConnectionStatus,
            MarketEvent::Alert { .. } => EventType::Alert,
            MarketEvent::Heartbeat { .. } => EventType::Heartbeat,
        }
    }

    /// Get symbol_id if applicable
    pub fn symbol_id(&self) -> Option<i64> {
        match self {
            MarketEvent::PriceTick { symbol_id, .. } => Some(*symbol_id),
            MarketEvent::BarClosed { symbol_id, .. } => Some(*symbol_id),
            MarketEvent::OrderFilled { symbol_id, .. } => Some(*symbol_id),
            MarketEvent::PositionUpdate { symbol_id, .. } => Some(*symbol_id),
            MarketEvent::PositionClosed { symbol_id, .. } => Some(*symbol_id),
            _ => None,
        }
    }

    /// Get timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            MarketEvent::PriceTick { timestamp, .. } => *timestamp,
            MarketEvent::BarClosed { timestamp, .. } => *timestamp,
            MarketEvent::OrderFilled { timestamp, .. } => *timestamp,
            MarketEvent::OrderRejected { timestamp, .. } => *timestamp,
            MarketEvent::PositionUpdate { timestamp, .. } => *timestamp,
            MarketEvent::PositionClosed { timestamp, .. } => *timestamp,
            MarketEvent::ConnectionStatus { timestamp, .. } => *timestamp,
            MarketEvent::Alert { timestamp, .. } => *timestamp,
            MarketEvent::Heartbeat { timestamp } => *timestamp,
        }
    }
}

impl EventFilter {
    /// Create a new empty filter (receives all events)
    pub fn all() -> Self {
        Self::default()
    }

    /// Filter for specific symbols only
    pub fn symbols(symbols: Vec<i64>) -> Self {
        Self {
            symbols,
            event_types: Vec::new(),
        }
    }

    /// Filter for specific event types only
    pub fn event_types(event_types: Vec<EventType>) -> Self {
        Self {
            symbols: Vec::new(),
            event_types,
        }
    }

    /// Check if event passes the filter
    pub fn matches(&self, event: &MarketEvent) -> bool {
        // Check event type filter
        if !self.event_types.is_empty() && !self.event_types.contains(&event.event_type()) {
            return false;
        }

        // Check symbol filter
        if !self.symbols.is_empty() {
            if let Some(symbol_id) = event.symbol_id() {
                if !self.symbols.contains(&symbol_id) {
                    return false;
                }
            }
        }

        true
    }
}

/// Subscriber information
struct Subscriber {
    sender: mpsc::Sender<MarketEvent>,
    filter: EventFilter,
}

/// Event channel for publish/subscribe pattern
pub struct EventChannel {
    subscribers: Arc<RwLock<HashMap<SubscriberId, Subscriber>>>,
    next_id: Arc<RwLock<SubscriberId>>,
    buffer_size: usize,
}

impl EventChannel {
    /// Create a new event channel with specified buffer size
    pub fn new(buffer_size: usize) -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
            buffer_size,
        }
    }

    /// Create with default buffer size (1000)
    pub fn default_channel() -> Self {
        Self::new(1000)
    }

    /// Subscribe to events with optional filter
    /// Returns (subscriber_id, receiver)
    pub async fn subscribe(&self, filter: EventFilter) -> (SubscriberId, mpsc::Receiver<MarketEvent>) {
        let (tx, rx) = mpsc::channel(self.buffer_size);

        let mut next_id = self.next_id.write().await;
        let id = *next_id;
        *next_id += 1;

        let subscriber = Subscriber {
            sender: tx,
            filter,
        };

        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(id, subscriber);

        (id, rx)
    }

    /// Subscribe to all events
    pub async fn subscribe_all(&self) -> (SubscriberId, mpsc::Receiver<MarketEvent>) {
        self.subscribe(EventFilter::all()).await
    }

    /// Subscribe to specific symbols
    pub async fn subscribe_symbols(&self, symbols: Vec<i64>) -> (SubscriberId, mpsc::Receiver<MarketEvent>) {
        self.subscribe(EventFilter::symbols(symbols)).await
    }

    /// Subscribe to specific event types
    pub async fn subscribe_types(&self, types: Vec<EventType>) -> (SubscriberId, mpsc::Receiver<MarketEvent>) {
        self.subscribe(EventFilter::event_types(types)).await
    }

    /// Unsubscribe a subscriber
    pub async fn unsubscribe(&self, id: SubscriberId) -> bool {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(&id).is_some()
    }

    /// Publish an event to all matching subscribers
    pub async fn publish(&self, event: MarketEvent) -> usize {
        let subscribers = self.subscribers.read().await;
        let mut sent_count = 0;

        for subscriber in subscribers.values() {
            if subscriber.filter.matches(&event) {
                // Non-blocking send - skip if buffer is full
                if subscriber.sender.try_send(event.clone()).is_ok() {
                    sent_count += 1;
                }
            }
        }

        sent_count
    }

    /// Publish with guaranteed delivery (blocks if buffer full)
    pub async fn publish_guaranteed(&self, event: MarketEvent) -> usize {
        let subscribers = self.subscribers.read().await;
        let mut sent_count = 0;

        for subscriber in subscribers.values() {
            if subscriber.filter.matches(&event)
                && subscriber.sender.send(event.clone()).await.is_ok()
            {
                sent_count += 1;
            }
        }

        sent_count
    }

    /// Get current subscriber count
    pub async fn subscriber_count(&self) -> usize {
        self.subscribers.read().await.len()
    }

    /// Remove disconnected subscribers (cleanup)
    pub async fn cleanup_disconnected(&self) -> usize {
        let mut subscribers = self.subscribers.write().await;
        let before = subscribers.len();

        subscribers.retain(|_, sub| !sub.sender.is_closed());

        before - subscribers.len()
    }
}

/// Handle to an event channel for easy cloning
#[derive(Clone)]
pub struct EventChannelHandle {
    inner: Arc<EventChannel>,
}

impl EventChannelHandle {
    /// Create a new handle
    pub fn new(buffer_size: usize) -> Self {
        Self {
            inner: Arc::new(EventChannel::new(buffer_size)),
        }
    }

    /// Subscribe to events
    pub async fn subscribe(&self, filter: EventFilter) -> (SubscriberId, mpsc::Receiver<MarketEvent>) {
        self.inner.subscribe(filter).await
    }

    /// Subscribe to all events
    pub async fn subscribe_all(&self) -> (SubscriberId, mpsc::Receiver<MarketEvent>) {
        self.inner.subscribe_all().await
    }

    /// Publish an event
    pub async fn publish(&self, event: MarketEvent) -> usize {
        self.inner.publish(event).await
    }

    /// Publish with guaranteed delivery
    pub async fn publish_guaranteed(&self, event: MarketEvent) -> usize {
        self.inner.publish_guaranteed(event).await
    }

    /// Unsubscribe
    pub async fn unsubscribe(&self, id: SubscriberId) -> bool {
        self.inner.unsubscribe(id).await
    }

    /// Get subscriber count
    pub async fn subscriber_count(&self) -> usize {
        self.inner.subscriber_count().await
    }

    /// Cleanup disconnected subscribers
    pub async fn cleanup(&self) -> usize {
        self.inner.cleanup_disconnected().await
    }
}

impl Default for EventChannelHandle {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscribe_and_publish() {
        let channel = EventChannel::new(100);

        let (_id, mut rx) = channel.subscribe_all().await;

        let event = MarketEvent::PriceTick {
            symbol_id: 1,
            symbol: "FCPO".to_string(),
            bid: 4000.0,
            ask: 4001.0,
            spread: 1.0,
            timestamp: Utc::now(),
        };

        let sent = channel.publish(event.clone()).await;
        assert_eq!(sent, 1);

        let received = rx.recv().await.unwrap();
        if let MarketEvent::PriceTick { symbol, bid, .. } = received {
            assert_eq!(symbol, "FCPO");
            assert_eq!(bid, 4000.0);
        } else {
            panic!("Wrong event type received");
        }
    }

    #[tokio::test]
    async fn test_filter_by_symbol() {
        let channel = EventChannel::new(100);

        // Subscribe only to symbol 1
        let (_id, mut rx) = channel.subscribe_symbols(vec![1]).await;

        // Publish for symbol 1
        channel.publish(MarketEvent::PriceTick {
            symbol_id: 1,
            symbol: "FCPO".to_string(),
            bid: 4000.0,
            ask: 4001.0,
            spread: 1.0,
            timestamp: Utc::now(),
        }).await;

        // Publish for symbol 2 (should be filtered)
        channel.publish(MarketEvent::PriceTick {
            symbol_id: 2,
            symbol: "CRUDE".to_string(),
            bid: 70.0,
            ask: 70.1,
            spread: 0.1,
            timestamp: Utc::now(),
        }).await;

        // Should only receive symbol 1
        let received = rx.recv().await.unwrap();
        assert_eq!(received.symbol_id(), Some(1));

        // Channel should be empty for symbol 2
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_filter_by_event_type() {
        let channel = EventChannel::new(100);

        // Subscribe only to alerts
        let (_id, mut rx) = channel.subscribe_types(vec![EventType::Alert]).await;

        // Publish price tick (should be filtered)
        channel.publish(MarketEvent::PriceTick {
            symbol_id: 1,
            symbol: "FCPO".to_string(),
            bid: 4000.0,
            ask: 4001.0,
            spread: 1.0,
            timestamp: Utc::now(),
        }).await;

        // Publish alert (should pass)
        channel.publish(MarketEvent::Alert {
            level: AlertLevel::Warning,
            message: "Test alert".to_string(),
            timestamp: Utc::now(),
        }).await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received.event_type(), EventType::Alert);

        // Should be no more events
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let channel = EventChannel::new(100);

        let (_id1, mut rx1) = channel.subscribe_all().await;
        let (_id2, mut rx2) = channel.subscribe_all().await;

        assert_eq!(channel.subscriber_count().await, 2);

        channel.publish(MarketEvent::Heartbeat {
            timestamp: Utc::now(),
        }).await;

        assert!(rx1.recv().await.is_some());
        assert!(rx2.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let channel = EventChannel::new(100);

        let (id, _rx) = channel.subscribe_all().await;
        assert_eq!(channel.subscriber_count().await, 1);

        let removed = channel.unsubscribe(id).await;
        assert!(removed);
        assert_eq!(channel.subscriber_count().await, 0);
    }

    #[tokio::test]
    async fn test_event_channel_handle() {
        let handle = EventChannelHandle::new(100);
        let handle2 = handle.clone();

        let (_id, mut rx) = handle.subscribe_all().await;

        // Publish from cloned handle
        handle2.publish(MarketEvent::Heartbeat {
            timestamp: Utc::now(),
        }).await;

        assert!(rx.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_cleanup_disconnected() {
        let channel = EventChannel::new(100);

        // Create and immediately drop receiver
        let (id, rx) = channel.subscribe_all().await;
        drop(rx);

        assert_eq!(channel.subscriber_count().await, 1);

        // Cleanup should remove disconnected subscriber
        let cleaned = channel.cleanup_disconnected().await;
        assert_eq!(cleaned, 1);
        assert_eq!(channel.subscriber_count().await, 0);

        // Verify the ID was removed
        assert!(!channel.unsubscribe(id).await);
    }

    #[test]
    fn test_event_filter_matches() {
        let filter = EventFilter {
            symbols: vec![1, 2],
            event_types: vec![EventType::PriceTick],
        };

        // Should match: correct symbol and type
        let event1 = MarketEvent::PriceTick {
            symbol_id: 1,
            symbol: "FCPO".to_string(),
            bid: 4000.0,
            ask: 4001.0,
            spread: 1.0,
            timestamp: Utc::now(),
        };
        assert!(filter.matches(&event1));

        // Should not match: wrong symbol
        let event2 = MarketEvent::PriceTick {
            symbol_id: 99,
            symbol: "OTHER".to_string(),
            bid: 100.0,
            ask: 101.0,
            spread: 1.0,
            timestamp: Utc::now(),
        };
        assert!(!filter.matches(&event2));

        // Should not match: wrong event type
        let event3 = MarketEvent::Alert {
            level: AlertLevel::Info,
            message: "Test".to_string(),
            timestamp: Utc::now(),
        };
        assert!(!filter.matches(&event3));
    }

    #[test]
    fn test_market_event_accessors() {
        let event = MarketEvent::BarClosed {
            symbol_id: 42,
            symbol: "FCPO".to_string(),
            timeframe: "1H".to_string(),
            open: 4000.0,
            high: 4050.0,
            low: 3990.0,
            close: 4030.0,
            volume: 1000.0,
            timestamp: Utc::now(),
        };

        assert_eq!(event.event_type(), EventType::BarClosed);
        assert_eq!(event.symbol_id(), Some(42));
    }
}
