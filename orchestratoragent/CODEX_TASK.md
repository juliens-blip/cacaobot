# CODEX TASK - Palm Oil Bot

**Date**: 2026-01-21 18:30
**Priority**: HIGH
**Status**: ASSIGNED

## Task: TASK-CODEX-003 - Create Bot Main Loop

Créer le fichier principal du bot avec la boucle de trading.

### Fichier à créer: `src/bot.rs`

### Fonctionnalités requises:

```rust
pub struct TradingBot {
    strategy: TradingStrategy,
    ctrader: CTraderClient,
    candle_builder: CandleBuilder,
    rsi_calculator: RsiCalculator,
    event_channel: EventChannelHandle,
}

impl TradingBot {
    pub fn new(config: Config) -> Result<Self>;
    
    /// Main trading loop
    pub async fn run(&mut self) -> Result<()>;
    
    /// Process a single tick
    async fn process_tick(&mut self, tick: Tick) -> Result<()>;
    
    /// Check for position exits
    async fn check_exits(&mut self) -> Result<()>;
    
    /// Generate and execute signals
    async fn process_signal(&mut self, candle: &Candle) -> Result<()>;
}
```

### Logique principale:
1. Connecter à cTrader
2. Boucle infinie avec cycle_interval_secs:
   - Récupérer prix actuel
   - Agréger en bougies (CandleBuilder)
   - Quand bougie complète → calculer RSI
   - Générer signal (TradingStrategy)
   - Vérifier exits (TP/SL)
   - Exécuter ordres si circuit breakers OK
3. Gérer les événements (EventChannel)

### Intégration:
Modifier `src/lib.rs`:
```rust
pub mod bot;
pub use bot::TradingBot;
```

### Tests:
Créer `tests/bot_integration_test.rs` avec dry_run mode

---
**Assigned by**: AMP Orchestrator
