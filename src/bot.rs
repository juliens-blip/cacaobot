//! Trading bot main loop.
//!
//! Aggregates ticks into candles, computes RSI, combines sentiment,
//! and executes trades while respecting circuit breakers.

use crate::config::Config;
use crate::error::{BotError, CTraderError, Result};
use crate::modules::monitoring::{metrics_enabled, start_metrics_server};
use crate::modules::monitoring::{MetricsHandle, Trade};
use crate::modules::scraper::{PerplexityClient, SentimentResult, TwitterScraper};
use crate::modules::security::ApiRateLimiter;
use crate::modules::trading::protobuf::ProtoOATradeSide;
use crate::modules::trading::{
    Candle, CandleBuilder, CTraderClient, EventChannelHandle, MarketEvent, OrderSide, OrderTicket,
    RsiCalculator, Signal, Tick, TimeFrame, TradingStrategy, PositionDatabase, CloseReason, Position,
};
use crate::modules::utils::{retry_with_backoff, RetryConfig};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::sync::Arc;
use std::{env, fs, path::Path};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Sentiment cache TTL in minutes
const SENTIMENT_CACHE_TTL_MINUTES: i64 = 5;

/// Neutral sentiment value used as fallback
const NEUTRAL_SENTIMENT: i32 = 0;

/// Cached sentiment data with TTL
#[derive(Debug, Clone)]
pub struct SentimentCache {
    /// Cached sentiment score (-100 to +100)
    pub value: i32,
    /// Full sentiment result
    pub result: Option<SentimentResult>,
    /// When the cache was last updated
    pub timestamp: DateTime<Utc>,
    /// Cache time-to-live
    pub ttl: ChronoDuration,
}

impl SentimentCache {
    /// Create a new empty cache with specified TTL
    pub fn new(ttl_minutes: i64) -> Self {
        Self {
            value: NEUTRAL_SENTIMENT,
            result: None,
            timestamp: DateTime::<Utc>::MIN_UTC,
            ttl: ChronoDuration::minutes(ttl_minutes),
        }
    }

    /// Check if the cache is still valid
    pub fn is_valid(&self) -> bool {
        let age = Utc::now() - self.timestamp;
        age < self.ttl
    }

    /// Get cached value if valid, otherwise None
    pub fn get(&self) -> Option<i32> {
        if self.is_valid() {
            Some(self.value)
        } else {
            None
        }
    }

    /// Update the cache with a new value
    pub fn update(&mut self, value: i32, result: Option<SentimentResult>) {
        self.value = value;
        self.result = result;
        self.timestamp = Utc::now();
    }

    /// Get time until cache expires (or zero if expired)
    pub fn time_until_expiry(&self) -> ChronoDuration {
        let age = Utc::now() - self.timestamp;
        let remaining = self.ttl - age;
        if remaining.num_seconds() > 0 {
            remaining
        } else {
            ChronoDuration::zero()
        }
    }

    /// Force invalidate the cache
    pub fn invalidate(&mut self) {
        self.timestamp = DateTime::<Utc>::MIN_UTC;
    }
}

impl Default for SentimentCache {
    fn default() -> Self {
        Self::new(SENTIMENT_CACHE_TTL_MINUTES)
    }
}

pub struct TradingBot {
    strategy: TradingStrategy,
    ctrader: CTraderClient,
    candle_builder: CandleBuilder,
    rsi_calculator: RsiCalculator,
    event_channel: EventChannelHandle,
    config: Config,
    perplexity: PerplexityClient,
    twitter: TwitterScraper,
    position_db: Option<PositionDatabase>,
    metrics: MetricsHandle,
    symbol_id: i64,
    last_price: Option<f64>,
    /// Sentiment cache to avoid excessive API calls
    sentiment_cache: Arc<RwLock<SentimentCache>>,
}

impl TradingBot {
    pub fn new(config: Config) -> Result<Self> {
        let timeframe = parse_timeframe(&config.strategy.rsi_timeframe);
        let strategy = TradingStrategy::new(
            config.strategy.clone(),
            config.trading.clone(),
            config.trading.initial_balance,
        );
        let ctrader = CTraderClient::new(config.ctrader.clone());
        let candle_builder = CandleBuilder::new(timeframe);
        let rsi_calculator = RsiCalculator::new(config.strategy.rsi_period);
        let event_channel = EventChannelHandle::default();
        
        // Create rate limiters for API clients
        let perplexity_rate_limiter = Arc::new(ApiRateLimiter::for_perplexity());
        let twitter_rate_limiter = Arc::new(ApiRateLimiter::for_twitter());
        
        let perplexity = PerplexityClient::with_symbol(config.perplexity.clone(), perplexity_rate_limiter, &config.trading.symbol);
        let twitter = TwitterScraper::new(config.kols.clone(), twitter_rate_limiter);
        let position_db = init_position_db();
        let metrics = MetricsHandle::new(config.trading.initial_balance);

        Ok(Self {
            strategy,
            ctrader,
            candle_builder,
            rsi_calculator,
            event_channel,
            config,
            perplexity,
            twitter,
            position_db,
            metrics,
            symbol_id: 0,
            last_price: None,
            sentiment_cache: Arc::new(RwLock::new(SentimentCache::default())),
        })
    }

    /// Main trading loop.
    pub async fn run(&mut self) -> Result<()> {
        info!("Trading bot starting...");

        if self.config.bot.dry_run && self.config.ctrader.access_token.is_none() {
            return self.run_offline_dry_run().await;
        }

        self.ctrader.verify_credentials()?;
        connect_with_retry(&self.ctrader).await?;
        authenticate_with_retry(&self.ctrader).await?;

        if metrics_enabled() {
            start_metrics_server(self.metrics.clone());
        }

        // Dynamically resolve symbol ID from broker
        let symbol_name = &self.config.trading.symbol;
        let symbol_id = self
            .ctrader
            .get_symbol_id(symbol_name)
            .await
            .map_err(|err| {
                BotError::Other(format!(
                    "Failed to resolve symbol ID for '{}': {}",
                    symbol_name, err
                ))
            })?;
        self.symbol_id = symbol_id;
        info!("🌴 Trading {} with symbol ID: {}", symbol_name, symbol_id);

        if !self.config.bot.dry_run {
            self.reconcile_positions().await?;
        } else {
            info!("Skipping broker reconciliation in dry_run mode");
        }
        self.ctrader.subscribe_to_symbol(self.symbol_id).await?;

        self.event_channel
            .publish(MarketEvent::ConnectionStatus {
                connected: true,
                message: "Connected to cTrader".to_string(),
                timestamp: Utc::now(),
            })
            .await;

        let mut ticker = interval(Duration::from_secs(self.config.bot.cycle_interval_secs));
        let mut reconcile_interval = interval(Duration::from_secs(300));
        reconcile_interval.tick().await;

        let shutdown = tokio::signal::ctrl_c();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    info!("Received shutdown signal");
                    break;
                }
                _ = reconcile_interval.tick() => {
                    if !self.config.bot.dry_run {
                        if let Err(err) = self.reconcile_positions().await {
                            warn!("Reconciliation error: {}", err);
                        }
                    }
                }
                _ = ticker.tick() => {
                    let price = match self.ctrader.get_price(self.symbol_id).await {
                        Ok(price) => price,
                        Err(err) => {
                            warn!("Failed to fetch price: {}", err);
                            if should_retry_ctrader(&err) {
                                warn!("Attempting reconnect after price error");
                                let _ = self.ctrader.disconnect().await;
                                if connect_with_retry(&self.ctrader).await.is_ok()
                                    && authenticate_with_retry(&self.ctrader).await.is_ok()
                                {
                                    continue;
                                }
                            }
                            continue;
                        }
                    };

                    let mid_price = (price.bid + price.ask) / 2.0;
                    let tick = Tick::new(price.timestamp, mid_price);
                    self.process_tick(tick).await?;
                }
            }
        }

        self.shutdown().await?;
        Ok(())
    }

    /// Offline dry-run mode: runs without cTrader connection using synthetic prices.
    /// Useful for testing the full trading pipeline without OAuth credentials.
    async fn run_offline_dry_run(&mut self) -> Result<()> {
        // Use faster cycle interval for offline testing (5s instead of 60s)
        let cycle_interval = 5;
        info!("========================================");
        info!("  OFFLINE DRY-RUN MODE (cycle: {}s)", cycle_interval);
        info!("  No cTrader connection required");
        info!("  Using synthetic price data");
        info!("========================================");

        if metrics_enabled() {
            start_metrics_server(self.metrics.clone());
        }

        self.symbol_id = 1; // synthetic symbol ID
        let base_price: f64 = 4200.0; // typical FCPO price in MYR
        let mut price = base_price;
        let mut cycle: u64 = 0;

        let mut ticker = interval(Duration::from_secs(cycle_interval));

        let shutdown = tokio::signal::ctrl_c();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    info!("Received shutdown signal");
                    break;
                }
                _ = ticker.tick() => {
                    cycle += 1;
                    // Random walk: ±0.5% per tick
                    let change = ((cycle as f64 * 1.618).sin() * 0.005) * base_price;
                    price += change;
                    price = price.max(base_price * 0.9).min(base_price * 1.1); // clamp ±10%

                    info!("[Cycle {}] Synthetic price: {:.2} MYR", cycle, price);

                    let tick = Tick::new(Utc::now(), price);
                    if let Err(err) = self.process_tick(tick).await {
                        warn!("Tick processing error: {}", err);
                    }
                }
            }
        }

        self.shutdown().await?;
        Ok(())
    }

    /// Process a single tick.
    async fn process_tick(&mut self, tick: Tick) -> Result<()> {
        self.last_price = Some(tick.price);

        self.event_channel
            .publish(MarketEvent::PriceTick {
                symbol_id: self.symbol_id,
                symbol: self.config.trading.symbol.clone(),
                bid: tick.price,
                ask: tick.price,
                spread: 0.0,
                timestamp: tick.timestamp,
            })
            .await;

        self.strategy.update_price(tick.price);
        self.check_exits().await?;

        if let Some(candle) = self.candle_builder.add_tick(tick) {
            self.event_channel
                .publish(MarketEvent::BarClosed {
                    symbol_id: self.symbol_id,
                    symbol: self.config.trading.symbol.clone(),
                    timeframe: candle.timeframe.to_string(),
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                    volume: candle.volume as f64,
                    timestamp: candle.timestamp,
                })
                .await;

            self.process_signal(&candle).await?;
        }

        Ok(())
    }

    /// Check for position exits.
    async fn check_exits(&mut self) -> Result<()> {
        let price = match self.last_price {
            Some(price) => price,
            None => return Ok(()),
        };

        let positions: Vec<_> = self.strategy.get_open_positions().to_vec();
        for position in positions {
            if let Some(reason) = self.strategy.check_position_exit(&position, price) {
                info!("Closing position {} due to {:?}", position.id, reason);

                if !self.config.bot.dry_run {
                    let position_id = match position.id.parse::<i64>() {
                        Ok(id) => id,
                        Err(_) => {
                            warn!("Skipping close: invalid position id {}", position.id);
                            continue;
                        }
                    };
                    let volume = (position.volume * 100.0) as i64;
                    self.ctrader.close_position(position_id, volume).await?;
                    // Reconcile immediately after close
                    if let Err(err) = self.reconcile_positions().await {
                        warn!("Post-close reconciliation failed: {}", err);
                    }
                }

                if let Some(pnl) = self.strategy.close_position(&position.id, price, reason) {
                    self.persist_close_position(&position.id, price, reason);
                    self.metrics.with_metrics_mut(|m| {
                        let _ = m.close_trade(&position.id, price);
                    });
                    self.event_channel
                        .publish(MarketEvent::PositionClosed {
                            position_id: position.id.parse().unwrap_or_default(),
                            symbol_id: self.symbol_id,
                            realized_pnl: pnl,
                            close_reason: reason.to_string(),
                            timestamp: Utc::now(),
                        })
                        .await;
                }
            }
        }

        Ok(())
    }

    async fn process_signal(&mut self, candle: &Candle) -> Result<()> {
        let rsi = match self.rsi_calculator.add_price(candle.close) {
            Some(value) => value,
            None => {
                debug!("RSI not ready yet");
                return Ok(());
            }
        };

        let sentiment = self.fetch_current_sentiment().await;
        self.metrics.with_metrics_mut(|m| {
            m.update_market_data(candle.close, rsi, sentiment.score);
        });
        let signal = self.strategy.generate_signal(rsi, sentiment.score);

        if !self.strategy.can_open_position()? {
            self.event_channel
                .publish(MarketEvent::Alert {
                    level: crate::modules::trading::AlertLevel::Warning,
                    message: "Circuit breakers active: no new positions".to_string(),
                    timestamp: Utc::now(),
                })
                .await;
            return Ok(());
        }

        match signal {
            Signal::Buy => self.execute_trade(OrderSide::Buy, candle.close).await?,
            Signal::Sell => self.execute_trade(OrderSide::Sell, candle.close).await?,
            Signal::Hold => {}
        }

        Ok(())
    }

    async fn execute_trade(&mut self, side: OrderSide, entry_price: f64) -> Result<()> {
        let take_profit = self.strategy.calculate_take_profit(entry_price, side);
        let stop_loss = self.strategy.calculate_stop_loss(entry_price, side);
        let volume = self.strategy.calculate_position_size(entry_price, stop_loss);

        info!(
            "Signal: {:?} entry={:.2} tp={:.2} sl={:.2} vol={:.2}",
            side, entry_price, take_profit, stop_loss, volume
        );

        if self.config.bot.dry_run {
            let position_id = format!("dry_run_{}", Utc::now().timestamp_millis());
            let position = crate::modules::trading::Position::new(
                position_id.clone(),
                self.config.trading.symbol.clone(),
                side,
                entry_price,
                volume,
            )
            .with_take_profit(take_profit)
            .with_stop_loss(stop_loss);
            self.persist_open_position(&position);
            self.metrics.with_metrics_mut(|m| {
                m.add_trade(Trade::new(position_id.clone(), format!("{:?}", side), volume, entry_price));
            });
            self.strategy.add_position(position);
            return Ok(());
        }

        let trade_side = match side {
            OrderSide::Buy => ProtoOATradeSide::Buy,
            OrderSide::Sell => ProtoOATradeSide::Sell,
        };

        let ticket = OrderTicket {
            symbol_id: self.symbol_id,
            side: trade_side,
            volume: (volume * 100.0) as i64,
            stop_loss: Some(stop_loss),
            take_profit: Some(take_profit),
            label: Some("PalmOilBot".to_string()),
        };

        match self.ctrader.place_order(ticket).await {
            Ok((order_id, position_id)) => {
                self.event_channel
                    .publish(MarketEvent::OrderFilled {
                        order_id,
                        symbol_id: self.symbol_id,
                        side: match side {
                            OrderSide::Buy => crate::modules::trading::event_system::OrderSide::Buy,
                            OrderSide::Sell => {
                                crate::modules::trading::event_system::OrderSide::Sell
                            }
                        },
                        volume,
                        price: entry_price,
                        timestamp: Utc::now(),
                    })
                    .await;

                let position = crate::modules::trading::Position::new(
                    position_id.to_string(),
                    self.config.trading.symbol.clone(),
                    side,
                    entry_price,
                    volume,
                )
                .with_take_profit(take_profit)
                .with_stop_loss(stop_loss);

                self.persist_open_position(&position);
                self.metrics.with_metrics_mut(|m| {
                    m.add_trade(Trade::new(position_id.to_string(), format!("{:?}", side), volume, entry_price));
                });
                self.strategy.add_position(position);

                // Reconcile immediately after order fill
                if let Err(err) = self.reconcile_positions().await {
                    warn!("Post-fill reconciliation failed: {}", err);
                }
            }
            Err(err) => {
                error!("Order placement failed: {}", err);
                self.event_channel
                    .publish(MarketEvent::OrderRejected {
                        order_id: 0,
                        reason: err.to_string(),
                        timestamp: Utc::now(),
                    })
                    .await;
            }
        }

        Ok(())
    }

    /// Fetch current sentiment with caching (TTL 5 minutes)
    ///
    /// Returns cached value if valid, otherwise fetches from Perplexity API.
    /// Falls back to Twitter sentiment, then neutral (0) if all APIs fail.
    pub async fn fetch_current_sentiment(&self) -> SentimentResult {
        // Check cache first
        {
            let cache = self.sentiment_cache.read().await;
            if let Some(result) = &cache.result {
                if cache.is_valid() {
                    debug!(
                        "Using cached sentiment: {} (expires in {}s)",
                        cache.value,
                        cache.time_until_expiry().num_seconds()
                    );
                    return result.clone();
                }
            }
        }

        // Cache miss - fetch from Perplexity API
        info!("Sentiment cache expired, fetching from Perplexity API...");

        let result = match self.perplexity.get_market_sentiment().await {
            Ok(sentiment) => {
                info!(
                    "Perplexity sentiment: {} ({:?}, confidence: {:.2})",
                    sentiment.score, sentiment.sentiment_type, sentiment.confidence
                );
                sentiment
            }
            Err(crate::error::BotError::Perplexity(crate::error::PerplexityError::RateLimited)) => {
                warn!("Perplexity rate limited (HTTP 429), falling back to Twitter");
                match self.twitter.get_sentiment().await {
                    Ok(sentiment) => {
                        info!("Twitter sentiment fallback: {}", sentiment.score);
                        sentiment
                    }
                    Err(err) => {
                        warn!("Twitter sentiment also failed: {}", err);
                        warn!("Using neutral sentiment ({}) as fallback", NEUTRAL_SENTIMENT);
                        SentimentResult::new(NEUTRAL_SENTIMENT, "fallback").with_confidence(0.1)
                    }
                }
            }
            Err(err) => {
                warn!("Perplexity API failed: {}", err);
                warn!("Using neutral sentiment ({}) as fallback", NEUTRAL_SENTIMENT);
                SentimentResult::new(NEUTRAL_SENTIMENT, "fallback").with_confidence(0.1)
            }
        };

        // Update cache
        {
            let mut cache = self.sentiment_cache.write().await;
            cache.update(result.score, Some(result.clone()));
        }

        result
    }

    /// Get cached sentiment without fetching (returns None if expired)
    pub async fn get_cached_sentiment(&self) -> Option<SentimentResult> {
        let cache = self.sentiment_cache.read().await;
        if cache.is_valid() {
            cache.result.clone()
        } else {
            None
        }
    }

    /// Force refresh sentiment (bypass cache)
    pub async fn force_refresh_sentiment(&self) -> SentimentResult {
        // Invalidate cache
        {
            let mut cache = self.sentiment_cache.write().await;
            cache.invalidate();
        }
        // Fetch fresh
        self.fetch_current_sentiment().await
    }

    /// Reconcile open positions with broker state (basic)
    async fn reconcile_positions(&mut self) -> Result<()> {
        let broker_positions = match self.ctrader.reconcile_positions().await {
            Ok(positions) => positions,
            Err(err) => {
                warn!("Reconciliation failed: {}", err);
                Vec::new()
            }
        };

        if broker_positions.is_empty() {
            info!("No broker positions found during reconciliation");
            return Ok(());
        }

        let mut reconciled = Vec::new();
        for pos in broker_positions {
            let side = match pos.side.as_str() {
                "BUY" => OrderSide::Buy,
                "SELL" => OrderSide::Sell,
                _ => {
                    warn!("Skipping position {} with unknown side: {}", pos.position_id, pos.side);
                    continue;
                }
            };

            let volume = (pos.volume as f64) / 100.0;
            let mut position = crate::modules::trading::Position::new(
                pos.position_id.to_string(),
                self.config.trading.symbol.clone(),
                side,
                pos.entry_price,
                volume,
            )
            .with_take_profit(self.strategy.calculate_take_profit(pos.entry_price, side))
            .with_stop_loss(self.strategy.calculate_stop_loss(pos.entry_price, side));
            position.current_price = pos.current_price;
            position.current_pnl = pos.profit;

            reconciled.push(position);
        }

        self.strategy.reconcile_positions(reconciled);
        info!("Reconciled broker positions into strategy state");
        Ok(())
    }

    /// Shutdown bot and disconnect
    pub async fn shutdown(&mut self) -> Result<()> {
        self.ctrader.disconnect().await?;
        Ok(())
    }

    fn persist_open_position(&self, position: &Position) {
        let Some(db) = &self.position_db else {
            return;
        };
        if let Err(err) = db.upsert_position(position) {
            warn!(
                "Failed to persist open position {}: {}",
                position.id, err
            );
        }
    }

    fn persist_close_position(&self, position_id: &str, exit_price: f64, reason: CloseReason) {
        let Some(db) = &self.position_db else {
            return;
        };
        if let Err(err) = db.close_position(position_id, exit_price, reason) {
            warn!(
                "Failed to persist closed position {}: {}",
                position_id, err
            );
        }
    }
}

fn init_position_db() -> Option<PositionDatabase> {
    let db_path = env::var("PERSISTENCE_DB_PATH").unwrap_or_else(|_| "data/positions.db".to_string());
    if let Some(parent) = Path::new(&db_path).parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            warn!("Failed to create persistence directory {:?}: {}", parent, err);
            return None;
        }
    }

    match PositionDatabase::new(&db_path) {
        Ok(db) => {
            info!("SQLite persistence enabled at {}", db_path);
            Some(db)
        }
        Err(err) => {
            warn!("SQLite persistence disabled: {}", err);
            None
        }
    }
}

fn should_retry_ctrader(err: &BotError) -> bool {
    matches!(
        err,
        BotError::CTrader(CTraderError::ConnectionFailed(_))
            | BotError::CTrader(CTraderError::Timeout)
            | BotError::CTrader(CTraderError::Disconnected)
    )
}

async fn connect_with_retry(client: &CTraderClient) -> Result<()> {
    let config = RetryConfig::default()
        .with_max_delay(30000)
        .with_backoff_multiplier(2.0);
    retry_with_backoff(config, || async { client.connect().await }, should_retry_ctrader).await
}

async fn authenticate_with_retry(client: &CTraderClient) -> Result<()> {
    let config = RetryConfig::default()
        .with_max_delay(30000)
        .with_backoff_multiplier(2.0);
    retry_with_backoff(
        config,
        || async { client.authenticate().await },
        should_retry_ctrader,
    )
    .await
}

fn parse_timeframe(timeframe: &str) -> TimeFrame {
    match timeframe.to_lowercase().as_str() {
        "1m" | "m1" => TimeFrame::M1,
        "5m" | "m5" => TimeFrame::M5,
        "15m" | "m15" => TimeFrame::M15,
        "30m" | "m30" => TimeFrame::M30,
        "1h" | "h1" => TimeFrame::H1,
        "4h" | "h4" => TimeFrame::H4,
        "1d" | "d1" => TimeFrame::D1,
        _ => TimeFrame::M5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentiment_cache_new() {
        let cache = SentimentCache::new(5);
        assert_eq!(cache.value, NEUTRAL_SENTIMENT);
        assert!(!cache.is_valid()); // Empty cache is not valid
        assert!(cache.result.is_none());
    }

    #[test]
    fn test_sentiment_cache_update_and_get() {
        let mut cache = SentimentCache::new(5);

        let result = SentimentResult::new(75, "test");
        cache.update(75, Some(result));

        assert!(cache.is_valid());
        assert_eq!(cache.get(), Some(75));
        assert!(cache.result.is_some());
    }

    #[test]
    fn test_sentiment_cache_invalidate() {
        let mut cache = SentimentCache::new(5);
        cache.update(50, Some(SentimentResult::new(50, "test")));

        assert!(cache.is_valid());

        cache.invalidate();

        assert!(!cache.is_valid());
        assert_eq!(cache.get(), None);
    }

    #[test]
    fn test_sentiment_cache_ttl() {
        let mut cache = SentimentCache::new(5);
        cache.update(50, None);

        // Should be valid immediately after update
        assert!(cache.is_valid());

        // Time until expiry should be close to 5 minutes
        let remaining = cache.time_until_expiry();
        assert!(remaining.num_minutes() >= 4);
    }

    #[test]
    fn test_sentiment_cache_default() {
        let cache = SentimentCache::default();
        assert_eq!(cache.ttl, ChronoDuration::minutes(SENTIMENT_CACHE_TTL_MINUTES));
        assert_eq!(cache.value, NEUTRAL_SENTIMENT);
    }

    #[test]
    fn test_parse_timeframe() {
        assert_eq!(parse_timeframe("1m"), TimeFrame::M1);
        assert_eq!(parse_timeframe("5m"), TimeFrame::M5);
        assert_eq!(parse_timeframe("M5"), TimeFrame::M5);
        assert_eq!(parse_timeframe("1h"), TimeFrame::H1);
        assert_eq!(parse_timeframe("H4"), TimeFrame::H4);
        assert_eq!(parse_timeframe("1d"), TimeFrame::D1);
        assert_eq!(parse_timeframe("invalid"), TimeFrame::M5); // Default
    }
}
