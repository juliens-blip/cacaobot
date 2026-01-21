//! Palm Oil Trading Bot - Main Entry Point
//!
//! Automated scalping bot for FCPO (Palm Oil CFDs) on Fusion Markets via cTrader.
//! Strategy: RSI (technical) + Sentiment (Perplexity API + Twitter)
//! Target: 2-3% daily returns with strict risk management

use palm_oil_bot::config::Config;
use palm_oil_bot::error::Result;
use palm_oil_bot::modules::monitoring::{Dashboard, MetricsHandle};
use palm_oil_bot::modules::scraper::{PerplexityClient, SentimentResult, TwitterScraper};
use palm_oil_bot::modules::trading::{
    CTraderClient, OrderSide, OrderTicket, RsiCalculator, Signal, TradingStrategy,
};
use palm_oil_bot::modules::trading::protobuf::ProtoOATradeSide;

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// FCPO Symbol ID (to be obtained from cTrader symbol list)
const FCPO_SYMBOL_ID: i64 = 1; // TODO: Get actual symbol ID via ProtoOASymbolsListReq

/// Main bot state
struct BotState {
    config: Config,
    ctrader: CTraderClient,
    perplexity: PerplexityClient,
    twitter: TwitterScraper,
    strategy: TradingStrategy,
    rsi: RsiCalculator,
    metrics: MetricsHandle,
    running: bool,
}

impl BotState {
    async fn new(config: Config) -> Result<Self> {
        let ctrader = CTraderClient::new(config.ctrader.clone());
        let perplexity = PerplexityClient::new(config.perplexity.clone());
        let twitter = TwitterScraper::new(config.kols.clone());
        let strategy = TradingStrategy::new(
            config.strategy.clone(),
            config.trading.clone(),
            10000.0, // Initial balance - will be updated from account
        );
        let rsi = RsiCalculator::new(config.strategy.rsi_period);
        let metrics = MetricsHandle::new(10000.0);

        Ok(Self {
            config,
            ctrader,
            perplexity,
            twitter,
            strategy,
            rsi,
            metrics,
            running: false,
        })
    }

    /// Initialize connections
    async fn initialize(&mut self) -> Result<()> {
        info!("Initializing bot...");

        // Connect to cTrader
        self.ctrader.connect().await?;
        info!("Connected to cTrader");

        // Authenticate
        self.ctrader.authenticate().await?;
        info!("Authenticated with cTrader");

        // Subscribe to FCPO prices
        self.ctrader.subscribe_to_symbol(FCPO_SYMBOL_ID).await?;
        info!("Subscribed to FCPO price feed");
        
        // Start continuous reader task
        self.ctrader.start_reader().await?;
        info!("cTrader reader task started");

        self.running = true;
        Ok(())
    }

    /// Get current sentiment (Perplexity with Twitter fallback)
    async fn get_sentiment(&self) -> SentimentResult {
        // Try Perplexity first
        match self.perplexity.get_market_sentiment().await {
            Ok(sentiment) => {
                debug!("Perplexity sentiment: {} ({:?})", sentiment.score, sentiment.sentiment_type);
                sentiment
            }
            Err(e) => {
                warn!("Perplexity failed, trying Twitter: {}", e);
                // Fallback to Twitter
                match self.twitter.get_sentiment().await {
                    Ok(sentiment) => {
                        debug!("Twitter sentiment: {} ({:?})", sentiment.score, sentiment.sentiment_type);
                        sentiment
                    }
                    Err(e) => {
                        error!("Both sentiment sources failed: {}", e);
                        // Return neutral sentiment with low confidence
                        SentimentResult::new(0, "fallback").with_confidence(0.1)
                    }
                }
            }
        }
    }

    /// Main trading cycle
    async fn trading_cycle(&mut self) -> Result<()> {
        // 1. Get current price
        let price = match self.ctrader.get_price(FCPO_SYMBOL_ID).await {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to get price: {}", e);
                return Ok(());
            }
        };

        let current_price = (price.bid + price.ask) / 2.0;
        debug!("FCPO Price: {:.2} (bid: {:.2}, ask: {:.2})", current_price, price.bid, price.ask);

        // 2. Update RSI
        let rsi_value = match self.rsi.add_price(current_price) {
            Some(rsi) => rsi,
            None => {
                debug!("Not enough data for RSI calculation");
                return Ok(());
            }
        };
        debug!("RSI: {:.2}", rsi_value);

        // 3. Get sentiment
        let sentiment = self.get_sentiment().await;
        debug!("Sentiment: {} ({:?})", sentiment.score, sentiment.sentiment_type);

        // 4. Update metrics
        self.metrics.with_metrics_mut(|m| {
            m.update_market_data(current_price, rsi_value, sentiment.score);
        });

        // 5. Check existing positions for exit
        let positions: Vec<_> = self.strategy.position_manager().open_positions().to_vec();
        for position in positions {
            if let Some(reason) = self.strategy.check_position_exit(&position, current_price) {
                info!("Closing position {} due to {:?}", position.id, reason);

                if !self.config.bot.dry_run {
                    // Close position via cTrader
                    let volume = (position.volume * 100.0) as i64; // Convert to cTrader units
                    if let Err(e) = self.ctrader.close_position(
                        position.id.parse().unwrap_or(0),
                        volume,
                    ).await {
                        error!("Failed to close position: {}", e);
                        continue;
                    }
                }

                // Record trade
                if let Some(pnl) = self.strategy.close_position(&position.id, current_price, reason) {
                    info!("Position {} closed with P&L: {:.2}", position.id, pnl);
                    self.metrics.with_metrics_mut(|m| {
                        m.close_trade(&position.id, current_price);
                        m.record_realized_pnl(pnl);
                    });
                }
            }
        }

        // 6. Check for new entry signals (if allowed)
        if self.strategy.can_open_position()? {
            let signal = self.strategy.generate_signal(rsi_value, sentiment.score);

            match signal {
                Signal::Buy => {
                    info!("BUY signal detected: RSI={:.2}, Sentiment={}", rsi_value, sentiment.score);
                    self.execute_trade(OrderSide::Buy, current_price).await?;
                }
                Signal::Sell => {
                    info!("SELL signal detected: RSI={:.2}, Sentiment={}", rsi_value, sentiment.score);
                    self.execute_trade(OrderSide::Sell, current_price).await?;
                }
                Signal::Hold => {
                    debug!("HOLD - no signal");
                }
            }
        }

        Ok(())
    }

    /// Execute a trade
    async fn execute_trade(&mut self, side: OrderSide, entry_price: f64) -> Result<()> {
        let tp = self.strategy.calculate_take_profit(entry_price, side);
        let sl = self.strategy.calculate_stop_loss(entry_price, side);
        let volume = self.strategy.calculate_position_size(entry_price, sl);

        info!(
            "Executing {} trade: entry={:.2}, TP={:.2}, SL={:.2}, volume={:.2}",
            side, entry_price, tp, sl, volume
        );

        if self.config.bot.dry_run {
            info!("DRY RUN - Trade not executed");
            return Ok(());
        }

        // Create order ticket
        let proto_side = match side {
            OrderSide::Buy => ProtoOATradeSide::Buy,
            OrderSide::Sell => ProtoOATradeSide::Sell,
        };

        let ticket = OrderTicket {
            symbol_id: FCPO_SYMBOL_ID,
            side: proto_side,
            volume: (volume * 100.0) as i64, // Convert to cTrader units
            stop_loss: Some(sl),
            take_profit: Some(tp),
            label: Some("PalmOilBot".to_string()),
        };

        // Place order
        match self.ctrader.place_order(ticket).await {
            Ok((order_id, position_id)) => {
                info!("Order placed successfully: order_id={} position_id={}", order_id, position_id);

                // Add position to strategy (use position_id as primary identifier)
                let position = palm_oil_bot::modules::trading::Position::new(
                    position_id.to_string(),
                    "FCPO",
                    side,
                    entry_price,
                    volume,
                ).with_take_profit(tp)
                 .with_stop_loss(sl);

                self.strategy.add_position(position);

                // Update metrics with position_id
                self.metrics.with_metrics_mut(|m| {
                    m.add_open_position(position_id.to_string(), side.to_string(), volume, entry_price);
                });
            }
            Err(e) => {
                error!("Failed to place order: {}", e);
            }
        }

        Ok(())
    }

    /// Shutdown bot
    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down bot...");
        self.running = false;
        self.ctrader.disconnect().await?;
        info!("Bot shutdown complete");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("palm_oil_bot=info".parse()?)
                .add_directive("reqwest=warn".parse()?)
        )
        .init();

    info!("========================================");
    info!("  Palm Oil Trading Bot v0.1.0");
    info!("  Symbol: FCPO (Palm Oil CFD)");
    info!("  Strategy: RSI + Sentiment Analysis");
    info!("========================================");

    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    info!("Configuration loaded:");
    info!("  Server: {}:{}", config.ctrader.server, config.ctrader.port);
    info!("  Account: {}", config.ctrader.account_id);
    info!("  Dry Run: {}", config.bot.dry_run);
    info!("  Cycle Interval: {}s", config.bot.cycle_interval_secs);

    // Initialize bot
    let mut bot = BotState::new(config.clone()).await?;

    // Initialize connections
    if let Err(e) = bot.initialize().await {
        error!("Failed to initialize bot: {}", e);
        return Err(e.into());
    }

    // Spawn dashboard in background (optional)
    let metrics_handle = bot.metrics.clone();
    let dashboard_handle = tokio::spawn(async move {
        if let Err(e) = run_dashboard(metrics_handle).await {
            warn!("Dashboard error: {}", e);
        }
    });

    // Main trading loop
    let mut trading_interval = interval(Duration::from_secs(config.bot.cycle_interval_secs));

    info!("Starting trading loop...");

    // Handle Ctrl+C
    let running = Arc::new(RwLock::new(true));
    let running_clone = running.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Received shutdown signal");
        *running_clone.write().await = false;
    });

    loop {
        trading_interval.tick().await;

        if !*running.read().await {
            break;
        }

        if let Err(e) = bot.trading_cycle().await {
            error!("Trading cycle error: {}", e);
        }
    }

    // Shutdown
    bot.shutdown().await?;
    dashboard_handle.abort();

    info!("Bot stopped. Goodbye!");
    Ok(())
}

/// Run the CLI dashboard
async fn run_dashboard(metrics: MetricsHandle) -> Result<()> {
    // Run dashboard in blocking task since it uses synchronous terminal I/O
    let result = tokio::task::spawn_blocking(move || {
        let mut dashboard = Dashboard::new(metrics)?;
        dashboard.run()
    })
    .await
    .map_err(|e| palm_oil_bot::BotError::Other(e.to_string()))?;

    result.map_err(|e| palm_oil_bot::BotError::Other(e.to_string()))
}
