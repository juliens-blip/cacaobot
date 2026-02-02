# 🎯 ORCHESTRATION PLAN V3 - Phase Production

**Date**: 2026-01-24  
**Orchestrator**: AMP  
**Status**: ACTIVE  

---

## 📊 État Actuel (d'après rapports + image)

### ✅ Complété
- Architecture core (main, config, error)
- Modules trading (ctrader, strategy, indicators)
- Modules scraper (perplexity, sentiment)
- Modules monitoring (dashboard, metrics)
- Tests unitaires (53 tests)
- Backtest engine
- Dockerfile + Railway config

### ❌ BLOQUANTS Production Live
1. **OAuth Production** - Actuellement auth DEMO uniquement
2. **TLS Verification** - Besoin tester serveur LIVE cTrader
3. **Installation Rust** - Impossible compiler sans cargo

### ⚠️ PRIORITÉ HAUTE (Sécurité)
4. **Circuit Breakers** - Implémentés mais besoin validation live
5. **Position Reconciliation** - Test avec vraies connexions intermittentes

### 📊 OPTIMISATION (Nice-to-have)
6. **Backtest Tuning** - Profit factor 1.31 → target 1.5+
7. **RSI Thresholds** - Optimiser via parameter sweep
8. **Sentiment Cache** - Éviter rate limits Perplexity

---

## 🎯 TODOs ORCHESTRATOR (AMP) - Tâches Complexes

### TODO-ORC-001: Circuit Breakers Live Validation ⚠️
**Priorité**: CRITIQUE  
**Agent**: apex-workflow (tâche complexe)  
**Durée estimée**: 2 prompts

**Objectif**: Valider les circuit breakers en conditions réelles
**Fichier**: `src/modules/trading/circuit_breakers.rs`

**Actions**:
1. Utiliser agent **apex-workflow** pour analyser la logique actuelle
2. Créer tests d'intégration pour:
   - Daily loss limit (-5%)
   - Consecutive losses (3+)
   - Volatility spike detection
3. Simuler des scénarios de stress avec backtest

**Livrable**: Tests passing + rapport de validation

---

### TODO-ORC-002: Position Reconciliation System ⚠️
**Priorité**: CRITIQUE  
**Agent**: backend-architect  
**Durée estimée**: 3 prompts

**Objectif**: Gérer les déconnexions réseau et reconcilier les positions
**Fichier**: `src/modules/trading/position_reconciliation.rs`

**Actions**:
1. Implémenter système de cache local des positions
2. Créer mécanisme de re-sync après reconnexion
3. Ajouter logs détaillés pour audit trail
4. Tests avec connexions intermittentes simulées

**Livrable**: Module complet + tests

---

### TODO-ORC-003: OAuth Production Setup 🔐
**Priorité**: BLOQUANT  
**Agent**: backend-architect  
**Durée estimée**: 2 prompts

**Objectif**: Configurer l'authentification pour serveur LIVE cTrader
**Fichiers**: 
- `src/modules/trading/ctrader.rs`
- `.env.example`

**Actions**:
1. Documenter le flux OAuth production dans README
2. Ajouter variables d'environnement LIVE vs DEMO
3. Implémenter switch automatique selon env
4. Créer guide de migration DEMO → LIVE

**Livrable**: Documentation + code updated

---

## 📋 TODOs CODEX - Tâches Simples (2-3 prompts chacune)

### TODO-CODEX-001: Backtest Parameter Sweep 📊
**Priorité**: OPTIMISATION  
**Agent**: test-engineer  
**Durée estimée**: 2 prompts

**Objectif**: Optimiser RSI thresholds pour profit factor > 1.5
**Fichier**: `src/bin/backtest_optimizer.rs`

**Actions**:
1. Créer nouveau binary `backtest_optimizer.rs`
2. Implémenter grid search pour:
   - RSI buy threshold (20-35)
   - RSI sell threshold (65-80)
   - Take profit (1.5%-3%)
   - Stop loss (1%-2%)
3. Output: CSV avec résultats par combinaison

**Livrable**: Binary + rapport d'optimisation

**REPORTING**: Quand terminé, ajouter dans CLAUDE.md:
```
### TODO-CODEX-001: COMPLETED
**Date**: [DATE]
**Profit Factor**: [BEST_VALUE]
**Optimal Params**: RSI=[XX,YY], TP=[Z%], SL=[W%]
```

---

### TODO-CODEX-002: Sentiment Cache System 🧠
**Priorité**: OPTIMISATION  
**Agent**: backend-architect  
**Durée estimée**: 2 prompts

**Objectif**: Éviter rate limits Perplexity via cache Redis-like
**Fichier**: `src/modules/scraper/sentiment_cache.rs`

**Actions**:
1. Implémenter cache in-memory avec TTL (5 min)
2. Ajouter fallback sur Twitter si Perplexity rate limited
3. Logger les cache hits/misses pour monitoring
4. Tests unitaires pour expiration cache

**Livrable**: Module + tests

**REPORTING**: Quand terminé, ajouter dans CLAUDE.md:
```
### TODO-CODEX-002: COMPLETED
**Date**: [DATE]
**Cache Hit Rate**: [XX%]
**Perplexity Calls Saved**: [YY%]
```

---

### TODO-CODEX-003: TLS Certificate Validation 🔒
**Priorité**: BLOQUANT  
**Agent**: backend-architect  
**Durée estimée**: 1 prompt

**Objectif**: Tester connexion TLS avec serveur LIVE cTrader
**Fichier**: `src/bin/test_tls_connection.rs`

**Actions**:
1. Créer binary pour tester:
   - live.ctraderapi.com:5035 (LIVE server)
   - demo.ctraderapi.com:5035 (DEMO server)
2. Vérifier certificats SSL/TLS
3. Tester handshake Protobuf
4. Documenter différences DEMO vs LIVE

**Livrable**: Binary + rapport de connexion

**REPORTING**: Quand terminé, ajouter dans CLAUDE.md:
```
### TODO-CODEX-003: COMPLETED
**Date**: [DATE]
**LIVE Server**: [OK/FAIL]
**Certificate**: [VALID/INVALID]
**Issues**: [DESCRIPTION]
```

---

## 🔄 Workflow d'Exécution

### Phase 1: Codex (Parallèle)
```
TODO-CODEX-001 ─┐
                ├─→ Codex execute en parallèle
TODO-CODEX-002 ─┤
                │
TODO-CODEX-003 ─┘
```

**Durée**: 1-2h max (3 tasks × 2 prompts)

### Phase 2: Orchestrator (Séquentiel après Codex)
```
TODO-ORC-003 (OAuth)
     ↓
TODO-ORC-001 (Circuit Breakers)
     ↓
TODO-ORC-002 (Position Reconciliation)
```

**Durée**: 2-3h (tâches complexes avec apex)

### Phase 3: Validation Finale
```
1. Compilation complète (cargo build)
2. Tests full suite (cargo test)
3. Backtest avec params optimisés
4. Review final par code-reviewer
```

---

## 📊 Métriques de Succès

| Critère | Cible | Status |
|---------|-------|--------|
| Compilation | 0 errors | ⏳ |
| Tests passing | 100% | ⏳ |
| Profit factor | >1.5 | ⏳ |
| Circuit breakers | Tested live | ⏳ |
| Position reconciliation | Functional | ⏳ |
| OAuth LIVE | Documented | ⏳ |
| TLS validation | OK | ⏳ |
| Cache hit rate | >80% | ⏳ |

---

## 🚨 Protocole de Communication

### Pour Codex:
**Quand tu termines une TODO-CODEX-XXX:**
1. Ajoute section dans CLAUDE.md avec header exact:
   ```markdown
   ### TODO-CODEX-XXX: COMPLETED
   **Date**: 2026-01-24 HH:MM
   **[Metrics specifiques]**
   ```
2. Commit les fichiers créés
3. Passe à la TODO suivante

### Pour Orchestrator (moi):
**Surveillance:**
- Checker CLAUDE.md toutes les 15 min
- Quand 3 TODO-CODEX complétées → commencer TODO-ORC

---

## 🎯 Ordre d'Exécution Optimal

**NOW (Codex - Parallèle):**
- TODO-CODEX-003 (TLS) ← BLOQUANT, rapide (1 prompt)
- TODO-CODEX-002 (Cache) ← Utile pour prod
- TODO-CODEX-001 (Backtest) ← Nice-to-have

**AFTER (Orchestrator - Séquentiel):**
- TODO-ORC-003 (OAuth) ← Dépend de TODO-CODEX-003
- TODO-ORC-001 (Circuit Breakers) ← Sécurité critique
- TODO-ORC-002 (Position Reconciliation) ← Robustesse

---

## 📝 Notes

- **Rust non installé**: Les binaries ne pourront pas être testés localement, mais la structure de code doit être complète et prête à compiler
- **Agents disponibles**: explore-code, apex-workflow, backend-architect, test-engineer, debugger, code-reviewer
- **Mémoire partagée**: CLAUDE.md est le point de synchronisation entre tous les agents

---

**Status**: ✅ Plan créé, prêt à distribuer les tâches  
**Next**: Codex commence TODO-CODEX-003 (le plus urgent)
