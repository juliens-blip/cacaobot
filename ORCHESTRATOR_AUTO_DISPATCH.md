# 🎯 ORCHESTRATOR AUTO-DISPATCH SYSTEM
**Date**: 2026-01-26 14:50:00  
**Orchestrator**: AMP (via tmux intercommunication)  
**Agents**: Antigravity (window 3) + Codex (window 5)

---

## 📡 SYSTÈME INTERCOMMUNICATION LLM

### Via tmux send-keys
```bash
# Session tmux: palm-oil-orchestration
# Window 3: Antigravity
# Window 5: Codex

# Envoyer tâche à Antigravity
tmux send-keys -t palm-oil-orchestration:3 "PROMPT" Enter

# Envoyer tâche à Codex
tmux send-keys -t palm-oil-orchestration:5 "PROMPT" Enter
```

---

## 🚀 DISPATCHING - TODO-BATCH-001 (Codex)

### TODO-CODEX-003: TLS Certificate Validation 🔒
**ID**: TODO-CODEX-003  
**Agent**: Codex (window 5)  
**Priorité**: BLOQUANT CRITIQUE

**Prompt à envoyer**:
```bash
tmux send-keys -t palm-oil-orchestration:5 "Tu es Codex. TÂCHE BLOQUANTE: TODO-CODEX-003 - TLS Certificate Validation. Créer src/bin/test_tls_connection.rs qui teste connexions à live.ctraderapi.com:5035 ET demo.ctraderapi.com:5035. Vérifier certificats SSL/TLS avec rustls. Utilise @agents_library/backend-architect.md et @agents_library/explore-code.md. Quand terminé, ajouter dans /home/julien/Documents/palm-oil-bot/CLAUDE.md: ### TODO-CODEX-003: COMPLETED **Date**: 2026-01-26 HH:MM **LIVE Server**: [OK/FAIL] **Certificate**: [VALID/INVALID] **Issues**: [description]. Commence maintenant." Enter
```

---

### TODO-CODEX-002: Sentiment Cache System 🧠
**ID**: TODO-CODEX-002  
**Agent**: Codex (window 5)  
**Priorité**: OPTIMISATION

**Prompt à envoyer (après TODO-CODEX-003)**:
```bash
tmux send-keys -t palm-oil-orchestration:5 "TODO-CODEX-002: Sentiment Cache System. Créer src/modules/scraper/sentiment_cache.rs avec cache in-memory HashMap<String, (i32, Instant)>, TTL 5min. Fallback Twitter si Perplexity rate limited (429). Logger cache hits/misses. Tests unitaires expiration. Utilise @agents_library/backend-architect.md. Documentation dans CLAUDE.md: ### TODO-CODEX-002: COMPLETED **Date** **Cache Hit Rate estimate** **TTL**. Exécute." Enter
```

---

### TODO-CODEX-001: Backtest Parameter Sweep 📊
**ID**: TODO-CODEX-001  
**Agent**: Codex (window 5)  
**Priorité**: OPTIMISATION

**Prompt à envoyer (après TODO-CODEX-002)**:
```bash
tmux send-keys -t palm-oil-orchestration:5 "TODO-CODEX-001: Backtest Optimizer. Créer src/bin/backtest_optimizer.rs avec grid search: RSI buy 20-35 step 5, RSI sell 65-80 step 5, TP 1.5-3% step 0.5%, SL 1-2% step 0.5%. Output CSV backtest_results.csv. Trouver profit_factor > 1.5. Utilise @agents_library/test-engineer.md. Documentation dans CLAUDE.md: ### TODO-CODEX-001: COMPLETED **Profit Factor** **Optimal Params**. Go." Enter
```

---

## 🚀 DISPATCHING - TODO-BATCH-002 (Antigravity)

### TODO-ANTI-001: Circuit Breakers Validation ⚠️
**ID**: TODO-ANTI-001  
**Agent**: Antigravity (window 3)  
**Priorité**: CRITIQUE

**Prompt à envoyer**:
```bash
tmux send-keys -t palm-oil-orchestration:3 "Tu es Antigravity avec extended thinking. TODO-ANTI-001: Circuit Breakers Live Validation. Utilise @agents_library/apex-workflow.md pour analyser src/modules/trading/strategy.rs. Créer tests/circuit_breakers_stress_test.rs pour: daily loss -5%, consecutive losses 3+, volatility spike. Simuler scénarios avec backtest. Utilise @agents_library/explore-code.md pour comprendre code existant. Documentation dans CLAUDE.md: ### TODO-ANTI-001: COMPLETED **Tests created** **Scenarios tested**. Start." Enter
```

---

### TODO-ANTI-002: Position Reconciliation System 🔄
**ID**: TODO-ANTI-002  
**Agent**: Antigravity (window 3)  
**Priorité**: HAUTE

**Prompt à envoyer (après TODO-ANTI-001)**:
```bash
tmux send-keys -t palm-oil-orchestration:3 "TODO-ANTI-002: Position Reconciliation. Créer src/modules/trading/position_reconciliation.rs avec cache local HashMap<String, Position>, mécanisme re-sync après reconnexion, logs détaillés audit trail. Tests connexions intermittentes: tests/position_reconciliation_test.rs. Utilise @agents_library/backend-architect.md et @agents_library/apex-workflow.md pour complexité. Documentation dans CLAUDE.md: ### TODO-ANTI-002: COMPLETED **Tests passing** **Cache implemented**. Execute." Enter
```

---

### TODO-ANTI-003: OAuth Production Setup 🔐
**ID**: TODO-ANTI-003  
**Agent**: Antigravity (window 3)  
**Priorité**: BLOQUANT

**Prompt à envoyer (après TODO-ANTI-002)**:
```bash
tmux send-keys -t palm-oil-orchestration:3 "TODO-ANTI-003: OAuth Production Setup. Modifier src/modules/trading/ctrader.rs pour switch DEMO/LIVE selon CTRADER_ENVIRONMENT env var. Ajouter dans .env.example. Documenter flux OAuth production dans README.md section Production Deployment. Créer guide migration DEMO → LIVE. Utilise @agents_library/backend-architect.md. Documentation dans CLAUDE.md: ### TODO-ANTI-003: COMPLETED **README updated** **Migration guide**. Go." Enter
```

---

## 📊 WORKFLOW AUTOMATIQUE

### Phase 1: Dispatch Initial (NOW)
```bash
# Lancer Codex sur TODO-CODEX-003
tmux send-keys -t palm-oil-orchestration:5 "[PROMPT TODO-CODEX-003]" Enter

# Lancer Antigravity sur TODO-ANTI-001
tmux send-keys -t palm-oil-orchestration:3 "[PROMPT TODO-ANTI-001]" Enter
```

### Phase 2: Monitoring (Auto)
```bash
# Script de surveillance toutes les 5 min
watch -n 300 'grep "TODO-CODEX.*COMPLETED" /home/julien/Documents/palm-oil-bot/CLAUDE.md'
watch -n 300 'grep "TODO-ANTI.*COMPLETED" /home/julien/Documents/palm-oil-bot/CLAUDE.md'
```

### Phase 3: Re-dispatch (Auto quand COMPLETED détecté)
```
CODEX termine TODO-CODEX-003
         ↓
CLAUDE.md updated avec "### TODO-CODEX-003: COMPLETED"
         ↓
Orchestrator détecte COMPLETED
         ↓
Dispatch TODO-CODEX-002 automatiquement
         ↓
Repeat jusqu'à toutes TODO done
```

---

## 🤖 COMMANDES D'EXÉCUTION

### Démarrer Orchestration
```bash
cd /home/julien/Documents/palm-oil-bot/orchestratoragent
./start-orchestration.sh
```

### Dispatch Codex TODO-CODEX-003 (MAINTENANT)
```bash
tmux send-keys -t palm-oil-orchestration:5 "Tu es Codex. TÂCHE BLOQUANTE: TODO-CODEX-003 - TLS Certificate Validation. Créer src/bin/test_tls_connection.rs qui teste connexions à live.ctraderapi.com:5035 ET demo.ctraderapi.com:5035. Vérifier certificats SSL/TLS avec rustls. Utilise agents_library/backend-architect.md et agents_library/explore-code.md. Quand terminé, ajouter dans CLAUDE.md: ### TODO-CODEX-003: COMPLETED **Date**: 2026-01-26 HH:MM **LIVE Server**: [OK/FAIL] **Certificate**: [VALID/INVALID]. Commence maintenant." Enter
```

### Dispatch Antigravity TODO-ANTI-001 (MAINTENANT)
```bash
tmux send-keys -t palm-oil-orchestration:3 "Tu es Antigravity avec extended thinking. TODO-ANTI-001: Circuit Breakers Live Validation. Utilise agents_library/apex-workflow.md pour analyser src/modules/trading/strategy.rs. Créer tests/circuit_breakers_stress_test.rs pour: daily loss -5%, consecutive losses 3+, volatility spike. Documentation dans CLAUDE.md: ### TODO-ANTI-001: COMPLETED **Tests**: [N] tests created. Start." Enter
```

---

## 📋 TRACKING STATUS

### Codex Tasks
| ID | Task | Status | ETA |
|----|------|--------|-----|
| TODO-CODEX-003 | TLS Certificate Validation | ⏳ DISPATCHED | 10 min |
| TODO-CODEX-002 | Sentiment Cache System | ⏸️ PENDING | +20 min |
| TODO-CODEX-001 | Backtest Parameter Sweep | ⏸️ PENDING | +30 min |

### Antigravity Tasks
| ID | Task | Status | ETA |
|----|------|--------|-----|
| TODO-ANTI-001 | Circuit Breakers Validation | ⏳ DISPATCHED | 20 min |
| TODO-ANTI-002 | Position Reconciliation | ⏸️ PENDING | +30 min |
| TODO-ANTI-003 | OAuth Production Setup | ⏸️ PENDING | +20 min |

---

## 🔍 MONITORING COMMANDS

### Vérifier session tmux active
```bash
tmux ls | grep palm-oil-orchestration
```

### Voir output Codex
```bash
tmux attach -t palm-oil-orchestration
# Ctrl+b puis 5 (window Codex)
```

### Voir output Antigravity
```bash
tmux attach -t palm-oil-orchestration
# Ctrl+b puis 3 (window Antigravity)
```

### Vérifier COMPLETED dans CLAUDE.md
```bash
tail -50 /home/julien/Documents/palm-oil-bot/CLAUDE.md | grep -A 5 "COMPLETED"
```

---

**STATUS**: ✅ READY TO DISPATCH  
**NEXT**: Exécuter les 2 commandes tmux send-keys ci-dessus
