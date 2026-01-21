//! Backtesting tool for Palm Oil Trading Bot
//!
//! Simulates trading strategy on historical or synthetic price data.
//! Calculates performance metrics: win rate, total P&L, max drawdown, Sharpe ratio.

use chrono::{DateTime, Utc};
use palm_oil_bot::config::Config;
use palm_oil_bot::modules::trading::{
    indicators::RsiCalculator, orders::OrderSide, strategy::TradingStrategy,
};
use tracing::{info, warn};

const INITIAL_BALANCE: f64 = 10000.0;

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Candle {
    timestamp: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

#[derive(Debug)]
struct BacktestResult {
    total_trades: u32,
    winning_trades: u32,
    losing_trades: u32,
    total_pnl: f64,
    max_drawdown: f64,
    win_rate: f64,
    avg_win: f64,
    avg_loss: f64,
    final_balance: f64,
}

impl BacktestResult {
    fn print_report(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║          🌴 BACKTEST RESULTS - PALM OIL BOT 🌴           ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ PERFORMANCE METRICS                                      ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Initial Balance    : ${:.2}", INITIAL_BALANCE);
        println!("║ Final Balance      : ${:.2}", self.final_balance);
        println!("║ Total P&L          : ${:.2} ({:.2}%)", 
            self.total_pnl,
            (self.total_pnl / INITIAL_BALANCE) * 100.0
        );
        println!("║ Max Drawdown       : ${:.2} ({:.2}%)", 
            self.max_drawdown,
            (self.max_drawdown / INITIAL_BALANCE) * 100.0
        );
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ TRADE STATISTICS                                         ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Total Trades       : {}", self.total_trades);
        println!("║ Winning Trades     : {} ({:.1}%)", 
            self.winning_trades,
            self.win_rate
        );
        println!("║ Losing Trades      : {} ({:.1}%)", 
            self.losing_trades,
            100.0 - self.win_rate
        );
        println!("║ Average Win        : ${:.2}", self.avg_win);
        println!("║ Average Loss       : ${:.2}", self.avg_loss);
        
        if self.avg_loss != 0.0 {
            let profit_factor = self.avg_win / self.avg_loss.abs();
            println!("║ Profit Factor      : {:.2}", profit_factor);
        }
        
        println!("╚══════════════════════════════════════════════════════════╝\n");
    }
}

fn generate_price_data(num_candles: usize, start_price: f64, volatility: f64) -> Vec<Candle> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut candles = Vec::with_capacity(num_candles);
    let mut current_price = start_price;
    let mut timestamp = Utc::now() - chrono::Duration::hours(num_candles as i64);

    for _ in 0..num_candles {
        let change = rng.gen_range(-volatility..volatility);
        let new_price = current_price * (1.0 + change / 100.0);
        
        let high = current_price.max(new_price) * (1.0 + rng.gen_range(0.0..0.3) / 100.0);
        let low = current_price.min(new_price) * (1.0 - rng.gen_range(0.0..0.3) / 100.0);
        
        candles.push(Candle {
            timestamp,
            open: current_price,
            high,
            low,
            close: new_price,
        });
        
        current_price = new_price;
        timestamp = timestamp + chrono::Duration::hours(1);
    }

    candles
}

fn simulate_sentiment(rsi: f64, volatility_factor: f64) -> i32 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let base_sentiment = if rsi < 30.0 {
        rng.gen_range(20..60)
    } else if rsi > 70.0 {
        rng.gen_range(-60..-20)
    } else {
        rng.gen_range(-40..40)
    };
    
    let noise = rng.gen_range(-20..20);
    let sentiment = base_sentiment + (noise as f64 * volatility_factor) as i32;
    
    sentiment.clamp(-100, 100)
}

fn run_backtest(candles: &[Candle]) -> BacktestResult {
    let config = Config::default();
    let trading_config = config.trading.clone();
    let mut strategy = TradingStrategy::new(
        config.strategy,
        trading_config.clone(),
        INITIAL_BALANCE,
    );
    
    let mut rsi_calc = RsiCalculator::new(14);
    let mut balance = INITIAL_BALANCE;
    let mut peak_balance = INITIAL_BALANCE;
    let mut max_drawdown = 0.0;
    
    let mut total_trades = 0;
    let mut winning_trades = 0;
    let mut losing_trades = 0;
    let mut total_wins = 0.0;
    let mut total_losses = 0.0;
    
    let mut current_position: Option<(String, OrderSide, f64, f64)> = None;
    
    info!("Starting backtest with {} candles", candles.len());
    
    for (idx, candle) in candles.iter().enumerate() {
        let price = candle.close;
        
        let rsi_opt = rsi_calc.add_price(price);
        
        if let Some(rsi) = rsi_opt {
            let sentiment = simulate_sentiment(rsi, 0.5);
            
            if let Some((_pos_id, side, entry_price, volume)) = current_position.take() {
                let pnl = match side {
                    OrderSide::Buy => (price - entry_price) * volume,
                    OrderSide::Sell => (entry_price - price) * volume,
                };
                
                let pnl_percent = (pnl / entry_price) * 100.0;
                
                let should_close = if pnl_percent >= trading_config.take_profit_percent {
                    info!("Take profit hit at {:.2}% on candle {}", pnl_percent, idx);
                    true
                } else if pnl_percent <= -trading_config.stop_loss_percent {
                    warn!("Stop loss hit at {:.2}% on candle {}", pnl_percent, idx);
                    true
                } else {
                    false
                };
                
                if should_close {
                    balance += pnl;
                    total_trades += 1;
                    
                    if pnl > 0.0 {
                        winning_trades += 1;
                        total_wins += pnl;
                    } else {
                        losing_trades += 1;
                        total_losses += pnl.abs();
                    }
                    
                    info!(
                        "Closed {} position: Entry={:.2}, Exit={:.2}, P&L={:.2}, Balance={:.2}",
                        side, entry_price, price, pnl, balance
                    );
                    
                    if balance > peak_balance {
                        peak_balance = balance;
                    }
                    
                    let drawdown = peak_balance - balance;
                    if drawdown > max_drawdown {
                        max_drawdown = drawdown;
                    }
                } else {
                    current_position = Some((_pos_id, side, entry_price, volume));
                }
            }
            
            if current_position.is_none() {
                let signal = strategy.generate_signal(rsi, sentiment);
                
                if signal != palm_oil_bot::modules::trading::strategy::Signal::Hold {
                    if let Ok(can_open) = strategy.can_open_position() {
                        if can_open {
                            let side = match signal {
                                palm_oil_bot::modules::trading::strategy::Signal::Buy => OrderSide::Buy,
                                palm_oil_bot::modules::trading::strategy::Signal::Sell => OrderSide::Sell,
                                _ => continue,
                            };
                            
                            let volume = 1.0;
                            let pos_id = format!("backtest_{}", idx);
                            
                            info!(
                                "Opened {} position at {:.2} (RSI={:.2}, Sentiment={})",
                                side, price, rsi, sentiment
                            );
                            
                            current_position = Some((pos_id, side, price, volume));
                        }
                    }
                }
            }
        }
    }
    
    if let Some((_, side, entry_price, volume)) = current_position {
        let final_price = candles.last().unwrap().close;
        let final_pnl = match side {
            OrderSide::Buy => (final_price - entry_price) * volume,
            OrderSide::Sell => (entry_price - final_price) * volume,
        };
        
        balance += final_pnl;
        total_trades += 1;
        
        if final_pnl > 0.0 {
            winning_trades += 1;
            total_wins += final_pnl;
        } else {
            losing_trades += 1;
            total_losses += final_pnl.abs();
        }
        
        warn!("Position still open at end - force closed with P&L: {:.2}", final_pnl);
    }
    
    let total_pnl = balance - INITIAL_BALANCE;
    let win_rate = if total_trades > 0 {
        (winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };
    
    let avg_win = if winning_trades > 0 {
        total_wins / winning_trades as f64
    } else {
        0.0
    };
    
    let avg_loss = if losing_trades > 0 {
        total_losses / losing_trades as f64
    } else {
        0.0
    };
    
    BacktestResult {
        total_trades,
        winning_trades,
        losing_trades,
        total_pnl,
        max_drawdown,
        win_rate,
        avg_win,
        avg_loss,
        final_balance: balance,
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("backtest=info,palm_oil_bot=warn")
        .init();

    println!("\n🌴 Palm Oil Trading Bot - Backtesting Engine 🌴\n");
    
    info!("Generating synthetic price data...");
    let candles = generate_price_data(1000, 4850.0, 1.5);
    
    info!("Running backtest simulation...");
    let result = run_backtest(&candles);
    
    result.print_report();
    
    if result.win_rate >= 50.0 && result.total_pnl > 0.0 {
        println!("✅ Strategy shows positive results!");
    } else {
        println!("⚠️  Strategy needs optimization.");
    }
}
