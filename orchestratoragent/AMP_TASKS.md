# AMP Worker - Tâches Assignées

**Agent**: AMP Worker v2
**Mode**: Implémentation directe (pas de sous-agents)
**Date**: 2026-01-20 12:35

---

## TASK-AMP-001: Enhanced Indicators Module ✅ EN COURS

**Objectif**: Ajouter indicateurs avancés (EMA, MACD, Bollinger Bands, ATR)

### Sous-tâches

#### 1. EMA (Exponential Moving Average)
```rust
// src/modules/trading/indicators.rs
pub struct EmaCalculator {
    period: usize,
    multiplier: f64,
    current_ema: Option<f64>,
}

impl EmaCalculator {
    pub fn new(period: usize) -> Self {
        let multiplier = 2.0 / (period as f64 + 1.0);
        Self { period, multiplier, current_ema: None }
    }
    
    pub fn update(&mut self, price: f64) -> Option<f64> {
        self.current_ema = Some(match self.current_ema {
            None => price,
            Some(prev) => (price - prev) * self.multiplier + prev,
        });
        self.current_ema
    }
}
```

#### 2. MACD (Moving Average Convergence Divergence)
```rust
pub struct MacdCalculator {
    fast_ema: EmaCalculator,
    slow_ema: EmaCalculator,
    signal_ema: EmaCalculator,
}

impl MacdCalculator {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            fast_ema: EmaCalculator::new(fast),
            slow_ema: EmaCalculator::new(slow),
            signal_ema: EmaCalculator::new(signal),
        }
    }
    
    pub fn update(&mut self, price: f64) -> Option<MacdValues> {
        let fast = self.fast_ema.update(price)?;
        let slow = self.slow_ema.update(price)?;
        let macd_line = fast - slow;
        let signal_line = self.signal_ema.update(macd_line)?;
        let histogram = macd_line - signal_line;
        
        Some(MacdValues { macd_line, signal_line, histogram })
    }
}
```

#### 3. Bollinger Bands
```rust
pub struct BollingerBands {
    period: usize,
    std_dev: f64,
    sma: SmaCalculator,
    prices: VecDeque<f64>,
}

impl BollingerBands {
    pub fn update(&mut self, price: f64) -> Option<BbValues> {
        self.prices.push_back(price);
        if self.prices.len() > self.period {
            self.prices.pop_front();
        }
        
        let middle = self.sma.update(price)?;
        let variance = self.calculate_variance(middle);
        let std = variance.sqrt();
        
        Some(BbValues {
            upper: middle + (self.std_dev * std),
            middle,
            lower: middle - (self.std_dev * std),
        })
    }
}
```

#### 4. ATR (Average True Range)
```rust
pub struct AtrCalculator {
    period: usize,
    prev_close: Option<f64>,
    atr_ema: EmaCalculator,
}

impl AtrCalculator {
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        let true_range = if let Some(prev) = self.prev_close {
            (high - low).max((high - prev).abs()).max((low - prev).abs())
        } else {
            high - low
        };
        
        self.prev_close = Some(close);
        self.atr_ema.update(true_range)
    }
}
```

**Status**: 🔄 À implémenter maintenant

---

## TASK-AMP-002: Circuit Breakers Implementation

**Objectif**: Système de circuit breakers enterprise-grade

### Fichier: `src/modules/trading/circuit_breakers.rs`

```rust
pub struct CircuitBreaker {
    daily_loss_limit: f64,
    consecutive_loss_limit: usize,
    volatility_spike_threshold: f64,
    
    consecutive_losses: usize,
    is_paused: bool,
    pause_until: Option<DateTime<Utc>>,
}

impl CircuitBreaker {
    pub fn check_daily_loss(&mut self, current_pnl: f64, starting_balance: f64) -> Result<()> {
        let loss_pct = (current_pnl / starting_balance).abs() * 100.0;
        if loss_pct >= self.daily_loss_limit {
            self.trigger_pause(Duration::hours(24), "Daily loss limit hit");
            return Err(BotError::CircuitBreakerTripped("Daily loss limit".into()).into());
        }
        Ok(())
    }
    
    pub fn check_consecutive_losses(&mut self, trade_result: TradeResult) -> Result<()> {
        match trade_result {
            TradeResult::Loss => {
                self.consecutive_losses += 1;
                if self.consecutive_losses >= self.consecutive_loss_limit {
                    self.trigger_pause(Duration::hours(1), "Consecutive losses");
                    return Err(BotError::CircuitBreakerTripped("Consecutive losses".into()).into());
                }
            }
            TradeResult::Win => self.consecutive_losses = 0,
            _ => {}
        }
        Ok(())
    }
    
    pub fn check_volatility_spike(&mut self, current_spread: f64, avg_spread: f64) -> Result<()> {
        if current_spread > avg_spread * self.volatility_spike_threshold {
            self.trigger_pause(Duration::minutes(15), "Volatility spike");
            return Err(BotError::CircuitBreakerTripped("Volatility spike".into()).into());
        }
        Ok(())
    }
}
```

**Status**: ⏳ Après TASK-AMP-001

---

## TASK-AMP-003: Webhook Notifications

**Objectif**: Notifier sur Telegram/Discord lors d'événements

### Fichier: `src/modules/notifications/mod.rs`

```rust
pub struct NotificationService {
    telegram_token: Option<String>,
    telegram_chat_id: Option<String>,
    discord_webhook: Option<String>,
}

impl NotificationService {
    pub async fn send_trade_alert(&self, position: &Position, action: &str) -> Result<()> {
        let message = format!(
            "🤖 *Palm Oil Bot*\n\n\
             Action: {}\n\
             Symbol: {}\n\
             Side: {}\n\
             Entry: {:.2}\n\
             Volume: {:.2}\n\
             P&L: {:.2}",
            action, position.symbol, position.side, 
            position.entry_price, position.volume, position.pnl
        );
        
        if let Some(token) = &self.telegram_token {
            self.send_telegram(&message, token).await?;
        }
        
        Ok(())
    }
}
```

**Status**: ⏳ Basse priorité

---

## PRIORITE D'EXECUTION

1. **NOW**: TASK-AMP-001 (Indicators) - 30 min
2. **NEXT**: TASK-AMP-002 (Circuit Breakers) - 20 min
3. **LATER**: TASK-AMP-003 (Notifications) - 15 min

---

**START: Implémentation TASK-AMP-001**
