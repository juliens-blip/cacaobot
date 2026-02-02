# 🤖 ORCHESTRATION AUTONOME V4 - Antigravity + Codex

**Date**: 2026-01-26 17:00 CET  
**Orchestrator**: AMP (MODE AUTONOME)  
**LLMs Actifs**: Antigravity, Codex  
**Status**: 🔄 RUNNING

---

## 📋 QUEUE TODOs - DISTRIBUTION AUTOMATIQUE

### 🔴 TODO-AUTO-001: OAuth Production Setup
**ID**: AUTO-001  
**Agent**: Antigravity  
**Priorité**: CRITIQUE  
**Durée estimée**: 3 prompts  
**Status**: 🔄 DISPATCHED

**Prompt pour Antigravity**:
```
Tu dois implémenter OAuth Production pour cTrader LIVE server.

CONTEXTE:
- Projet: Palm Oil Trading Bot (Rust)
- Auth DEMO fonctionne déjà (demo.ctraderapi.com:5035)
- Besoin: OAuth 2.0 flow pour LIVE (live.ctraderapi.com:5035)

UTILISE LES AGENTS:
1. @agents_library/explore-code.md pour analyser src/modules/trading/oauth.rs
2. @agents_library/backend-architect.md pour implémenter le flow OAuth
3. @agents_library/test-code.md pour créer les tests

TÂCHES:
1. Analyser oauth.rs existant avec explore-code
2. Implémenter avec backend-architect:
   - Enum Environment { Demo, Live }
   - OAuth flow complet (access + refresh tokens)
   - Token persistence (JSON sécurisé)
   - Auto-refresh avant expiration
3. Tests avec test-code:
   - test_oauth_live_flow()
   - test_token_refresh()
   - test_token_persistence()

FICHIERS:
- src/modules/trading/oauth.rs (modifier)
- src/config.rs (ajouter LIVE vars)
- .env.example (documenter)
- tests/oauth_live_test.rs (créer)

REPORTING:
Quand terminé, ajoute dans CLAUDE.md:
```markdown
### TODO-AUTO-001: COMPLETED ✅
**Date**: 2026-01-26 HH:MM
**Agent**: Antigravity
**Durée**: XXm
**Fichiers modifiés**: oauth.rs, config.rs, .env.example
**Tests**: XX tests PASSING
**Notes**: [Problèmes rencontrés si applicable]
```

VALIDATION:
- cargo build --release (doit passer)
- cargo test oauth (doit passer)
```

---

### 🟡 TODO-AUTO-002: Circuit Breakers Live Testing
**ID**: AUTO-002  
**Agent**: Codex  
**Priorité**: HAUTE  
**Durée estimée**: 2 prompts  
**Status**: 🔄 DISPATCHED

**Prompt pour Codex**:
```
Tu dois créer tests d'intégration pour circuit breakers en conditions LIVE.

CONTEXTE:
- Circuit breakers déjà implémentés (src/modules/trading/circuit_breakers.rs)
- Besoin: validation avec scénarios réalistes de crash/volatilité

UTILISE LES AGENTS:
1. @agents_library/test-code.md pour créer les tests
2. @agents_library/debugger.md si tests échouent

TÂCHES:
1. Créer tests/circuit_breakers_stress_test.rs avec:
   - test_daily_loss_limit_triggers_at_threshold()
   - test_consecutive_losses_exact_threshold()
   - test_volatility_spike_detection()
   - test_circuit_breaker_reset()
   - test_multiple_triggers_simultaneously()

2. Scénarios stress:
   - 10 trades perdants consécutifs
   - Volatilité spike 5x ATR
   - Daily loss -10% (flash crash simulation)
   - Recovery cycle complet

3. Validation avec position_manager persistence

REPORTING:
Quand terminé, ajoute dans CLAUDE.md:
```markdown
### TODO-AUTO-002: COMPLETED ✅
**Date**: 2026-01-26 HH:MM
**Agent**: Codex
**Tests créés**: XX tests
**Scénarios validés**: Daily loss, Consecutive, Volatility
**Status**: ALL PASSING
```

VALIDATION:
- cargo test circuit_breakers_stress (doit passer)
```

---

### 🟢 TODO-AUTO-003: Position Reconciliation Network Tests
**ID**: AUTO-003  
**Agent**: Antigravity  
**Priorité**: HAUTE  
**Durée estimée**: 3 prompts  
**Status**: ⏳ QUEUED

**Prompt pour Antigravity**:
```
Tu dois tester position reconciliation avec connexions réseau instables.

CONTEXTE:
- Position reconciliation implémenté (src/modules/trading/reconciliation.rs)
- Besoin: tests avec déconnexions/reconnexions simulées

UTILISE LES AGENTS:
1. @agents_library/explore-code.md pour analyser reconciliation.rs
2. @agents_library/test-code.md pour créer tests réseau
3. @agents_library/apex-workflow.md si scénarios complexes

TÂCHES:
1. Créer tests/reconciliation_network_test.rs:
   - test_network_disconnect_during_trade()
   - test_missing_execution_event()
   - test_orphaned_position_cleanup()
   - test_concurrent_reconciliation()

2. Simuler conditions réseau:
   - Déconnexion TCP pendant ordre
   - Timeout sur execution event
   - Position locale vs broker mismatch

REPORTING:
Quand terminé, ajoute dans CLAUDE.md:
```markdown
### TODO-AUTO-003: COMPLETED ✅
**Date**: 2026-01-26 HH:MM
**Agent**: Antigravity
**Tests network**: XX tests PASSING
**Scénarios**: Disconnect, Timeout, Mismatch, Concurrent
```
```

---

### 🔵 TODO-AUTO-004: Sentiment Cache + Twitter Fallback
**ID**: AUTO-004  
**Agent**: Codex  
**Priorité**: MOYENNE  
**Durée estimée**: 2 prompts  
**Status**: ⏳ QUEUED

**Prompt pour Codex**:
```
Tu dois optimiser le sentiment scraping avec cache et fallback Twitter.

CONTEXTE:
- Sentiment cache déjà implémenté (TODO-CODEX-002 COMPLETED)
- Besoin: intégration Twitter fallback quand Perplexity rate limited

UTILISE LES AGENTS:
1. @agents_library/backend-architect.md pour implémenter fallback
2. @agents_library/test-code.md pour tests

TÂCHES:
1. Modifier src/modules/scraper/sentiment.rs:
   - Détecter HTTP 429 Perplexity
   - Fallback automatique sur twitter.rs
   - Logger switch de source
   - Combiner scores si les deux disponibles

2. Tests:
   - test_perplexity_rate_limit_triggers_twitter()
   - test_combined_sentiment_scores()
   - test_fallback_chain()

REPORTING:
Quand terminé, ajoute dans CLAUDE.md:
```markdown
### TODO-AUTO-004: COMPLETED ✅
**Date**: 2026-01-26 HH:MM
**Agent**: Codex
**Fallback**: Twitter activé sur 429
**Tests**: XX tests PASSING
```
```

---

## 🔄 BOUCLE AUTONOME - PROTOCOLE

### Cycle de Monitoring (toutes les 60 secondes)

```bash
while true; do
    # 1. Checker CLAUDE.md pour TODOs complétées
    grep "TODO-AUTO-.*: COMPLETED" CLAUDE.md
    
    # 2. Si TODO complétée → Dispatcher suivante dans queue
    # 3. Si queue vide → Mission terminée
    # 4. Logger status dans ORCHESTRATOR_STATUS.md
    
    sleep 60
done
```

### Instructions aux LLMs

**Antigravity & Codex**:
1. ✅ Utiliser IMPÉRATIVEMENT les agents de `agents_library/`
2. ✅ Pour tâches complexes: utiliser `@agents_library/apex-workflow.md`
3. ✅ Documenter dans CLAUDE.md quand terminé avec le format exact
4. ✅ Commit fichiers créés
5. ✅ Notifier completion avec ID TODO

**Format REPORTING obligatoire**:
```markdown
### TODO-AUTO-XXX: COMPLETED ✅
**Date**: 2026-01-26 HH:MM
**Agent**: [Antigravity|Codex]
**Durée**: XXm
**Fichiers**: [liste]
**Tests**: XX tests [PASSING|FAILING]
**Notes**: [optionnel]
```

---

## 📊 DASHBOARD TEMPS RÉEL

| ID | Tâche | Agent | Status | ETA |
|----|-------|-------|--------|-----|
| AUTO-001 | OAuth LIVE | Antigravity | 🔄 RUNNING | 30m |
| AUTO-002 | Circuit Breakers Tests | Codex | 🔄 RUNNING | 20m |
| AUTO-003 | Network Reconciliation | Antigravity | ⏳ QUEUED | 30m |
| AUTO-004 | Sentiment Fallback | Codex | ⏳ QUEUED | 20m |

**Total TODOs**: 4  
**Complétées**: 0  
**En cours**: 2  
**Queued**: 2

---

## 🎯 CRITÈRES DE SUCCÈS

### Phase 1 (AUTO-001, AUTO-002):
- ✅ OAuth LIVE fonctionnel
- ✅ Circuit breakers testés avec stress tests
- ✅ Compilation OK (cargo build)

### Phase 2 (AUTO-003, AUTO-004):
- ✅ Position reconciliation testée réseau instable
- ✅ Sentiment fallback Twitter opérationnel
- ✅ Tests coverage > 90%

### Final:
- ✅ TOUS les tests passent (cargo test)
- ✅ Bot prêt pour déploiement Railway
- ✅ Documentation complète

---

## 📡 COMMUNICATION INTER-LLM

**Via CLAUDE.md** (source unique de vérité):
- Antigravity check section "TODO-AUTO-00X: COMPLETED"
- Codex check section "TODO-AUTO-00X: COMPLETED"
- Orchestrator (moi) surveille et dispatche nouvelles TODOs

**Pas de questions** - Exécution autonome pure.

---

**Orchestrator**: AMP  
**Mode**: FULL AUTONOMOUS  
**Start Time**: 2026-01-26 17:00 CET  
**Expected End**: 2026-01-26 19:00 CET (+2h)
