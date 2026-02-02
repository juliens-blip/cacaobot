# 📋 Next Tasks Queue - Palm Oil Bot

**Date**: 2026-01-21 18:52
**Status**: Waiting for current tasks completion

---

## 🔄 Currently Running

| LLM | Task | File | Status |
|-----|------|------|--------|
| Codex | bot.rs creation | src/bot.rs | ✅ DONE (needs validation) |
| Antigravity | Sentiment cache | src/bot.rs | 🔄 IN PROGRESS (compilation error) |

---

## ⏭️ Next Wave (Once Current Tasks Done)

### TASK-CODEX-004: Symbol ID Discovery
**Priority**: HAUTE
**File**: `src/modules/trading/ctrader.rs`
**Description**: Implémenter get_symbol_id() pour découvrir automatiquement FCPO symbol ID
**Prompt**:
```
Ajoute la méthode get_symbol_id() dans ctrader.rs. Elle doit: 1) Envoyer ProtoOASymbolsListReq, 2) Parser la réponse, 3) Trouver le symbol par nom "FCPO", 4) Retourner le symbol_id. Remplace ensuite DEFAULT_SYMBOL_ID dans bot.rs.
```

### TASK-ANTIGRAVITY-002: Orderbook Module
**Priority**: MOYENNE
**File**: `src/modules/trading/orderbook.rs`
**Description**: Créer module de reconstruction du carnet d'ordres
**Prompt**:
```
Crée orderbook.rs avec struct OrderBook. Méthodes: update_bid(), update_ask(), get_spread(), detect_support_resistance(). Intègre avec event_system pour recevoir les updates de prix.
```

### TASK-CODEX-005: Integration Tests
**Priority**: HAUTE
**File**: `tests/bot_integration_test.rs`
**Description**: Tests end-to-end du bot en mode dry_run
**Prompt**:
```
Crée tests/bot_integration_test.rs avec tests: 1) test_bot_initialization(), 2) test_process_tick_updates_candles(), 3) test_signal_generation(), 4) test_circuit_breaker_triggers(), 5) test_dry_run_mode(). Mock les réponses cTrader.
```

### TASK-AMP-005: Main Entry Point Integration
**Priority**: CRITIQUE
**File**: `src/main.rs`
**Description**: Intégrer TradingBot dans main.rs
**Prompt (pour moi)**:
```
Modifier main.rs pour utiliser TradingBot::new() et bot.run() au lieu du code actuel. Simplifier la boucle principale en déléguant à bot.run(). Garder le graceful shutdown avec Ctrl+C.
```

---

## 🎯 Final Sprint (Last 2 Tasks Before Deploy)

### TASK-ALL-001: Final Validation
- Tous: `cargo test` (tous les tests doivent passer)
- Codex: `cargo clippy` (0 warnings)
- AMP: `cargo build --release` (compilation production)

### TASK-AMP-006: Deployment
- Build Docker image
- Test dry-run 5 minutes
- Deploy to Railway
- Monitor logs

---

## 📊 Progress Tracker

| Module | Before | After | Target |
|--------|--------|-------|--------|
| Circuit Breakers | 0% | 100% | 100% |
| Candles | 0% | 100% | 100% |
| Event System | 0% | 100% | 100% |
| Risk Metrics | 0% | 100% | 100% |
| **Bot Loop** | 0% | 90% | 100% |
| **Sentiment Cache** | 0% | 80% | 100% |
| Symbol Discovery | 0% | 0% | 100% |
| Orderbook | 0% | 0% | 80% |
| Integration Tests | 0% | 0% | 100% |
| Main Integration | 0% | 0% | 100% |

**Overall**: 95% → **98%** → **100%** (target)

---

## ⏱️ Time Estimates

| Task | Estimate | Blocker |
|------|----------|---------|
| Finish sentiment cache | 5 min | - |
| Symbol ID discovery | 15 min | - |
| Integration tests | 20 min | bot.rs compiling |
| Main.rs integration | 10 min | bot.rs ready |
| Orderbook (optional) | 30 min | - |
| Final validation | 10 min | All tests pass |
| Deployment | 15 min | Build success |

**Total**: ~1h 45min to production-ready

---

**Next Action**: Wait for Antigravity to fix sentiment_cache compilation
