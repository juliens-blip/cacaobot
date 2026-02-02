# 🎯 ORCHESTRATOR TODO SYSTEM - Palm Oil Bot
## Distribution Automatique de Tâches Multi-LLM

**Date de démarrage**: 2026-01-26
**Orchestrateur actif**: AMP (Universal Orchestrator)
**Session tmux**: orchestration-palm-oil-bot

---

## 📋 QUEUE GLOBALE DE TODO

### TODO-ORC-101 [HAUTE PRIORITÉ - AMP]
**Assigné à**: AMP (window 2)
**Status**: PENDING
**Créé**: 2026-01-26 16:00

**Tâches**:
1. Implémenter système de persistence des positions (SQLite) avec réconciliation au startup
2. Ajouter socket concurrency pattern (single-reader) pour éviter corruption framing cTrader
3. Créer module de réconciliation post-reconnexion avec cTrader
4. Documenter dans CLAUDE.md section "Persistence & Reconciliation"

**Agents library requis**: 
- `@backend-architect.md` pour architecture persistence
- `@apex-workflow.md` pour tâches complexes si nécessaire

**Prompt engineering**:
```xml
<system>Tu es un backend architect expert Rust/async.</system>
<task>Implémenter la persistence des positions avec réconciliation</task>
<context>
Palm Oil Bot nécessite:
- Persistence SQLite pour positions/trades (audit + recovery)
- Pattern single-reader pour socket cTrader (éviter framing corruption)
- Réconciliation startup/reconnect
Fichiers existants: src/modules/trading/position_manager.rs
</context>
<constraints>
- UTILISER @backend-architect.md de agents_library
- UTILISER tokio::sync::Mutex pour single-reader
- UTILISER rusqlite pour SQLite
- TESTER avec cargo test après implémentation
</constraints>
<deliverables>
1. src/modules/trading/persistence.rs (module SQLite)
2. Modification position_manager.rs (intégration persistence)
3. src/modules/trading/reconciliation.rs (logique reconciliation)
4. Tests unitaires dans tests/persistence_test.rs
</deliverables>
<acceptance_criteria>
- cargo build --release passe
- Tests unitaires passent
- Réconciliation testée manuellement
</acceptance_criteria>
</task>
```

**Soumission**: 
```bash
tmux send-keys -t orchestration-palm-oil-bot:2 "$(cat TODO-ORC-101.txt)" Enter
```

**Documentation post-completion**:
Une fois terminé, documenter dans CLAUDE.md:
```markdown
### TODO-ORC-101: COMPLETED
**Date**: [timestamp]
**Agent**: AMP
**Fichiers créés**: persistence.rs, reconciliation.rs, tests/persistence_test.rs
**Tests**: PASSED
```

---

### TODO-ORC-102 [MOYENNE PRIORITÉ - ANTIGRAVITY]
**Assigné à**: Antigravity (window 4)
**Status**: PENDING
**Créé**: 2026-01-26 16:00

**Tâches**:
1. Ajouter structured logging avec timestamps/trade IDs/symbol IDs
2. Implémenter metrics export Prometheus-style (P&L, win rate, drawdown)
3. Créer health check endpoint ou heartbeat logs
4. Configurer alerting pour cTrader disconnects + order rejections

**Agents library requis**:
- `@backend-architect.md` pour architecture monitoring
- `@test-code.md` pour validation après implémentation

**Prompt engineering**:
```xml
<system>Tu es un backend developer expert en observabilité Rust.</system>
<task>Implémenter monitoring & alerting production-ready</task>
<context>
Palm Oil Bot nécessite:
- Structured logs (tracing) avec context (trade_id, symbol_id)
- Metrics Prometheus pour dashboards
- Health checks pour supervision
Fichiers existants: src/modules/monitoring/metrics.rs
</context>
<constraints>
- UTILISER @backend-architect.md
- UTILISER crate 'metrics' pour Prometheus export
- UTILISER 'tracing' avec JSON formatter
- AJOUTER health check dans main.rs
</constraints>
<deliverables>
1. src/modules/monitoring/prometheus.rs (export metrics)
2. Modification metrics.rs (structured logging)
3. src/modules/monitoring/health_check.rs
4. Tests dans tests/monitoring_test.rs
</deliverables>
<acceptance_criteria>
- cargo build passe
- Logs JSON-formatted
- Metrics exportables via /metrics endpoint
</acceptance_criteria>
</task>
```

---

### TODO-ORC-103 [SIMPLE PRIORITÉ - CODEX]
**Assigné à**: Codex (window 5)
**Status**: PENDING
**Créé**: 2026-01-26 16:00

**Tâches**:
1. Créer tests d'intégration cTrader DEMO (auth + subscribe flow)
2. Ajouter tests Perplexity API (happy path + 429 handling)
3. Tester Twitter fallback path
4. Générer rapport de tests dans TESTS_INTEGRATION_REPORT.md

**Agents library requis**:
- `@test-code.md` pour validation
- `@test-engineer.md` pour création tests

**Prompt engineering**:
```xml
<system>Tu es un test engineer expert Rust.</system>
<task>Créer suite de tests d'intégration</task>
<context>
Palm Oil Bot requiert tests avant production:
- cTrader demo connection + auth + subscribe
- Perplexity API + rate limiting
- Twitter fallback
Fichiers: src/modules/trading/ctrader.rs, src/modules/scraper/perplexity.rs
</context>
<constraints>
- UTILISER @test-engineer.md de agents_library
- UTILISER tokio::test pour async tests
- AJOUTER mocks pour external APIs
- DOCUMENTER résultats dans TESTS_INTEGRATION_REPORT.md
</constraints>
<deliverables>
1. tests/integration/ctrader_test.rs
2. tests/integration/perplexity_test.rs
3. tests/integration/twitter_fallback_test.rs
4. TESTS_INTEGRATION_REPORT.md
</deliverables>
<acceptance_criteria>
- cargo test --test integration passe
- Rapport complet généré
</acceptance_criteria>
</task>
```

---

### TODO-ORC-104 [HAUTE PRIORITÉ - AMP]
**Assigné à**: AMP (window 2)
**Status**: PENDING
**Créé**: 2026-01-26 16:00

**Tâches**:
1. Implémenter secrets management Railway-compatible (env vars strictes)
2. Ajouter TLS enforcement validation (rustls) LIVE + DEMO
3. Créer network allow-list pour endpoints (cTrader, Perplexity, Nitter)
4. Ajouter rate limiting avec backoff exponential + jitter

**Agents library requis**:
- `@backend-architect.md`
- `@apex-workflow.md` si complexité élevée

**Prompt engineering**:
```xml
<system>Tu es un security expert Rust backend.</system>
<task>Hardening sécurité production</task>
<context>
Palm Oil Bot doit sécuriser:
- Secrets management (pas de .env runtime)
- TLS strict pour cTrader
- Rate limiting Perplexity/Twitter
Fichiers: src/config.rs, src/modules/trading/ctrader.rs
</context>
<constraints>
- UTILISER @backend-architect.md
- VÉRIFIER TLS avec rustls-native-certs
- AJOUTER backoff avec crate 'backoff'
- LOGGER sans exposer secrets
</constraints>
<deliverables>
1. src/modules/security/secrets_manager.rs
2. src/modules/security/rate_limiter.rs
3. Modification ctrader.rs (TLS strict)
4. Tests dans tests/security_test.rs
</deliverables>
<acceptance_criteria>
- Secrets chargés depuis env uniquement
- TLS vérifié pour LIVE/DEMO
- Rate limiting actif
</acceptance_criteria>
</task>
```

---

## 📊 STATUT DES LLMs

### AMP (window 2)
- **Status**: IDLE
- **Current Task**: null
- **Tasks Queue**: [TODO-ORC-101, TODO-ORC-104]
- **Completed**: 0

### Antigravity (window 4)
- **Status**: IDLE
- **Current Task**: null
- **Tasks Queue**: [TODO-ORC-102]
- **Completed**: 0

### Codex (window 5)
- **Status**: IDLE
- **Current Task**: null
- **Tasks Queue**: [TODO-ORC-103]
- **Completed**: 0

---

## 🔄 PROTOCOL DE COMMUNICATION

### Pour chaque TODO:
1. **Générer prompt XML** (voir templates ci-dessus)
2. **Soumettre via tmux**:
   ```bash
   tmux send-keys -t orchestration-palm-oil-bot:N "[prompt]" Enter
   ```
3. **Vérifier démarrage** (attendre 5s):
   ```bash
   tmux capture-pane -t orchestration-palm-oil-bot:N -p | tail -20
   ```
4. **Une fois LLM terminé** (détecte "files changed" ou prompt vide):
   - Marquer TODO comme COMPLETED
   - Documenter dans CLAUDE.md section "Task Completion Log"
   - Assigner nouveau TODO depuis queue

### Détection état LLM:
- **WORKING**: "Working (Xs • esc to interrupt)"
- **DONE**: "files changed +X ~Y -Z" OU prompt vide stable 30s
- **IDLE**: Prompt "›" vide sans activité

---

## 📝 DOCUMENTATION POST-COMPLETION

Template à ajouter dans CLAUDE.md après chaque TODO:

```markdown
### TODO-ORC-XXX: COMPLETED
**Date**: [timestamp]
**Agent**: [AMP/Antigravity/Codex]
**Durée**: [Xm Ys]
**Fichiers créés**: [liste]
**Fichiers modifiés**: [liste]
**Tests**: [PASSED/FAILED]
**Notes**: [observations importantes]
```

---

## 🎯 RÈGLES D'ORCHESTRATION

1. **MAX 2-3 TODO par LLM** dans tasks_queue
2. **Distribuer selon complexité**:
   - HAUTE → AMP (window 2)
   - MOYENNE → Antigravity (window 4)
   - SIMPLE → Codex (window 5)
3. **Vérifier LLM status toutes les 60s**
4. **Appliquer Méthode Ralph** après chaque implémentation:
   - TEST → cargo test
   - DEBUG → Si erreur, analyser
   - FIX → Corriger et re-tester
   - MAX 3 cycles
5. **Documenter immédiatement** après completion
6. **Utiliser agents_library** pour chaque tâche
7. **Prompts en XML Anthropic** (system, task, context, constraints, deliverables)

---

## 🚨 ESCALADE

Si après 3 cycles Ralph le TODO échoue toujours:
1. Marquer TODO comme BLOCKED
2. Documenter l'erreur dans CLAUDE.md
3. Notifier utilisateur
4. Passer au TODO suivant

---

**Dernière mise à jour**: 2026-01-26 16:00 CET
**Orchestrateur**: AMP (Autonomous Mode)
