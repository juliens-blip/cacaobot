# AMP TASK - Palm Oil Bot

**Date**: 2026-01-21 13:00
**Priority**: HIGH
**Status**: ASSIGNED

## Task: TASK-AMP-002 - Implement Circuit Breakers

Créer le module de circuit breakers pour la gestion du risque.

### Fichier à créer: `src/modules/trading/circuit_breakers.rs`

### Fonctionnalités requises:

```rust
pub struct CircuitBreakers {
    daily_loss_limit: f64,      // -5% par défaut
    max_consecutive_losses: u32, // 3 par défaut
    volatility_threshold: f64,   // ATR multiplier

    // State
    daily_pnl: f64,
    consecutive_losses: u32,
    is_triggered: bool,
}

impl CircuitBreakers {
    pub fn new(config: CircuitBreakerConfig) -> Self;
    pub fn check_daily_loss(&mut self, pnl: f64) -> bool;
    pub fn record_trade_result(&mut self, won: bool);
    pub fn check_volatility(&self, atr: f64, avg_atr: f64) -> bool;
    pub fn is_trading_allowed(&self) -> bool;
    pub fn reset_daily(&mut self);
}
```

### Tests requis:
- Test daily loss trigger
- Test consecutive losses trigger
- Test volatility spike detection
- Test reset functionality

### Intégration:
Ajouter dans `src/modules/trading/mod.rs`:
```rust
pub mod circuit_breakers;
pub use circuit_breakers::CircuitBreakers;
```

---
**Assigned by**: Claude Orchestrator
