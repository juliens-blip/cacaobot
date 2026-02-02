# 🤖 ORCHESTRATOR AUTO MODE - ACTIVE
## Distribution Automatique Multi-LLM

**Date**: 2026-01-26 16:05
**Orchestrateur**: AMP (remplace Claude)
**Mode**: AUTONOMOUS DISTRIBUTION
**Session**: orchestration-palm-oil-bot

---

## 🎯 MISSION ORCHESTRATEUR

Je suis AMP, orchestrateur autonome. Je distribue des TODO avec 3-4 tâches, monitore les completions, et redistribue automatiquement.

**Protocole**:
1. Soumettre TODO via tmux send-keys + Enter
2. LLM exécute et documente dans CLAUDE.md
3. Je détecte completion et assigne nouveau TODO
4. Boucle infinie jusqu'à épuisement queue

---

## 📋 TODO DISTRIBUTION QUEUE

### 🔴 TODO-POB-001 [HAUTE] → AMP (window 2)
**Status**: READY_TO_SEND
**Priority**: CRITICAL
**Agent requis**: @backend-architect.md + @apex-workflow.md

**Tâches**:
1. Implémenter persistence SQLite pour positions/trades avec reconciliation startup
2. Ajouter single-reader pattern pour socket cTrader (tokio::sync::Mutex)
3. Créer module reconciliation post-reconnexion
4. Tester avec cargo test + documenter résultats

**Prompt XML**:
```xml
<system>
Tu es backend-architect expert Rust/async. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/backend-architect.md immédiatement.
</system>

<task>
Implémenter système de persistence + reconciliation pour Palm Oil Bot
</task>

<context>
Projet: Palm Oil Trading Bot (Rust)
Besoin: Persistence durable positions/trades + réconciliation après crash/reconnect
Fichiers existants: src/modules/trading/position_manager.rs, src/modules/trading/strategy.rs
Base code: Protobuf cTrader + RSI strategy
</context>

<constraints>
- OBLIGATOIRE: Charger @backend-architect.md de agents_library
- Si complexité haute: Utiliser @apex-workflow.md pour décomposer
- Utiliser rusqlite pour SQLite (lightweight)
- Pattern single-reader: tokio::sync::Mutex<TcpStream>
- Tests unitaires requis dans tests/persistence_test.rs
- Documenter dans CLAUDE.md section "TODO-POB-001 COMPLETED" une fois fini
</constraints>

<deliverables>
1. src/modules/trading/persistence.rs (CRUD positions/trades SQLite)
2. src/modules/trading/reconciliation.rs (logique reconciliation)
3. Modification position_manager.rs (integration persistence)
4. tests/persistence_test.rs (4+ tests unitaires)
5. Documentation CLAUDE.md avec status COMPLETED + fichiers créés
</deliverables>

<acceptance_criteria>
- cargo build --release: PASSED
- cargo test: PASSED (tous tests persistence)
- Reconciliation testée manuellement (simuler crash)
- Documentation CLAUDE.md complétée
</acceptance_criteria>

<scratchpad>
Étapes:
1. Charger backend-architect skill
2. Créer schema SQLite (positions: id, symbol, side, volume, entry, exit, status)
3. Impl CRUD operations (create_position, update_position, get_open_positions)
4. Single-reader socket via Arc<Mutex<TcpStream>>
5. Reconciliation: comparer DB vs cTrader positions au startup
6. Tests: insert, update, query, reconciliation
7. Documenter CLAUDE.md
</scratchpad>
</task>
```

---

### 🟡 TODO-POB-002 [MOYENNE] → ANTIGRAVITY (window 4)
**Status**: QUEUED
**Priority**: HIGH
**Agent requis**: @backend-architect.md

**Tâches**:
1. Structured logging JSON avec tracing (trade_id, symbol_id, timestamps)
2. Prometheus metrics export (P&L, win_rate, drawdown, cache_hit_rate)
3. Health check endpoint HTTP ou heartbeat logs
4. Alerting config pour disconnects + order rejections

**Prompt XML**:
```xml
<system>
Tu es backend developer expert observabilité Rust. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/backend-architect.md.
</system>

<task>
Implémenter monitoring production-ready avec structured logging + Prometheus metrics
</task>

<context>
Projet: Palm Oil Trading Bot
Besoin: Observabilité pour production Railway (logs JSON + metrics Prometheus)
Fichiers existants: src/modules/monitoring/metrics.rs, src/modules/monitoring/dashboard.rs
</context>

<constraints>
- OBLIGATOIRE: Charger @backend-architect.md
- Utiliser crate 'tracing-subscriber' avec JSON formatter
- Utiliser crate 'metrics' + 'metrics-exporter-prometheus'
- Health check: Simple HTTP endpoint /health ou log heartbeat 30s
- Documenter dans CLAUDE.md section "TODO-POB-002 COMPLETED"
</constraints>

<deliverables>
1. src/modules/monitoring/prometheus.rs (metrics export /metrics)
2. Modification metrics.rs (structured logging JSON)
3. src/modules/monitoring/health_check.rs (endpoint HTTP ou heartbeat)
4. tests/monitoring_test.rs (tests metrics + health)
5. Documentation CLAUDE.md COMPLETED
</deliverables>

<acceptance_criteria>
- cargo build: PASSED
- Logs JSON-formatted avec context fields
- Metrics accessibles via /metrics (ou exportées)
- Health check testable
</acceptance_criteria>

<scratchpad>
1. Charger backend-architect
2. JSON logging: tracing_subscriber::fmt().json()
3. Metrics: counter (trades), gauge (balance), histogram (latency)
4. Health: axum endpoint /health returning {"status":"ok"}
5. Tests: vérifier format logs + metrics increment
6. CLAUDE.md documentation
</scratchpad>
</task>
```

---

### 🟢 TODO-POB-003 [SIMPLE] → CODEX (window 5)
**Status**: QUEUED
**Priority**: MEDIUM
**Agent requis**: @test-engineer.md + @test-code.md

**Tâches**:
1. Tests d'intégration cTrader DEMO (auth + subscribe + spot events)
2. Tests Perplexity API (happy path + HTTP 429 rate limit)
3. Tests Twitter fallback activation
4. Rapport TESTS_INTEGRATION_REPORT.md

**Prompt XML**:
```xml
<system>
Tu es test-engineer expert Rust. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/test-engineer.md ET @test-code.md.
</system>

<task>
Créer suite complète de tests d'intégration pour APIs externes
</task>

<context>
Projet: Palm Oil Trading Bot
Besoin: Tests intégration avant production (cTrader, Perplexity, Twitter)
Fichiers: src/modules/trading/ctrader.rs, src/modules/scraper/perplexity.rs, src/modules/scraper/twitter.rs
</context>

<constraints>
- OBLIGATOIRE: Charger @test-engineer.md + @test-code.md
- Utiliser tokio::test pour async tests
- Mock external APIs avec wiremock ou reqwest::mock
- Tests DEMO uniquement (pas LIVE cTrader)
- Documenter dans CLAUDE.md section "TODO-POB-003 COMPLETED"
</constraints>

<deliverables>
1. tests/integration/ctrader_demo_test.rs (auth + subscribe + spot)
2. tests/integration/perplexity_api_test.rs (success + rate limit)
3. tests/integration/twitter_fallback_test.rs (activation quand Perplexity fail)
4. TESTS_INTEGRATION_REPORT.md (résumé + recommandations)
5. Documentation CLAUDE.md COMPLETED
</deliverables>

<acceptance_criteria>
- cargo test --test integration: PASSED
- Tous scénarios couverts (success + failure paths)
- Rapport détaillé généré
</acceptance_criteria>

<scratchpad>
1. Charger test-engineer + test-code skills
2. cTrader test: connexion demo.ctraderapi.com mock auth response
3. Perplexity test: mock 200 OK + mock 429 rate limit
4. Twitter test: forcer Perplexity fail → vérifier Twitter call
5. Rapport: tableaux résultats + gaps détectés
6. CLAUDE.md doc
</scratchpad>
</task>
```

---

### 🔴 TODO-POB-004 [HAUTE] → AMP (window 2)
**Status**: QUEUED
**Priority**: CRITICAL
**Agent requis**: @backend-architect.md + @apex-workflow.md

**Tâches**:
1. Secrets management Railway (env vars strictes, no .env runtime)
2. TLS enforcement validation rustls (LIVE + DEMO certificates)
3. Network allow-list endpoints (cTrader, Perplexity, Nitter only)
4. Rate limiting backoff exponential + jitter

**Prompt XML**:
```xml
<system>
Tu es security architect expert Rust. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/backend-architect.md + @apex-workflow.md si complexe.
</system>

<task>
Hardening sécurité production: secrets, TLS, network boundaries, rate limiting
</task>

<context>
Projet: Palm Oil Trading Bot
Besoin: Sécurité production Railway (secrets management, TLS strict, rate limiting)
Fichiers: src/config.rs, src/modules/trading/ctrader.rs, src/modules/scraper/perplexity.rs
</context>

<constraints>
- OBLIGATOIRE: Charger @backend-architect.md (+ apex-workflow si complexe)
- Secrets: std::env::var() uniquement, panic si manquant
- TLS: rustls avec certificate verification (rustls-native-certs)
- Rate limiting: crate 'governor' ou 'backoff' (exponential + jitter)
- Documenter CLAUDE.md "TODO-POB-004 COMPLETED"
</constraints>

<deliverables>
1. src/modules/security/secrets_manager.rs (load + validate env vars)
2. src/modules/security/rate_limiter.rs (backoff + jitter)
3. Modification ctrader.rs (TLS strict mode)
4. tests/security_test.rs (secrets validation + rate limit)
5. Documentation CLAUDE.md COMPLETED
</deliverables>

<acceptance_criteria>
- Secrets chargés depuis env uniquement (no .env file)
- TLS certificates validés pour LIVE + DEMO
- Rate limiting actif avec backoff
- Tests security PASSED
</acceptance_criteria>

<scratchpad>
1. Charger backend-architect (+ apex si needed)
2. Secrets: struct Config::from_env() avec std::env::var().expect()
3. TLS: TlsConnector::builder().min_protocol_version(TLS_1_2)
4. Rate limiter: governor::RateLimiter::direct() + backoff crate
5. Tests: mock env vars + assert panic si manquant
6. CLAUDE.md doc
</scratchpad>
</task>
```

---

## 🚀 DISTRIBUTION AUTOMATIQUE - COMMANDES

### Étape 1: Envoyer TODO-POB-001 à AMP
```bash
tmux send-keys -t orchestration-palm-oil-bot:2 '<system>
Tu es backend-architect expert Rust/async. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/backend-architect.md immédiatement.
</system>

<task>
Implémenter système de persistence + reconciliation pour Palm Oil Bot
</task>

<context>
Projet: Palm Oil Trading Bot (Rust)
Besoin: Persistence durable positions/trades + réconciliation après crash/reconnect
Fichiers existants: src/modules/trading/position_manager.rs, src/modules/trading/strategy.rs
Base code: Protobuf cTrader + RSI strategy
</context>

<constraints>
- OBLIGATOIRE: Charger @backend-architect.md de agents_library
- Si complexité haute: Utiliser @apex-workflow.md pour décomposer
- Utiliser rusqlite pour SQLite (lightweight)
- Pattern single-reader: tokio::sync::Mutex<TcpStream>
- Tests unitaires requis dans tests/persistence_test.rs
- Documenter dans CLAUDE.md section "TODO-POB-001 COMPLETED" une fois fini
</constraints>

<deliverables>
1. src/modules/trading/persistence.rs (CRUD positions/trades SQLite)
2. src/modules/trading/reconciliation.rs (logique reconciliation)
3. Modification position_manager.rs (integration persistence)
4. tests/persistence_test.rs (4+ tests unitaires)
5. Documentation CLAUDE.md avec status COMPLETED + fichiers créés
</deliverables>

<acceptance_criteria>
- cargo build --release: PASSED
- cargo test: PASSED (tous tests persistence)
- Reconciliation testée manuellement (simuler crash)
- Documentation CLAUDE.md complétée
</acceptance_criteria>' Enter
```

### Étape 2: Monitoring
```bash
# Attendre 5s puis vérifier AMP a démarré
sleep 5
tmux capture-pane -t orchestration-palm-oil-bot:2 -p | tail -20
```

### Étape 3: Boucle Auto (toutes les 60s)
```bash
# Vérifier status tous les LLMs
for w in 2 4 5; do 
  echo "=== Window $w ===" 
  tmux capture-pane -t orchestration-palm-oil-bot:$w -p | tail -10
done

# Si "files changed" détecté → Marquer COMPLETED → Envoyer prochain TODO
```

---

## 📊 TRACKING COMPLETIONS

### Template Documentation CLAUDE.md

Chaque LLM doit ajouter ceci dans CLAUDE.md quand terminé:

```markdown
### TODO-POB-XXX: COMPLETED ✅
**Date**: 2026-01-26 [HH:MM]
**Agent**: [AMP/Antigravity/Codex]
**Durée**: [Xm Ys]
**Fichiers créés**: 
- [liste fichiers]
**Fichiers modifiés**:
- [liste fichiers]
**Tests**: [PASSED/FAILED - détails]
**Notes**: [observations importantes]
**Prêt pour**: [TODO suivant]
```

---

## 🔄 AUTO-REDISTRIBUTION LOGIC

```
SI LLM window N montre "files changed" OU prompt vide 30s:
  1. Lire CLAUDE.md → Chercher "TODO-POB-XXX COMPLETED"
  2. Marquer TODO comme DONE dans tracking
  3. Sélectionner prochain TODO dans queue
  4. Envoyer via tmux send-keys -t :N "[prompt XML]" Enter
  5. Update ORCHESTRATOR_AUTO_MODE.md
  6. Répéter
```

---

**ORCHESTRATEUR ACTIF - PRÊT À DISTRIBUER**
**Prochaine action**: Envoyer TODO-POB-001 à AMP window 2
