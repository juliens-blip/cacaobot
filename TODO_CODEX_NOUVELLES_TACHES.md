# 🎯 NOUVELLES TÂCHES CODEX - Palm Oil Bot
**Date**: 2026-01-26 16:30
**Orchestrateur**: AMP
**Agent**: Codex (window 5)
**Session**: orchestration-palm-oil-bot

---

## 📋 QUEUE CODEX (Antigravity INDISPONIBLE)

### 🟢 TODO-CODEX-004 [SIMPLE] - Tests Integration Modules
**Status**: 🔄 IN_PROGRESS (envoyé à Codex 16:32)
**Priority**: MEDIUM
**Agents requis**: @test-engineer.md + @test-code.md

**Contexte**: Les nouveaux modules persistence.rs et reconciliation.rs n'ont pas de tests d'intégration. Besoin de valider avec vraie DB SQLite + scénarios crash recovery.

**Tâches**:
1. Créer `tests/integration/persistence_integration_test.rs` avec scénarios crash/recovery
2. Créer `tests/integration/reconciliation_integration_test.rs` avec broker mock
3. Créer `tests/integration/full_stack_recovery_test.rs` simulant crash + reconciliation
4. Générer rapport dans `INTEGRATION_TESTS_REPORT.md`

**Prompt XML**:
```xml
<system>
Tu es test-engineer expert Rust. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/test-engineer.md ET @test-code.md immédiatement.
</system>

<task>
TODO-CODEX-004: Créer tests d'intégration pour persistence + reconciliation
</task>

<context>
Projet: Palm Oil Trading Bot
Nouveaux modules créés aujourd'hui:
- src/modules/trading/persistence.rs (SQLite CRUD positions/trades)
- src/modules/trading/reconciliation.rs (sync local ↔ broker)

Besoin: Tests intégration validant:
1. Crash recovery (DB persiste, reload au startup)
2. Reconciliation avec broker (orphaned, missing, mismatched positions)
3. Full stack: crash → reload → reconcile → trading reprise
</context>

<constraints>
- OBLIGATOIRE: Charger @test-engineer.md + @test-code.md
- Utiliser tokio::test pour async
- Mock broker avec BrokerPosition fixtures
- Tests doivent passer avec cargo test --test integration
- Documenter résultats dans INTEGRATION_TESTS_REPORT.md
- Documenter dans CLAUDE.md "TODO-CODEX-004 COMPLETED"
</constraints>

<deliverables>
1. tests/integration/persistence_integration_test.rs (6+ tests)
2. tests/integration/reconciliation_integration_test.rs (8+ tests)
3. tests/integration/full_stack_recovery_test.rs (3+ scénarios)
4. INTEGRATION_TESTS_REPORT.md (format tableau + recommandations)
5. Documentation CLAUDE.md COMPLETED
</deliverables>

<acceptance_criteria>
- cargo test --test integration: PASSED
- Tous scénarios crash/recovery fonctionnent
- Rapport détaillé avec métriques
</acceptance_criteria>

<scratchpad>
Étapes:
1. Charger skills test-engineer + test-code
2. Analyser persistence.rs pour comprendre API
3. Créer fixtures BrokerPosition
4. Tests persistence: insert → crash → reload → verify
5. Tests reconciliation: local vs broker discrepancies
6. Full stack: simuler bot crash + recovery
7. Rapport markdown avec tableaux
8. CLAUDE.md doc
</scratchpad>
</task>
```

---

### 🟡 TODO-CODEX-005 [MOYENNE] - Module Security Hardening
**Status**: QUEUED
**Priority**: HIGH
**Agents requis**: @backend-architect.md

**Contexte**: Production Railway nécessite secrets management strict + rate limiting Perplexity/Twitter.

**Tâches**:
1. Créer `src/modules/security/mod.rs` + exports
2. Créer `src/modules/security/secrets_manager.rs` (validation env vars stricte)
3. Créer `src/modules/security/rate_limiter.rs` (backoff exponential + jitter)
4. Modifier `src/config.rs` pour utiliser secrets_manager
5. Tests dans `tests/security_test.rs`

**Prompt XML**:
```xml
<system>
Tu es backend-architect expert sécurité Rust. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/backend-architect.md.
</system>

<task>
TODO-CODEX-005: Implémenter security hardening (secrets + rate limiting)
</task>

<context>
Projet: Palm Oil Trading Bot
Déploiement: Railway (production)
Besoin sécurité:
- Secrets chargés depuis env uniquement (no .env runtime)
- Panic si secrets manquants (fail-fast)
- Rate limiting Perplexity API (éviter ban + coûts)
- Rate limiting Twitter scraping (éviter IP ban)

Fichiers existants:
- src/config.rs (charge config depuis .env)
- src/modules/scraper/perplexity.rs (appelle Perplexity)
- src/modules/scraper/twitter.rs (scrappe Twitter)
</context>

<constraints>
- OBLIGATOIRE: Charger @backend-architect.md
- Secrets: std::env::var().expect() avec messages clairs
- Rate limiter: crate 'governor' OU 'backoff' (exponential + jitter)
- Logs ne doivent JAMAIS exposer secrets (redacted)
- Tests: mock env vars + assert panic si manquant
- Documenter CLAUDE.md "TODO-CODEX-005 COMPLETED"
</constraints>

<deliverables>
1. src/modules/security/mod.rs (module declaration)
2. src/modules/security/secrets_manager.rs (Config::from_env_strict)
3. src/modules/security/rate_limiter.rs (RateLimiter with backoff)
4. Modification config.rs (integration secrets_manager)
5. tests/security_test.rs (8+ tests secrets + rate limit)
6. Documentation CLAUDE.md COMPLETED
</deliverables>

<acceptance_criteria>
- cargo build --release: PASSED
- Panic si CTRADER_CLIENT_ID manquant
- Rate limiter bloque après N calls (configurable)
- Tests security PASSED
</acceptance_criteria>

<scratchpad>
1. Charger backend-architect skill
2. Créer SecretsManager struct
3. fn load_strict() → panic si var manquante
4. RateLimiter avec governor crate ou backoff
5. Intégrer dans config.rs + perplexity.rs
6. Tests: env::set_var + panic assertions
7. CLAUDE.md doc
</scratchpad>
</task>
```

---

### 🔴 TODO-CODEX-006 [COMPLEXE] - Monitoring Production (Prometheus)
**Status**: QUEUED
**Priority**: CRITICAL
**Agents requis**: @backend-architect.md + @apex-workflow.md (si complexe)

**Contexte**: Production Railway nécessite observabilité: structured logs JSON + metrics Prometheus + health check.

**Tâches**:
1. Créer `src/modules/monitoring/prometheus.rs` (export metrics /metrics)
2. Modifier `src/modules/monitoring/metrics.rs` (structured logging JSON)
3. Créer `src/modules/monitoring/health_check.rs` (HTTP endpoint /health)
4. Tests dans `tests/monitoring_integration_test.rs`

**Prompt XML**:
```xml
<system>
Tu es backend-architect expert observabilité. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/backend-architect.md. Si complexité haute, utilise @apex-workflow.md pour décomposer.
</system>

<task>
TODO-CODEX-006: Monitoring production (Prometheus + structured logs + health check)
</task>

<context>
Projet: Palm Oil Trading Bot
Déploiement: Railway
Besoin observabilité:
- Structured logs JSON (tracing-subscriber fmt().json())
- Metrics Prometheus: counter trades, gauge balance, histogram latency
- Health check HTTP /health returning {"status":"ok", "uptime_seconds": N}

Fichiers existants:
- src/modules/monitoring/metrics.rs (actuellement basic tracking)
- src/modules/monitoring/dashboard.rs (CLI dashboard)
</context>

<constraints>
- OBLIGATOIRE: Charger @backend-architect.md (+ apex si complexe)
- JSON logging: tracing_subscriber::fmt().json().init()
- Prometheus: crate 'metrics' + 'metrics-exporter-prometheus'
- Health: Simple HTTP server (axum lightweight) sur port 9090
- Tests: vérifier logs JSON + metrics increment + health respond 200
- Documenter CLAUDE.md "TODO-CODEX-006 COMPLETED"
</constraints>

<deliverables>
1. src/modules/monitoring/prometheus.rs (metrics registry + export)
2. Modification monitoring/metrics.rs (JSON structured logging)
3. src/modules/monitoring/health_check.rs (axum server /health)
4. Modification Cargo.toml (add metrics, metrics-exporter-prometheus, axum)
5. tests/monitoring_integration_test.rs (test metrics + health)
6. Documentation CLAUDE.md COMPLETED
</deliverables>

<acceptance_criteria>
- cargo build: PASSED
- Logs formatés JSON avec context fields (trade_id, symbol, etc.)
- Metrics accessibles via /metrics endpoint
- Health check répond 200 OK
- Tests monitoring PASSED
</acceptance_criteria>

<scratchpad>
1. Charger backend-architect (+ apex si needed)
2. JSON logs: tracing_subscriber::fmt().json()
3. Metrics: register counter/gauge/histogram
4. Health: axum Router::new().route("/health", get(health_handler))
5. Spawn health server dans main.rs
6. Tests: capture logs + assert format JSON
7. CLAUDE.md doc
</scratchpad>
</task>
```

---

### 🟢 TODO-CODEX-007 [SIMPLE] - Documentation Déploiement Railway
**Status**: QUEUED
**Priority**: MEDIUM
**Agents requis**: @api-documenter.md

**Contexte**: Besoin guide déploiement Railway avec checklist production.

**Tâches**:
1. Créer `docs/RAILWAY_DEPLOYMENT.md` (guide complet)
2. Créer `docs/PRODUCTION_CHECKLIST.md` (checklist pré-deploy)
3. Mettre à jour `README.md` section deployment
4. Créer `.railway/railway.toml` optimisé

**Prompt XML**:
```xml
<system>
Tu es technical writer expert. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/api-documenter.md.
</system>

<task>
TODO-CODEX-007: Documentation déploiement Railway
</task>

<context>
Projet: Palm Oil Trading Bot
Déploiement: Railway (Docker container 24/7)
Besoin: Guide deployment + checklist production

Fichiers existants:
- Dockerfile (déjà configuré)
- railway.toml (basic config)
- NEXT_STEPS.md (recommandations production)
</context>

<constraints>
- OBLIGATOIRE: Charger @api-documenter.md
- Guide Railway: env vars setup, deploy command, logs monitoring
- Checklist: secrets validation, TLS config, monitoring setup
- Format markdown professionnel avec exemples CLI
- Documenter CLAUDE.md "TODO-CODEX-007 COMPLETED"
</constraints>

<deliverables>
1. docs/RAILWAY_DEPLOYMENT.md (guide step-by-step)
2. docs/PRODUCTION_CHECKLIST.md (tableau checklist)
3. Mise à jour README.md section "Deployment"
4. .railway/railway.toml optimisé
5. Documentation CLAUDE.md COMPLETED
</deliverables>

<acceptance_criteria>
- Docs lisibles et actionnables
- Checklist exhaustive (secrets, TLS, monitoring, tests)
- Railway toml correct
</acceptance_criteria>

<scratchpad>
1. Charger api-documenter skill
2. Analyser Dockerfile + railway.toml existants
3. Rédiger guide Railway (env vars, deploy, rollback)
4. Créer checklist production (format tableau)
5. Mettre à jour README
6. CLAUDE.md doc
</scratchpad>
</task>
```

---

## 📊 RÉSUMÉ QUEUE CODEX

| TODO ID | Tâches | Priorité | Agents | Status |
|---------|--------|----------|--------|--------|
| TODO-CODEX-004 | Tests intégration persistence/reconciliation | MEDIUM | test-engineer, test-code | READY |
| TODO-CODEX-005 | Security hardening (secrets + rate limit) | HIGH | backend-architect | QUEUED |
| TODO-CODEX-006 | Monitoring Prometheus + logs JSON | CRITICAL | backend-architect, apex-workflow | QUEUED |
| TODO-CODEX-007 | Documentation Railway deployment | MEDIUM | api-documenter | QUEUED |

---

## 🚀 ORDRE DE DISTRIBUTION

1. **Envoyer TODO-CODEX-004** immédiatement (tests intégration - rapide)
2. Attendre completion → Envoyer TODO-CODEX-005 (security)
3. Attendre completion → Envoyer TODO-CODEX-006 (monitoring - complexe)
4. Attendre completion → Envoyer TODO-CODEX-007 (docs - facile)

---

## 📝 TEMPLATE DOCUMENTATION CLAUDE.md

Chaque fois que Codex termine un TODO:

```markdown
### TODO-CODEX-XXX: COMPLETED ✅
**Date**: 2026-01-26 [HH:MM]
**Agent**: Codex
**Durée**: [Xm Ys]
**Fichiers créés**: 
- [liste]
**Fichiers modifiés**:
- [liste]
**Tests**: [cargo test output]
**Notes**: [observations]
**Prêt pour**: TODO-CODEX-XXX (suivant)
```

---

**PRÊT À DISTRIBUER TODO-CODEX-004 À CODEX (window 5)**
