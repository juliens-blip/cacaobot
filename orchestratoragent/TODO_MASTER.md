# 📋 TODO Master List - Palm Oil Bot

**Date**: 2026-01-21 18:35
**Orchestrator**: AMP
**Project Status**: 95% Complete

---

## 🔴 CRITIQUE (Bloquer le déploiement)

### 1. Bot Main Loop - TASK-CODEX-003
**Assigné**: Codex (ACTIVE)
**Fichier**: `src/bot.rs`
**Statut**: En cours
**Priorité**: CRITIQUE
- Créer la boucle principale de trading
- Intégrer candles, RSI, strategy, circuit breakers
- Gérer les événements (EventChannel)

### 2. Main Entry Point - TASK-AMP-004
**Assigné**: AMP
**Fichier**: `src/main.rs` (update)
**Priorité**: CRITIQUE
- Instancier TradingBot
- Remplacer le TODO: Get actual symbol ID
- Lancer bot.run() avec graceful shutdown

### 3. Sentiment Integration - TASK-ANTIGRAVITY-001
**Assigné**: Antigravity
**Fichier**: `src/modules/scraper/sentiment.rs`
**Priorité**: HAUTE
- Connecter Perplexity API au signal generation
- Calculer sentiment score en temps réel
- Intégrer dans strategy.generate_signal()

---

## 🟡 IMPORTANT (Améliorer la robustesse)

### 4. Symbol ID Discovery - TASK-CODEX-004
**Assigné**: Codex
**Fichier**: `src/modules/trading/ctrader.rs`
**Priorité**: HAUTE
- Implémenter ProtoOASymbolsListReq
- Trouver automatiquement FCPO symbol ID
- Supprimer hardcoded constant

### 5. Orderbook Module - TASK-ANTIGRAVITY-002
**Assigné**: Antigravity
**Fichier**: `src/modules/trading/orderbook.rs`
**Priorité**: MOYENNE
- Reconstruction du carnet d'ordres
- Détection de niveaux de support/résistance
- Intégration avec event_system

### 6. Integration Tests - TASK-CODEX-005
**Assigné**: Codex
**Fichier**: `tests/bot_integration_test.rs`
**Priorité**: HAUTE
- Tests end-to-end en dry_run mode
- Simulations de scénarios (profit, loss, circuit breaker)
- Mock cTrader responses

---

## 🟢 NICE-TO-HAVE (Optimisations)

### 7. Performance Profiling - TASK-CODEX-006
**Assigné**: Codex
**Priorité**: BASSE
- Benchmarks (Criterion)
- Profiling avec cargo flamegraph
- Optimiser hot paths

### 8. Advanced Backtesting - TASK-ANTIGRAVITY-003
**Assigné**: Antigravity
**Fichier**: `src/bin/backtest.rs`
**Priorité**: BASSE
- Importer données historiques FCPO
- Walk-forward testing
- Monte Carlo simulations

### 9. Dashboard Web - TASK-CODEX-007
**Assigné**: Codex
**Fichier**: `src/modules/monitoring/web_dashboard.rs`
**Priorité**: BASSE
- Serveur Axum pour dashboard temps réel
- WebSocket pour live updates
- Charts avec Plotters

---

## 📊 Progression par Module

| Module | Completion | Blockers |
|--------|-----------|----------|
| Core (main, config, error) | 95% | TASK-AMP-004 |
| Trading (ctrader, strategy, indicators) | 100% | - |
| Circuit Breakers | 100% | - |
| Risk Metrics | 100% | - |
| Event System | 100% | - |
| Candles | 100% | - |
| **Bot Loop** | 0% | **TASK-CODEX-003** |
| **Sentiment Integration** | 30% | **TASK-ANTIGRAVITY-001** |
| Orderbook | 0% | TASK-ANTIGRAVITY-002 |
| Testing | 70% | TASK-CODEX-005 |
| Monitoring | 85% | - |
| Backtesting | 70% | - |

**Overall**: 95% → Besoin de 3 tâches critiques pour déploiement

---

## 🎯 Plan d'Exécution (Prochaines 30 min)

### Parallèle 1 (0-15 min)
- **Codex**: Termine bot.rs (TASK-CODEX-003) ← EN COURS
- **Antigravity**: Intègre sentiment (TASK-ANTIGRAVITY-001)
- **AMP**: Update main.rs (TASK-AMP-004)

### Parallèle 2 (15-30 min)
- **Codex**: Symbol ID discovery (TASK-CODEX-004)
- **Codex**: Tests intégration (TASK-CODEX-005)
- **AMP**: Validation finale + cargo test

### Déploiement (30 min)
- Tous: cargo build --release
- AMP: docker build
- AMP: railway deploy

---

## ✅ Critères de Succès Déploiement

- [ ] bot.rs créé avec run() loop
- [ ] main.rs instancie TradingBot
- [ ] Sentiment intégré dans signals
- [ ] Tous les tests passent (>50)
- [ ] 0 warnings cargo clippy
- [ ] Dry-run fonctionne 5 min sans crash
- [ ] README à jour

---

**Next**: Distribution des tâches aux LLMs
