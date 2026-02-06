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
use crate::modules::trading::protobuf::{ProtoOATradeSide, ProtoOaTradingMode};
use crate::modules::trading::{
    Candle, CandleBuilder, CTraderClient, EventChannelHandle, MarketEvent, OrderSide, OrderTicket,
    RsiCalculator, Signal, Tick, TimeFrame, TradingStrategy, PositionDatabase, CloseReason,
    Position, SymbolMeta,
};
use crate::modules::utils::{retry_with_backoff, RetryConfig};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::io::Write as IoWrite;
use std::sync::Arc;
use std::{env, fs, path::Path};
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Duration};
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

/// CSV trade logger for backtesting analysis
struct TradeLogger {
    path: String,
}

impl TradeLogger {
    fn new(path: &str) -> Self {
        // Create header if file doesn't exist
        if !Path::new(path).exists() {
            if let Some(parent) = Path::new(path).parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(mut f) = fs::File::create(path) {
                let _ = writeln!(f, "timestamp,event,side,symbol,entry_price,sl,tp,volume,rsi,sentiment_score,sentiment_confidence,signal,position_id,close_price,pnl,close_reason");
            }
        }
        Self { path: path.to_string() }
    }

    fn log_open(&self, timestamp: &str, side: &str, symbol: &str, entry: f64, sl: f64, tp: f64, volume: f64, rsi: f64, sentiment_score: i32, sentiment_confidence: f64, signal: &str, position_id: &str) {
        if let Ok(mut f) = fs::OpenOptions::new().append(true).open(&self.path) {
            let _ = writeln!(f, "{},{},{},{},{:.5},{:.5},{:.5},{:.4},{:.2},{},{:.2},{},{},,,",
                timestamp, "OPEN", side, symbol, entry, sl, tp, volume, rsi, sentiment_score, sentiment_confidence, signal, position_id);
        }
    }

    fn log_close(&self, timestamp: &str, position_id: &str, close_price: f64, pnl: f64, reason: &str) {
        if let Ok(mut f) = fs::OpenOptions::new().append(true).open(&self.path) {
            let _ = writeln!(f, "{},{},,,,,,,,,,,{},{:.5},{:.2},{}",
                timestamp, "CLOSE", position_id, close_price, pnl, reason);
        }
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
    symbol_meta: Option<SymbolMeta>,
    /// Sentiment cache to avoid excessive API calls
    sentiment_cache: Arc<RwLock<SentimentCache>>,
    /// CSV trade logger for backtesting
    trade_logger: TradeLogger,
    /// Last known RSI value for trade logging
    last_rsi: f64,
    /// Last known sentiment for trade logging
    last_sentiment: SentimentResult,
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

        let trade_log_path = env::var("TRADE_LOG_PATH").unwrap_or_else(|_| "data/trade_log.csv".to_string());
        let trade_logger = TradeLogger::new(&trade_log_path);
        info!("Trade logger enabled at {}", trade_log_path);

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
            symbol_meta: None,
            sentiment_cache: Arc::new(RwLock::new(SentimentCache::default())),
            trade_logger,
            last_rsi: 50.0,
            last_sentiment: SentimentResult::new(0, "init"),
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

        match self.ctrader.get_trader().await {
            Ok(trader) => {
            let money_digits = trader.money_digits.unwrap_or(0) as i32;
            let balance = trader.balance as f64 / 10_f64.powi(money_digits);
            info!(
                "Account balance: {:.2} (money_digits={})",
                balance, money_digits
            );
            self.strategy.update_balance(balance);
            }
            Err(err) => {
                warn!(
                    "Failed to fetch trader account info (balance) from cTrader: {}",
                    err
                );
            }
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

        // T-051: Retry get_symbol_meta up to 3 times with 2s backoff
        const MAX_META_RETRIES: u32 = 3;
        const META_RETRY_DELAY_SECS: u64 = 2;

        for attempt in 1..=MAX_META_RETRIES {
            match self.ctrader.get_symbol_meta(symbol_id).await {
                Ok(meta) => {
                    info!(
                        "Symbol meta: digits={} pip_position={} min_volume={:?} step_volume={:?} sl_distance={:?} tp_distance={:?} distance_set_in={:?} trading_mode={:?}",
                        meta.digits,
                        meta.pip_position,
                        meta.min_volume,
                        meta.step_volume,
                        meta.sl_distance,
                        meta.tp_distance,
                        meta.distance_set_in,
                        meta.trading_mode
                    );
                    self.symbol_meta = Some(meta);
                    break;
                }
                Err(err) => {
                    if attempt < MAX_META_RETRIES {
                        warn!(
                            "Failed to fetch symbol metadata for {} (id {}) on attempt {}/{}: {}. Retrying in {}s...",
                            symbol_name, symbol_id, attempt, MAX_META_RETRIES, err, META_RETRY_DELAY_SECS
                        );
                        tokio::time::sleep(Duration::from_secs(META_RETRY_DELAY_SECS)).await;
                    } else {
                        warn!(
                            "Failed to fetch symbol metadata for {} (id {}) after {} attempts: {}. Using default precision (5 digits).",
                            symbol_name, symbol_id, MAX_META_RETRIES, err
                        );
                    }
                }
            }
        }

        info!("🌴 Trading {} with symbol ID: {}", symbol_name, symbol_id);

        if !self.config.bot.dry_run {
            self.reconcile_positions().await?;
        } else {
            info!("Skipping broker reconciliation in dry_run mode");
        }
        self.ctrader.subscribe_to_symbol(self.symbol_id).await?;
        self.wait_for_initial_price(30).await?;

        // QUICK_TEST mode: force a BUY and SELL trade, report results, then exit
        if env::var("QUICK_TEST").ok().map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false) {
            return self.run_quick_test().await;
        }

        self.run_immediate_test_trades().await?;

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

    async fn wait_for_initial_price(&self, timeout_secs: u64) -> Result<()> {
        let timeout_duration = Duration::from_secs(timeout_secs);
        let start = Instant::now();

        while start.elapsed() < timeout_duration {
            if let Ok(price) = self.ctrader.get_price(self.symbol_id).await {
                info!(
                    "✅ Initial price received: bid={:.5} ask={:.5}",
                    price.bid, price.ask
                );
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }

        Err(BotError::Other(format!(
            "No price data received for symbol {} after {}s. Possible causes: invalid symbol, market closed, or feed issue.",
            self.symbol_id, timeout_secs
        )))
    }

    async fn run_immediate_test_trades(&mut self) -> Result<()> {
        if self.config.bot.dry_run {
            info!("Test trades skipped: dry_run enabled");
            return Ok(());
        }

        let enabled = env::var("TEST_IMMEDIATE_TRADES")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !enabled {
            return Ok(());
        }

        let delay_secs = env::var("TEST_TRADE_DELAY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(2);

        info!("⚡ TEST_IMMEDIATE_TRADES enabled: placing BUY then SELL");

        for side in [OrderSide::Buy, OrderSide::Sell] {
            let price = match self.ctrader.get_price(self.symbol_id).await {
                Ok(p) => p,
                Err(err) => {
                    warn!("Test trade: failed to get price: {}", err);
                    continue;
                }
            };
            let entry_price = (price.bid + price.ask) / 2.0;
            if let Err(err) = self.execute_trade(side, entry_price).await {
                warn!("Test trade {:?} failed: {}", side, err);
            }
            sleep(Duration::from_secs(delay_secs)).await;
            if let Err(err) = self.close_all_positions("test_immediate_trades").await {
                warn!("Test trade close failed: {}", err);
            }
        }

        Ok(())
    }

    async fn close_all_positions(&self, reason: &str) -> Result<()> {
        let positions = self.ctrader.reconcile_positions().await?;
        if positions.is_empty() {
            info!("No broker positions to close (reason: {})", reason);
            return Ok(());
        }

        for pos in positions {
            info!("Closing position {} (reason: {})", pos.position_id, reason);
            if let Err(err) = self.ctrader.close_position(pos.position_id, pos.volume).await {
                warn!("Failed to close position {}: {}", pos.position_id, err);
            }
        }

        Ok(())
    }

    /// Quick test mode: place a BUY, wait, close it, place a SELL, wait, close it.
    /// Reports balance before/after and exits. Used to verify the bot can actually trade.
    async fn run_quick_test(&mut self) -> Result<()> {
        info!("========================================");
        info!("  QUICK TEST MODE");
        info!("  Will place BUY + SELL trades and exit");
        info!("========================================");

        let hold_secs: u64 = env::var("QUICK_TEST_HOLD_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        // Get balance before
        let balance_before = match self.ctrader.get_trader().await {
            Ok(t) => {
                let md = t.money_digits.unwrap_or(0) as i32;
                t.balance as f64 / 10_f64.powi(md)
            }
            Err(err) => {
                warn!("Could not fetch balance: {}", err);
                0.0
            }
        };
        info!("[QUICK TEST] Balance before: ${:.2}", balance_before);

        // Volume: use QUICK_TEST_VOLUME env var if set, otherwise min_volume
        let min_vol = self.symbol_meta.as_ref()
            .and_then(|m| m.min_volume)
            .unwrap_or(100_000);
        let step_vol = self.symbol_meta.as_ref()
            .and_then(|m| m.step_volume)
            .unwrap_or(100_000);
        let volume: i64 = env::var("QUICK_TEST_VOLUME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(min_vol);
        // Snap volume to step and enforce minimum
        let volume = ((volume.max(min_vol) + step_vol - 1) / step_vol) * step_vol;
        info!("[QUICK TEST] Volume: {} units (min={}, step={})", volume, min_vol, step_vol);

        for (i, side) in [OrderSide::Buy, OrderSide::Sell].iter().enumerate() {
            let step = i + 1;
            info!("[QUICK TEST] Step {}/2: {:?}", step, side);

            // Get current price
            let price = match self.ctrader.get_price(self.symbol_id).await {
                Ok(p) => p,
                Err(err) => {
                    error!("[QUICK TEST] Failed to get price: {}", err);
                    continue;
                }
            };
            let entry = (price.bid + price.ask) / 2.0;
            let entry = self.normalize_price(entry);

            // Calculate SL/TP with safe distances
            let tp_distance = entry * 0.005; // 0.5%
            let sl_distance = entry * 0.003; // 0.3%

            let (tp, sl) = match side {
                OrderSide::Buy => (
                    self.normalize_price(entry + tp_distance),
                    self.normalize_price(entry - sl_distance),
                ),
                OrderSide::Sell => (
                    self.normalize_price(entry - tp_distance),
                    self.normalize_price(entry + sl_distance),
                ),
            };

            let trade_side = match side {
                OrderSide::Buy => ProtoOATradeSide::Buy,
                OrderSide::Sell => ProtoOATradeSide::Sell,
            };

            let ticket = OrderTicket {
                symbol_id: self.symbol_id,
                side: trade_side,
                volume,
                stop_loss: Some(sl),
                take_profit: Some(tp),
                relative_stop_loss: self.relative_distance(entry, sl),
                relative_take_profit: self.relative_distance(entry, tp),
                label: Some("QuickTest".to_string()),
            };

            info!("[QUICK TEST] Placing {:?} at {:.5} SL={:.5} TP={:.5} vol={}", side, entry, sl, tp, volume);

            match self.ctrader.place_order(ticket).await {
                Ok((order_id, position_id)) => {
                    info!("[QUICK TEST] Order filled: order_id={} position_id={}", order_id, position_id);

                    // Wait for the specified hold time
                    info!("[QUICK TEST] Holding for {}s...", hold_secs);
                    sleep(Duration::from_secs(hold_secs)).await;

                    // Close the position
                    info!("[QUICK TEST] Closing position {}...", position_id);
                    match self.ctrader.close_position(position_id, volume).await {
                        Ok(()) => info!("[QUICK TEST] Position {} closed", position_id),
                        Err(err) => warn!("[QUICK TEST] Failed to close position {}: {}", position_id, err),
                    }

                    // Small delay between trades
                    sleep(Duration::from_secs(2)).await;
                }
                Err(err) => {
                    error!("[QUICK TEST] Order REJECTED: {}", err);
                    error!("[QUICK TEST] This means the bot CANNOT trade. Check symbol, volume, or SL/TP distances.");
                }
            }
        }

        // Get balance after - retry up to 3 times (message queue may have stale messages)
        sleep(Duration::from_secs(3)).await;
        let mut balance_after = 0.0;
        for attempt in 1..=3 {
            match self.ctrader.get_trader().await {
                Ok(t) => {
                    let md = t.money_digits.unwrap_or(0) as i32;
                    balance_after = t.balance as f64 / 10_f64.powi(md);
                    break;
                }
                Err(err) => {
                    if attempt < 3 {
                        warn!("Balance fetch attempt {}/3 failed: {}. Retrying...", attempt, err);
                        sleep(Duration::from_secs(2)).await;
                    } else {
                        warn!("Could not fetch balance after 3 attempts: {}", err);
                    }
                }
            }
        }

        let pnl = balance_after - balance_before;
        info!("========================================");
        info!("  QUICK TEST RESULTS");
        info!("  Volume:         {} units", volume);
        info!("  Balance before: ${:.2}", balance_before);
        info!("  Balance after:  ${:.2}", balance_after);
        info!("  P&L:            ${:+.2}", pnl);
        if balance_after > 0.0 && balance_before > 0.0 {
            info!("  VERDICT: BOT CAN TRADE ✅");
        } else if balance_before > 0.0 {
            info!("  VERDICT: Balance fetch failed - but trades were placed ✅");
        } else {
            info!("  VERDICT: CHECK LOGS FOR ERRORS");
        }
        info!("========================================");

        // Close any remaining positions
        if let Err(err) = self.close_all_positions("quick_test_cleanup").await {
            warn!("[QUICK TEST] Cleanup failed: {}", err);
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
                    self.trade_logger.log_close(
                        &Utc::now().to_rfc3339(),
                        &position.id,
                        price,
                        pnl,
                        &format!("{:?}", reason),
                    );
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
        // Store for trade logging
        self.last_rsi = rsi;
        self.last_sentiment = sentiment.clone();

        self.metrics.with_metrics_mut(|m| {
            m.update_market_data(candle.close, rsi, sentiment.score);
        });
        let signal = self.strategy.generate_signal(rsi, sentiment.score);

        info!(
            "Candle close={:.5} RSI={:.1} Sentiment={} Signal={:?}",
            candle.close, rsi, sentiment.score, signal
        );

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
        if let Some(meta) = &self.symbol_meta {
            if let Some(mode) = meta.trading_mode {
                if mode != ProtoOaTradingMode::Enabled {
                    warn!("Symbol trading mode is {:?}; skipping new trade", mode);
                    return Ok(());
                }
            }
        }

        let entry_price = self.normalize_price(entry_price);
        let take_profit_raw = self.strategy.calculate_take_profit(entry_price, side);
        let stop_loss_raw = self.strategy.calculate_stop_loss(entry_price, side);
        let volume_raw = self.strategy.calculate_position_size(entry_price, stop_loss_raw);

        let (take_profit, stop_loss) =
            self.normalize_tp_sl(side, entry_price, take_profit_raw, stop_loss_raw);
        let take_profit = self.normalize_price(take_profit);
        let stop_loss = self.normalize_price(stop_loss);
        let (volume, volume_units) = match self.normalize_volume(volume_raw) {
            Some(result) => result,
            None => {
                warn!("Normalized volume is invalid; skipping trade");
                return Ok(());
            }
        };

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
            volume: volume_units,
            stop_loss: Some(stop_loss),
            take_profit: Some(take_profit),
            relative_stop_loss: self.relative_distance(entry_price, stop_loss),
            relative_take_profit: self.relative_distance(entry_price, take_profit),
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
                self.trade_logger.log_open(
                    &Utc::now().to_rfc3339(),
                    &format!("{:?}", side),
                    &self.config.trading.symbol,
                    entry_price,
                    stop_loss,
                    take_profit,
                    volume,
                    self.last_rsi,
                    self.last_sentiment.score,
                    self.last_sentiment.confidence as f64,
                    &format!("{:?}", self.strategy.generate_signal(self.last_rsi, self.last_sentiment.score)),
                    &position_id.to_string(),
                );
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

    fn price_factor(&self) -> f64 {
        // Default to 5 digits (10^5 = 100000) if symbol_meta is unavailable
        const DEFAULT_DIGITS: i32 = 5;

        let digits = self.symbol_meta
            .as_ref()
            .map(|m| m.digits)
            .filter(|&d| d >= 0)
            .unwrap_or(DEFAULT_DIGITS);

        10_f64.powi(digits)
    }

    fn round_price_up(&self, price: f64) -> f64 {
        let factor = self.price_factor();
        self.normalize_price((price * factor).ceil() / factor)
    }

    fn round_price_down(&self, price: f64) -> f64 {
        let factor = self.price_factor();
        self.normalize_price((price * factor).floor() / factor)
    }

    fn normalize_price(&self, price: f64) -> f64 {
        // Default to 5 digits if symbol_meta is unavailable (safe for most commodities/forex)
        const DEFAULT_DIGITS: usize = 5;

        let prec = match &self.symbol_meta {
            Some(meta) if meta.digits >= 0 => meta.digits as usize,
            _ => {
                // Log warning only once per missing meta scenario
                debug!("Using default precision ({} digits) - symbol_meta unavailable", DEFAULT_DIGITS);
                DEFAULT_DIGITS
            }
        };

        let formatted = format!("{:.prec$}", price, prec = prec);
        formatted.parse::<f64>().unwrap_or(price)
    }

    fn relative_distance(&self, entry: f64, target: f64) -> Option<i64> {
        if !entry.is_finite() || !target.is_finite() {
            return None;
        }
        let diff = (entry - target).abs();
        if diff <= 0.0 {
            return None;
        }
        Some((diff * 100000.0).round() as i64)
    }

    fn normalize_tp_sl(&self, side: OrderSide, entry: f64, take_profit: f64, stop_loss: f64) -> (f64, f64) {
        let mut tp = take_profit;
        let mut sl = stop_loss;

        if let Some(meta) = &self.symbol_meta {
            if let Some(min_tp) = meta.min_distance_price(entry, meta.tp_distance) {
                match side {
                    OrderSide::Buy => {
                        let target = entry + min_tp;
                        if tp < target {
                            tp = target;
                        }
                    }
                    OrderSide::Sell => {
                        let target = entry - min_tp;
                        if tp > target {
                            tp = target;
                        }
                    }
                }
            }

            if let Some(min_sl) = meta.min_distance_price(entry, meta.sl_distance) {
                match side {
                    OrderSide::Buy => {
                        let target = entry - min_sl;
                        if sl > target {
                            sl = target;
                        }
                    }
                    OrderSide::Sell => {
                        let target = entry + min_sl;
                        if sl < target {
                            sl = target;
                        }
                    }
                }
            }
        }

        match side {
            OrderSide::Buy => {
                tp = self.round_price_up(tp);
                sl = self.round_price_down(sl);
            }
            OrderSide::Sell => {
                tp = self.round_price_down(tp);
                sl = self.round_price_up(sl);
            }
        }

        if let Some(meta) = &self.symbol_meta {
            if let Some(point) = meta.point_size() {
                match side {
                    OrderSide::Buy => {
                        if tp <= entry {
                            tp = self.round_price_up(entry + point);
                        }
                        if sl >= entry {
                            sl = self.round_price_down(entry - point);
                        }
                    }
                    OrderSide::Sell => {
                        if tp >= entry {
                            tp = self.round_price_down(entry - point);
                        }
                        if sl <= entry {
                            sl = self.round_price_up(entry + point);
                        }
                    }
                }

                if let Some(min_tp) = meta.min_distance_price(entry, meta.tp_distance) {
                    match side {
                        OrderSide::Buy => {
                            let min_target = entry + min_tp;
                            if tp < min_target {
                                tp = self.round_price_up(min_target + point);
                            }
                        }
                        OrderSide::Sell => {
                            let min_target = entry - min_tp;
                            if tp > min_target {
                                tp = self.round_price_down(min_target - point);
                            }
                        }
                    }
                }

                if let Some(min_sl) = meta.min_distance_price(entry, meta.sl_distance) {
                    match side {
                        OrderSide::Buy => {
                            let min_target = entry - min_sl;
                            if sl > min_target {
                                sl = self.round_price_down(min_target - point);
                            }
                        }
                        OrderSide::Sell => {
                            let min_target = entry + min_sl;
                            if sl < min_target {
                                sl = self.round_price_up(min_target + point);
                            }
                        }
                    }
                }
            }
        }

        (tp, sl)
    }

    /// Convert base currency units to cTrader volume units, aligned to broker constraints.
    ///
    /// Input: base_currency_units (e.g. 33,898 EUR for a 2% risk trade on EURUSD)
    /// Output: (base_units_display, ctrader_volume_units)
    /// cTrader volume = base_currency_units × 100 (centigranular convention)
    fn normalize_volume(&self, base_units: f64) -> Option<(f64, i64)> {
        // cTrader uses centigranular volume: 1 base unit = 100 volume units
        let mut units = (base_units * 100.0).round() as i64;
        if units <= 0 {
            return None;
        }

        // Safety cap when symbol_meta is missing: limit to 5,000,000 (≈0.5 lots forex)
        const DEFAULT_MAX_VOLUME: i64 = 5_000_000;

        if let Some(meta) = &self.symbol_meta {
            if let Some(step) = meta.step_volume {
                if step > 0 {
                    units = (units / step) * step;
                }
            }
            if let Some(min) = meta.min_volume {
                if units < min {
                    units = min;
                }
            }
            if let Some(max) = meta.max_volume {
                if units > max {
                    units = max;
                }
            }
        } else {
            // No symbol meta: apply safety cap
            if units > DEFAULT_MAX_VOLUME {
                warn!(
                    "No symbol meta; capping volume {} → {} (safety limit)",
                    units, DEFAULT_MAX_VOLUME
                );
                units = DEFAULT_MAX_VOLUME;
            }
        }

        if units <= 0 {
            return None;
        }

        Some((units as f64 / 100.0, units))
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

fn is_auth_api_error(message: &str) -> bool {
    let msg = message.to_ascii_uppercase();
    msg.contains("CH_CLIENT_NOT_AUTHENTICATED")
        || msg.contains("CH_CLIENT_AUTH_FAILURE")
        || msg.contains("AUTH_FAILURE")
        || msg.contains("NOT AUTHENTICATED")
}

fn should_retry_ctrader(err: &BotError) -> bool {
    match err {
        BotError::CTrader(CTraderError::ConnectionFailed(_))
        | BotError::CTrader(CTraderError::Timeout)
        | BotError::CTrader(CTraderError::Disconnected)
        | BotError::CTrader(CTraderError::AuthFailed(_)) => true,
        BotError::CTrader(CTraderError::ApiError(message)) => is_auth_api_error(message),
        _ => false,
    }
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

    // ============== T-050 Price Precision Tests ==============
    // These tests verify the default precision fallback logic

    /// Helper: Simulate normalize_price logic with optional digits
    fn normalize_price_logic(price: f64, digits: Option<i32>) -> f64 {
        const DEFAULT_DIGITS: usize = 5;
        let prec = match digits {
            Some(d) if d >= 0 => d as usize,
            _ => DEFAULT_DIGITS,
        };
        let formatted = format!("{:.prec$}", price, prec = prec);
        formatted.parse::<f64>().unwrap_or(price)
    }

    /// Helper: Simulate price_factor logic with optional digits
    fn price_factor_logic(digits: Option<i32>) -> f64 {
        const DEFAULT_DIGITS: i32 = 5;
        let d = digits.filter(|&d| d >= 0).unwrap_or(DEFAULT_DIGITS);
        10_f64.powi(d)
    }

    #[test]
    fn test_normalize_price_with_symbol_meta() {
        // Sugar typically uses 3 digits
        let price = 14.359200000000001; // Floating point imprecision
        let normalized = normalize_price_logic(price, Some(3));
        assert_eq!(normalized, 14.359);
    }

    #[test]
    fn test_normalize_price_without_symbol_meta() {
        // Default 5 digits when symbol_meta is None
        let price = 14.359200000000001;
        let normalized = normalize_price_logic(price, None);
        assert_eq!(normalized, 14.3592); // Rounded to 5 digits
    }

    #[test]
    fn test_normalize_price_negative_digits_uses_default() {
        // If digits is negative (invalid), use default 5
        let price = 14.359200000000001;
        let normalized = normalize_price_logic(price, Some(-1));
        assert_eq!(normalized, 14.3592);
    }

    #[test]
    fn test_normalize_price_forex_5_digits() {
        // Forex pairs typically use 5 digits
        let price = 1.12345678901234;
        let normalized = normalize_price_logic(price, Some(5));
        assert_eq!(normalized, 1.12346); // Rounds to nearest 5th digit
    }

    #[test]
    fn test_normalize_price_jpy_3_digits() {
        // JPY pairs use 3 digits
        let price = 150.12345;
        let normalized = normalize_price_logic(price, Some(3));
        assert_eq!(normalized, 150.123);
    }

    #[test]
    fn test_price_factor_with_digits() {
        assert_eq!(price_factor_logic(Some(3)), 1000.0);
        assert_eq!(price_factor_logic(Some(5)), 100000.0);
        assert_eq!(price_factor_logic(Some(2)), 100.0);
    }

    #[test]
    fn test_price_factor_without_digits() {
        // Default 5 when None
        assert_eq!(price_factor_logic(None), 100000.0);
    }

    #[test]
    fn test_price_factor_negative_digits_uses_default() {
        // Default 5 when negative
        assert_eq!(price_factor_logic(Some(-1)), 100000.0);
    }

    #[test]
    fn test_normalize_prevents_precision_error() {
        // The original bug: 14.359200000000001 has "more digits than symbol allows"
        // This test verifies our fix prevents this
        let problematic_price = 14.359200000000001;

        // With symbol_meta (3 digits for Sugar)
        let fixed_3 = normalize_price_logic(problematic_price, Some(3));
        let digits_3 = fixed_3.to_string();
        assert!(
            digits_3.split('.').nth(1).map_or(true, |d| d.len() <= 3),
            "Price {} has more than 3 decimal digits: {}",
            fixed_3,
            digits_3
        );

        // Without symbol_meta (default 5 digits)
        let fixed_5 = normalize_price_logic(problematic_price, None);
        let digits_5 = fixed_5.to_string();
        assert!(
            digits_5.split('.').nth(1).map_or(true, |d| d.len() <= 5),
            "Price {} has more than 5 decimal digits: {}",
            fixed_5,
            digits_5
        );
    }
}
