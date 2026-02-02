# 🎯 ORCHESTRATION FINALE - 100% FONCTIONNEL

**Date**: 2026-01-24  
**Orchestrateur**: AMP  
**Mode**: Distribution Multi-LLM via tmux

---

## 📊 ÉTAT ACTUEL

✅ **Compilation**: OK (cargo installé)  
✅ **Tests**: 190/190 PASS  
✅ **Backtest**: Fonctionnel (profit factor 1.31)  
✅ **Circuit Breakers**: Implémentés  
✅ **Risk Management**: OK

---

## 🚨 BLOQUANTS pour Production LIVE

### ❌ TASK-PROD-001: OAuth Production
**Status**: ❌ BLOQUANT  
**Description**: Actuellement auth DEMO uniquement  
**Fichier**: `src/modules/trading/oauth.rs` existe mais DEMO only  
**Agent**: **Apex** (complexe - OAuth flow complet)  
**Priorité**: CRITIQUE

**Prompt pour Apex**:
```
Implémenter OAuth Production pour cTrader dans src/modules/trading/oauth.rs.

Contexte:
- Fichier existe avec auth DEMO fonctionnelle
- Besoin: OAuth 2.0 flow complet pour serveur LIVE
- Endpoint LIVE: live.ctraderapi.com:5035
- Variables: CTRADER_CLIENT_ID_LIVE, CTRADER_CLIENT_SECRET_LIVE

Implémentation requise:
1. Enum Environment { Demo, Live }
2. OAuth flow complet avec refresh token
3. Token persistence (JSON ou fichier sécurisé)
4. Auto-refresh avant expiration
5. Tests unitaires pour les deux environments

Fichiers à modifier:
- src/modules/trading/oauth.rs
- src/config.rs (ajouter config LIVE)
- .env.example (documenter variables LIVE)

Tests requis:
- test_oauth_demo_flow()
- test_oauth_live_flow()
- test_token_refresh()
- test_token_persistence()

Livrable: oauth.rs production-ready + tests
```

---

### ❌ TASK-PROD-002: TLS Verification
**Status**: ❌ BLOQUANT  
**Description**: Besoin de tester avec serveur LIVE cTrader  
**Agent**: **Codex** (testing + validation)  
**Priorité**: CRITIQUE

**Prompt pour Codex**:
```
Créer tests de validation TLS pour connexion cTrader LIVE.

Fichier: tests/tls_verification_test.rs

Tests requis:
1. test_live_server_connection()
   - Connect à live.ctraderapi.com:5035
   - Vérifier handshake TLS réussi
   - Vérifier certificat valide

2. test_tls_certificate_chain()
   - Vérifier chaîne de certificats
   - Vérifier date d'expiration

3. test_tls_cipher_suites()
   - Vérifier ciphers supportés
   - Vérifier TLS 1.2+ minimum

4. test_demo_vs_live_connection()
   - Comparer comportement DEMO/LIVE
   - Documenter différences

Dépendances:
- rustls ou native-tls
- tokio-rustls pour tests async

Note: Tests peuvent fail si pas d'accès LIVE - documenter comment tester manuellement

Livrable: tls_verification_test.rs + documentation
```

---

### ❌ TASK-PROD-003: Installation Rust Railway
**Status**: ❌ BLOQUANT  
**Description**: Impossible de compiler sans cargo sur Railway  
**Agent**: **Infrastructure** (agent library devops)  
**Priorité**: CRITIQUE

**Prompt pour Infrastructure Agent**:
```
Vérifier et corriger Dockerfile pour Railway deployment.

Contexte:
- Dockerfile existe: /home/julien/Documents/palm-oil-bot/Dockerfile
- Build échoue probablement: cargo introuvable
- Besoin: Multi-stage build avec Rust toolchain

Étapes:
1. Vérifier Dockerfile actuel
2. Corriger si besoin:
   - FROM rust:1.75-slim (stage builder)
   - Install protobuf-compiler + libssl-dev
   - Cargo build --release
   - Runtime stage: debian slim + binary seulement

3. Test local:
   docker build -t palm-oil-bot .
   docker run -it palm-oil-bot cargo --version

4. Vérifier railway.toml
   - Builder: DOCKERFILE
   - Healthcheck si applicable

Livrable: Dockerfile validé + test build local réussi
```

---

## ⚠️ PRIORITÉ HAUTE (Sécurité)

### 🔄 TASK-SEC-001: Circuit Breakers Validation Live
**Status**: ⏳ À TESTER  
**Description**: Implémentés mais besoin validation live  
**Agent**: **Antigravity** (testing avancé)  
**Priorité**: HAUTE

**Prompt pour Antigravity**:
```
Créer tests de validation LIVE pour circuit breakers.

Fichier: tests/circuit_breakers_live_test.rs

Scénarios à tester:
1. Daily loss limit (-5%)
   - Simuler 5 trades perdants
   - Vérifier bot s'arrête

2. Consecutive losses (3x)
   - Simuler 3 pertes consécutives
   - Vérifier cooldown activé

3. Volatility spike detector
   - Injecter volatilité > 3%
   - Vérifier pause trading

4. Recovery after circuit break
   - Vérifier bot reprend après cooldown
   - Vérifier état persiste

Note: Utiliser position_manager persistence pour validation

Livrable: circuit_breakers_live_test.rs avec 4+ tests
```

---

### 🔄 TASK-SEC-002: Position Reconciliation
**Status**: ⏳ À TESTER  
**Description**: Test avec vraies connexions intermittentes  
**Agent**: **Antigravity** (async/concurrency expert)  
**Priorité**: HAUTE

**Prompt pour Antigravity**:
```
Tester position reconciliation avec connexions instables.

Fichier: tests/position_reconciliation_network_test.rs

Scénarios:
1. test_network_disconnect_during_trade()
   - Ouvrir position
   - Simuler déconnexion réseau
   - Reconnect
   - Vérifier position réconciliée

2. test_missing_execution_event()
   - Envoyer ordre
   - Drop execution event
   - Vérifier reconciliation détecte position manquante

3. test_orphaned_position_cleanup()
   - Créer position locale
   - Pas de position sur cTrader
   - Vérifier cleanup

4. test_concurrent_reconciliation()
   - Lancer 3 reconciliations simultanées
   - Vérifier pas de race conditions

Utiliser: position_manager.rs (déjà implémenté)

Livrable: Tests réseau + rapport comportement
```

---

## 🎨 OPTIMISATION (Nice-to-have)

### 📊 TASK-OPT-001: Backtest Tuning
**Status**: ⏳ EN ATTENTE  
**Description**: Profit factor 1.31 → target 1.5+  
**Agent**: **Algorithmic Trader** (agent library)  
**Priorité**: MOYENNE

**Prompt pour Algorithmic Trader**:
```
Optimiser stratégie trading pour améliorer profit factor.

Contexte actuel:
- Profit factor: 1.31 (objectif: 1.5+)
- Win rate: 44.8% (objectif: >50%)
- Max drawdown: 6.63% (objectif: <5%)

Fichier: src/bin/backtest_optimizer.rs (à créer)

Implémentation:
1. Parameter grid search:
   - RSI thresholds: 20-40 (buy) / 60-80 (sell)
   - Sentiment thresholds: 20-40 / -20 à -40
   - TP: 1.5-3%
   - SL: 1-2%

2. Métriques à optimiser:
   - Profit factor (poids: 40%)
   - Win rate (poids: 30%)
   - Max drawdown (poids: 30%)

3. Algorithme:
   - Grid search ou genetic algorithm
   - 1000+ iterations
   - Export best params → config

Livrable: backtest_optimizer.rs + best_params.json
```

---

### 📈 TASK-OPT-002: RSI Thresholds Optimization
**Status**: ⏳ EN ATTENTE  
**Description**: Optimiser via parameter sweep  
**Agent**: **Data Scientist** (agent library)  
**Priorité**: MOYENNE

**Prompt pour Data Scientist**:
```
Analyser et optimiser thresholds RSI via data analysis.

Fichier: scripts/rsi_analysis.py (Python pour analysis)

Analyse:
1. Extraire données backtest:
   - Prix, RSI, sentiment, P&L par trade
   - Export depuis backtest Rust → CSV

2. Correlation analysis:
   - RSI vs P&L
   - Sentiment vs P&L
   - RSI+Sentiment vs P&L

3. Threshold optimization:
   - Heatmap RSI thresholds vs profit factor
   - Identifier sweet spots
   - Recommandations

4. Visualizations:
   - Matplotlib: scatter plots, heatmaps
   - Export PNG → docs/

Livrable: rsi_analysis.py + rapport PDF + recommandations
```

---

### 🗄️ TASK-OPT-003: Sentiment Cache
**Status**: ⏳ EN ATTENTE  
**Description**: Éviter rate limits Perplexity  
**Agent**: **Backend Architect** (agent library)  
**Priorité**: MOYENNE

**Prompt pour Backend Architect**:
```
Implémenter cache pour Perplexity API avec TTL.

Fichier: src/modules/scraper/sentiment_cache.rs

Fonctionnalités:
1. In-memory cache:
   - HashMap<String, CachedSentiment>
   - TTL: 5 minutes
   - Max size: 100 entries

2. CachedSentiment struct:
   - score: i32
   - timestamp: DateTime
   - query: String

3. Integration:
   - Modifier perplexity.rs
   - Check cache avant API call
   - Fallback API si cache miss/expired

4. Tests:
   - test_cache_hit()
   - test_cache_miss()
   - test_cache_expiry()
   - test_cache_max_size()

Livrable: sentiment_cache.rs + integration tests
```

---

## 📋 DISTRIBUTION AGENTS

| Tâche | Agent | Complexité | ETA | Window tmux |
|-------|-------|------------|-----|-------------|
| TASK-PROD-001 | Apex | DIFFICILE | 30min | 3-Apex |
| TASK-PROD-002 | Codex | MOYENNE | 20min | 2-Codex |
| TASK-PROD-003 | Infrastructure | FACILE | 15min | 4-Infra |
| TASK-SEC-001 | Antigravity | MOYENNE | 25min | 5-Anti |
| TASK-SEC-002 | Antigravity | MOYENNE | 30min | 5-Anti |
| TASK-OPT-001 | Algorithmic | DIFFICILE | 45min | 6-Algo |
| TASK-OPT-002 | Data Scientist | MOYENNE | 40min | 7-Data |
| TASK-OPT-003 | Backend | FACILE | 20min | 8-Backend |

---

## 🚀 ORDRE D'EXÉCUTION

### Phase 1: BLOQUANTS (Parallèle)
```bash
# Lancer simultanément dans tmux
Window 3: Apex → TASK-PROD-001 (OAuth)
Window 2: Codex → TASK-PROD-002 (TLS)
Window 4: Infrastructure → TASK-PROD-003 (Dockerfile)
```

### Phase 2: SÉCURITÉ (Séquentiel après OAuth)
```bash
# Attendre TASK-PROD-001 terminé
Window 5: Antigravity → TASK-SEC-001 (Circuit breakers)
Window 5: Antigravity → TASK-SEC-002 (Reconciliation)
```

### Phase 3: OPTIMISATION (Optionnel)
```bash
# Parallèle après Phase 1+2
Window 6: Algorithmic → TASK-OPT-001
Window 7: Data Scientist → TASK-OPT-002
Window 8: Backend → TASK-OPT-003
```

---

## 📊 CRITÈRES DE SUCCÈS

### Production Ready:
- ✅ OAuth LIVE fonctionnel
- ✅ TLS validé sur serveur LIVE
- ✅ Docker build réussi
- ✅ Circuit breakers testés LIVE
- ✅ Reconciliation testée réseau instable

### Optimisé:
- ✅ Profit factor > 1.5
- ✅ Win rate > 50%
- ✅ Max drawdown < 5%
- ✅ Cache Perplexity implémenté

---

## 🎯 NEXT: Orchestrator Actions

1. ✅ Créer session tmux `palm-oil-final`
2. ✅ Distribuer prompts Phase 1 (3 agents)
3. ⏳ Surveiller progression (watch TASK-*.md reports)
4. ⏳ Distribuer Phase 2 après Phase 1
5. ⏳ Rapport final consolidé

---

**Orchestrateur**: AMP  
**Status**: READY TO EXECUTE  
**ETA Total**: 2-3 heures
