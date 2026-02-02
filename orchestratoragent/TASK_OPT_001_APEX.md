# TASK-OPT-001: Backtest Parameter Optimization

**Assigné à**: Apex (agents_library/apex-workflow.md)  
**Priorité**: MOYENNE  
**ETA**: 45min  
**Complexité**: DIFFICILE

## Objectif

Optimiser paramètres stratégie trading pour profit factor > 1.5+

## Contexte Actuel

Backtest actuel (`src/bin/backtest.rs`):
- Profit factor: 1.31 (objectif: 1.5+)
- Win rate: 44.8% (objectif: >50%)
- Max drawdown: 6.63% (objectif: <5%)

## Fichier à Créer

`src/bin/backtest_optimizer.rs`

## Implémentation

### 1. Parameter Grid
```rust
struct ParamGrid {
    rsi_oversold: Vec<f64>,      // [20, 25, 30, 35, 40]
    rsi_overbought: Vec<f64>,    // [60, 65, 70, 75, 80]
    sentiment_threshold: Vec<i32>, // [20, 25, 30, 35, 40]
    take_profit: Vec<f64>,        // [1.5, 2.0, 2.5, 3.0]
    stop_loss: Vec<f64>,          // [1.0, 1.5, 2.0]
}
```

### 2. Optimization Algorithm

**Option A: Grid Search** (simple)
- Iterate all combinations
- Run backtest for each
- Track best params

**Option B: Genetic Algorithm** (avancé, optionnel)
- Population de 50 param sets
- Fitness = profit factor
- 100 generations

### 3. Metrics à Optimiser

Fonction objectif:
```rust
score = 0.4 * profit_factor + 0.3 * win_rate + 0.3 * (1 - drawdown_ratio)
```

### 4. Output

Export meilleurs paramètres → `best_params.json`:
```json
{
  "rsi_oversold": 28,
  "rsi_overbought": 72,
  "sentiment_threshold": 32,
  "take_profit_percent": 2.2,
  "stop_loss_percent": 1.3,
  "profit_factor": 1.68,
  "win_rate": 54.2,
  "max_drawdown": 4.1
}
```

## Utiliser

- `src/bin/backtest.rs` (backtest existant)
- Réutiliser `run_backtest()` function
- Modifier params avant chaque run

## Livrables

1. `src/bin/backtest_optimizer.rs` complet
2. Grid search implémenté
3. Export JSON meilleurs params
4. Rapport avec top 10 param sets
5. Documentation dans `orchestratoragent/APEX_OPT_001_RESPONSE.md`

## Commande

```bash
cargo run --bin backtest-optimizer --release
```

## Agent

Utilise **apex-workflow.md** (3 étapes: analyze, plan, implement)
