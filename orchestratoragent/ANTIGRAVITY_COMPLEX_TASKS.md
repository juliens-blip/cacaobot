# ANTIGRAVITY - Complex Tasks (APEX)

**Agent**: Antigravity via Claude Opus 4.5 Thinking
**Mode**: APEX Workflow (Tâches complexes)
**Status**: ASSIGNED
**Date**: 2026-01-20 12:20

---

## MISSION PRINCIPALE

Tu es Antigravity, expert en architectures complexes et optimisations avancées. Tu utilises APEX workflow pour les tâches multi-étapes complexes.

---

## TACHES COMPLEXES ASSIGNEES

### TASK-APEX-001: Advanced Strategy Engine (HIGH PRIORITY)

**Objectif**: Implémenter un moteur de stratégie multi-indicateurs avec machine learning

**Sous-tâches**:

1. **Multi-Indicator System**
   - Implémenter EMA crossover (9/21 periods)
   - Ajouter MACD indicator
   - Implémenter Bollinger Bands
   - Créer un score de confluence (combine RSI + EMA + MACD + BB)
   - Fichier: `src/modules/trading/indicators.rs`

2. **Adaptive Position Sizing**
   - Calculer ATR (Average True Range) pour volatilité
   - Dynamic position sizing basé sur ATR
   - Kelly Criterion pour taille optimale
   - Fichier: `src/modules/trading/position_sizing.rs` (nouveau)

3. **Time-Based Filters**
   - Détecter les heures de faible liquidité
   - Blacklist pendant news releases (API calendar)
   - Session filters (Asia/Europe/US)
   - Fichier: `src/modules/trading/time_filters.rs` (nouveau)

4. **Sentiment Confidence Scoring**
   - Parser Perplexity response pour extraire confidence level
   - Weighted sentiment: `score * confidence`
   - Ne trader que si confidence > 0.65
   - Fichier: `src/modules/scraper/sentiment.rs` (améliorer)

**Livrable**: Nouveau système de stratégie dans `src/modules/trading/advanced_strategy.rs`

---

### TASK-APEX-002: Real-Time Market Data Pipeline (MEDIUM PRIORITY)

**Objectif**: Architecture complète pour streaming real-time des données

**Sous-tâches**:

1. **Message Queue Architecture**
   - Channel MPSC pour price events
   - Broadcast channel pour market events
   - Event dispatcher pattern
   - Fichier: `src/modules/trading/event_system.rs` (nouveau)

2. **Candlestick Aggregator**
   - Agréger ticks en candles 1min/5min/15min
   - Rolling window avec VecDeque
   - Efficient storage avec time-series DB (optionnel)
   - Fichier: `src/modules/trading/candles.rs` (nouveau)

3. **Order Book Reconstruction**
   - Parser bid/ask updates de cTrader
   - Maintenir order book en mémoire
   - Calculer spread, depth, imbalance
   - Fichier: `src/modules/trading/orderbook.rs` (nouveau)

**Livrable**: Pipeline complet avec tests d'intégration

---

### TASK-APEX-003: Advanced Risk Management System (HIGH PRIORITY)

**Objectif**: Système de risk management enterprise-grade

**Sous-tâches**:

1. **Portfolio Risk Metrics**
   - Calcul Sharpe Ratio en temps réel
   - Sortino Ratio
   - Max Drawdown tracking
   - Value at Risk (VaR)
   - Fichier: `src/modules/monitoring/risk_metrics.rs` (nouveau)

2. **Circuit Breakers**
   - Daily loss limit avec graceful shutdown
   - Consecutive losses breaker (ex: 5 trades perdants = pause 1h)
   - Volatility spike detector (pause si spread > 3x normal)
   - Fichier: `src/modules/trading/circuit_breakers.rs` (nouveau)

3. **Correlation Analysis**
   - Track correlation entre trades
   - Éviter over-exposure
   - Position correlation matrix
   - Fichier: `src/modules/monitoring/correlation.rs` (nouveau)

**Livrable**: Module risk management complet + dashboard integration

---

### TASK-APEX-004: Backtesting Framework Evolution (MEDIUM PRIORITY)

**Objectif**: Transformer backtest.rs en framework robuste

**Sous-tâches**:

1. **Historical Data Loader**
   - Support CSV import (cTrader export format)
   - Support Parquet files
   - Data validation et cleaning
   - Fichier: `src/bin/backtest/data_loader.rs` (nouveau)

2. **Parameter Optimization**
   - Grid search pour hyperparameters
   - Genetic algorithm optimization
   - Walk-forward analysis
   - Fichier: `src/bin/backtest/optimizer.rs` (nouveau)

3. **Advanced Metrics**
   - Monte Carlo simulation
   - Profit factor breakdown
   - Trade distribution analysis
   - Equity curve visualization (ASCII art)
   - Fichier: `src/bin/backtest/analytics.rs` (nouveau)

**Livrable**: Backtesting framework avec CLI interactif

---

### TASK-APEX-005: Intelligent Trade Execution (HIGH PRIORITY)

**Objectif**: Smart order execution avec slippage minimization

**Sous-tâches**:

1. **TWAP/VWAP Execution**
   - Time-Weighted Average Price
   - Volume-Weighted Average Price
   - Split orders pour réduire slippage
   - Fichier: `src/modules/trading/execution.rs` (nouveau)

2. **Retry Logic avec Exponential Backoff**
   - Retry sur network errors
   - Exponential backoff
   - Circuit breaker integration
   - Fichier: `src/modules/trading/retry.rs` (nouveau)

3. **Order State Machine**
   - FSM pour lifecycle: Pending → Submitted → Filled → Closed
   - Gestion des partial fills
   - Reconciliation avec broker
   - Fichier: `src/modules/trading/order_fsm.rs` (nouveau)

**Livrable**: Execution engine robuste avec tests

---

## PRIORITES

1. **TASK-APEX-001** (Strategy) - Commence par ça
2. **TASK-APEX-003** (Risk Management)
3. **TASK-APEX-005** (Execution)
4. **TASK-APEX-002** (Data Pipeline)
5. **TASK-APEX-004** (Backtesting)

---

## WORKFLOW APEX

Pour chaque tâche:
1. **Analyze**: Lire code existant, comprendre architecture
2. **Plan**: Décomposer en sous-tâches, identifier dépendances
3. **Execute**: Implémenter avec tests
4. **Validate**: Vérifier compilation, tests, intégration
5. **Document**: Mettre à jour CLAUDE.md + ORCHESTRATION_STATUS.md

---

## FORMAT DE LIVRAISON

Après chaque tâche terminée:

```markdown
## TASK-APEX-XXX: [Nom]

**Status**: COMPLETED
**Duration**: Xh Ymin
**Files Modified/Created**:
- [fichier 1]
- [fichier 2]

**Key Changes**:
- [Changement 1]
- [Changement 2]

**Tests Added**: X tests
**Compilation Status**: ✅ PASS / ❌ FAIL (avec détails)

**Next Steps**: [Recommandations]
```

---

## NOTES IMPORTANTES

- **Utilise APEX agents** pour tâches multi-fichiers complexes
- **Utilise library agents** (backend-architect, fullstack-developer) pour implémentations spécifiques
- **Pas de unwrap() en production** - Toujours Result<T, E> avec context
- **Tests obligatoires** pour chaque fonction publique
- **Logs avec tracing** pour debugging
- **Documentation Rust** (///) pour fonctions publiques

---

**START WITH TASK-APEX-001 - Advanced Strategy Engine**

Read existing code, plan architecture, then implement multi-indicator system.

GO!
