# 🎯 ORCHESTRATOR SESSION ACTIVE
**Date**: 2026-01-26 14:45:00  
**Orchestrator**: AMP (remplace Claude)  
**Mode**: AUTONOME - Distribution automatique des TODOs

---

## 📊 ÉTAT ACTUEL

### ✅ Déjà Complété (Session précédente)
- TASK-PO-001 à TASK-PO-008: Architecture core + modules
- TASK-PO-010: main.rs + lib.rs
- TASK-PO-011: Strategy analysis

### ❌ BLOQUANTS Production Live (3 tâches)
1. **OAuth Production** - Auth DEMO uniquement
2. **TLS Verification** - Besoin test serveur LIVE
3. **Installation Rust** - cargo non disponible (⚠️ Bloquant système)

### ⚠️ PRIORITÉ HAUTE (2 tâches)
4. **Circuit Breakers** - Validation live requise
5. **Position Reconciliation** - Test connexions intermittentes

### 📊 OPTIMISATION (3 tâches)
6. **Backtest Tuning** - Profit factor 1.31 → 1.5+
7. **RSI Thresholds** - Parameter sweep
8. **Sentiment Cache** - Rate limits Perplexity

---

## 🚀 DISTRIBUTION DES TODOs

### TODO-BATCH-001 (BLOQUANTS + SÉCURITÉ)
**ID**: TODO-BATCH-001  
**Agents**: backend-architect + apex-workflow  
**Priorité**: CRITIQUE  
**Tâches**:
1. TLS Certificate Validation (test_tls_connection.rs)
2. OAuth Production Setup (documentation + code)
3. Circuit Breakers Live Validation (tests stress)

**Prompt Engineering**:
```
Agent @backend-architect, tu as 3 tâches CRITIQUES pour production live:

TÂCHE 1: TLS Certificate Validation 🔒
- Créer src/bin/test_tls_connection.rs
- Tester live.ctraderapi.com:5035 ET demo.ctraderapi.com:5035
- Vérifier certificats SSL/TLS avec rustls
- Documenter différences DEMO vs LIVE

TÂCHE 2: OAuth Production Setup 🔐
- Modifier src/modules/trading/ctrader.rs pour switch DEMO/LIVE
- Ajouter variables CTRADER_ENVIRONMENT=demo|live dans .env.example
- Documenter flux OAuth production dans README.md section "Production Deployment"
- Créer guide migration DEMO → LIVE

TÂCHE 3: Circuit Breakers avec @apex-workflow 🛡️
- Utiliser apex-workflow pour analyser src/modules/trading/strategy.rs
- Créer tests/circuit_breakers_stress_test.rs
- Tester: daily loss -5%, consecutive losses 3+, volatility spike
- Simuler scénarios avec backtest

SKILLS À UTILISER:
- @agents_library/backend-architect.md pour impl
- @agents_library/apex-workflow.md pour tâche 3
- @agents_library/explore-code.md pour comprendre code existant

REPORTING:
Quand terminé, ajouter dans CLAUDE.md:
---
### TODO-BATCH-001: COMPLETED
**Date**: 2026-01-26 HH:MM
**Agent**: backend-architect + apex-workflow
**Tâches**:
1. ✅ TLS Validation: [OK/FAIL] - Certificate: [VALID/INVALID]
2. ✅ OAuth Production: Documented in README.md L[XXX-YYY]
3. ✅ Circuit Breakers: [N] tests added, stress scenarios passing
---

IMPORTANT: Ne pas attendre entre les tâches, exécuter en séquence.
```

**Livrable attendu**:
- test_tls_connection.rs (nouveau binary)
- README.md updated (OAuth section)
- circuit_breakers_stress_test.rs (tests)
- Section dans CLAUDE.md

---

### TODO-BATCH-002 (ROBUSTESSE + OPTIMISATION)
**ID**: TODO-BATCH-002  
**Agents**: backend-architect + test-engineer  
**Priorité**: HAUTE  
**Tâches**:
1. Position Reconciliation System
2. Backtest Parameter Sweep
3. Sentiment Cache System

**Prompt Engineering**:
```
Agent @backend-architect + @test-engineer, vous avez 3 tâches d'optimisation:

TÂCHE 1: Position Reconciliation System 🔄
- Créer src/modules/trading/position_reconciliation.rs
- Implémenter cache local positions (HashMap<String, Position>)
- Mécanisme re-sync après reconnexion (compare local vs remote)
- Logs détaillés pour audit trail (avec timestamps)
- Tests connexions intermittentes: tests/position_reconciliation_test.rs

TÂCHE 2: Backtest Parameter Sweep 📊 (@test-engineer)
- Créer src/bin/backtest_optimizer.rs
- Grid search:
  * RSI buy: 20-35 (step 5)
  * RSI sell: 65-80 (step 5)
  * TP: 1.5%-3% (step 0.5%)
  * SL: 1%-2% (step 0.5%)
- Output CSV: backtest_results.csv avec colonnes [rsi_buy, rsi_sell, tp, sl, profit_factor, win_rate]
- Trouver combinaison avec profit_factor > 1.5

TÂCHE 3: Sentiment Cache System 🧠
- Créer src/modules/scraper/sentiment_cache.rs
- Cache in-memory: HashMap<String, (i32, Instant)> avec TTL 5min
- Fallback Twitter si Perplexity rate limited (429 error)
- Logger cache hits/misses avec tracing::info!
- Tests unitaires: expiration, hit/miss scenarios

SKILLS À UTILISER:
- @agents_library/backend-architect.md
- @agents_library/test-engineer.md
- @agents_library/explore-code.md pour patterns existants

REPORTING:
Quand terminé, ajouter dans CLAUDE.md:
---
### TODO-BATCH-002: COMPLETED
**Date**: 2026-01-26 HH:MM
**Agent**: backend-architect + test-engineer
**Tâches**:
1. ✅ Position Reconciliation: [N] tests passing, cache implemented
2. ✅ Backtest Optimizer: Best profit_factor=[X.XX], params=[details]
3. ✅ Sentiment Cache: Cache hit rate estimate [XX%], TTL=5min
---
```

**Livrable attendu**:
- position_reconciliation.rs + tests
- backtest_optimizer.rs + backtest_results.csv
- sentiment_cache.rs + tests
- Section dans CLAUDE.md

---

## 📋 PROTOCOLE INTER-LLM

### Communication
**Fichier central**: CLAUDE.md (section "🎯 ORCHESTRATION V3")

### Format de reporting (obligatoire)
Chaque agent doit ajouter dans CLAUDE.md:
```markdown
### TODO-BATCH-XXX: COMPLETED
**Date**: 2026-01-26 HH:MM
**Agent**: [nom agent]
**ID**: [TODO-BATCH-XXX]
**Durée**: [XX minutes]
**Tâches**:
1. ✅ [Tâche 1]: [résultat]
2. ✅ [Tâche 2]: [résultat]
3. ✅ [Tâche 3]: [résultat]

**Files Created**:
- [path/to/file1.rs]
- [path/to/file2.rs]

**Status**: DONE - Prêt pour nouvelle TODO
```

### Workflow automatique
```
Agent termine TODO-BATCH-XXX
         ↓
Documentation dans CLAUDE.md
         ↓
Orchestrator détecte COMPLETED
         ↓
Distribution TODO-BATCH-(XXX+1)
         ↓
Repeat
```

---

## 🎯 AGENTS LIBRARY - SKILLS DISPONIBLES

Agents à utiliser selon la tâche:
- **backend-architect**: Modules backend, API, Protobuf
- **test-engineer**: Tests unitaires, integration, optimization
- **apex-workflow**: Tâches complexes multi-étapes
- **explore-code**: Analyse codebase existante
- **debugger**: Debug et troubleshooting
- **code-reviewer**: Review final avant prod

Tous dans `/home/julien/Documents/palm-oil-bot/agents_library/`

---

## 📊 MÉTRIQUES DE SUCCÈS

| Critère | Cible | Status |
|---------|-------|--------|
| TODO-BATCH-001 | 3/3 tasks done | ⏳ PENDING |
| TODO-BATCH-002 | 3/3 tasks done | ⏳ PENDING |
| TLS Validation | LIVE server OK | ⏳ |
| OAuth Production | Documented | ⏳ |
| Circuit Breakers | Stress tested | ⏳ |
| Position Reconciliation | Implemented | ⏳ |
| Backtest Optimizer | PF > 1.5 | ⏳ |
| Sentiment Cache | TTL 5min | ⏳ |

---

## 🚦 STATUS LIVE

### TODO-BATCH-001 (CRITIQUE)
**Status**: ⏳ À DÉMARRER  
**Agent assigné**: backend-architect + apex-workflow  
**Deadline**: 60 minutes  

### TODO-BATCH-002 (HAUTE)
**Status**: ⏳ EN ATTENTE (après BATCH-001)  
**Agent assigné**: backend-architect + test-engineer  
**Deadline**: 90 minutes  

---

**NEXT ACTION**: Distribuer TODO-BATCH-001 à backend-architect via prompt ci-dessus
