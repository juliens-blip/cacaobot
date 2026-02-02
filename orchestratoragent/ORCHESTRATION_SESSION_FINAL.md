# 🎯 SESSION ORCHESTRATION FINALE - PALM OIL BOT

**Date**: 2026-01-24
**Orchestrateur**: AMP (via Claude)
**Session tmux**: `palm-oil-final`
**Protocole**: ORCHESTRATION_COMPLETE.md

---

## 📊 OBJECTIF

Rendre le bot 100% fonctionnel en LIVE via distribution multi-agents.

---

## 🎭 AGENTS DISPONIBLES

| Agent | Window | Compétence | Status |
|-------|--------|------------|--------|
| AMP | 1-amp | Tasks complexes, compilation | ✅ READY |
| Codex | 2-codex | Tests, validation | ✅ READY |
| Antigravity | 3-anti | Analysis, async/concurrency | ✅ READY |
| Apex | 4-apex | OAuth, architecture complexe | ⏳ PENDING |
| Infrastructure | 5-infra | DevOps, Docker | ⏳ PENDING |

---

## 📋 DISTRIBUTION TASKS

### Phase 1: BLOQUANTS (Parallèle)

**TASK-PROD-001**: OAuth Production  
- **Agent**: Apex (Window 4)
- **Prompt**: Voir ORCHESTRATION_FINAL_TODO.md ligne 20-58
- **Livrable**: oauth.rs production-ready + tests
- **ETA**: 30min

**TASK-PROD-002**: TLS Verification  
- **Agent**: Codex (Window 2)
- **Prompt**: Voir ORCHESTRATION_FINAL_TODO.md ligne 66-98
- **Livrable**: tls_verification_test.rs
- **ETA**: 20min

**TASK-PROD-003**: Dockerfile Railway  
- **Agent**: Infrastructure (Window 5)
- **Prompt**: Voir ORCHESTRATION_FINAL_TODO.md ligne 106-138
- **Livrable**: Dockerfile validé + test build
- **ETA**: 15min

### Phase 2: SÉCURITÉ (Après Phase 1)

**TASK-SEC-001**: Circuit Breakers Live  
- **Agent**: Antigravity (Window 3)
- **Prompt**: Voir ORCHESTRATION_FINAL_TODO.md ligne 146-177
- **Livrable**: circuit_breakers_live_test.rs
- **ETA**: 25min

**TASK-SEC-002**: Position Reconciliation  
- **Agent**: Antigravity (Window 3)
- **Prompt**: Voir ORCHESTRATION_FINAL_TODO.md ligne 185-223
- **Livrable**: Tests réseau + rapport
- **ETA**: 30min

---

## 🔄 WORKFLOW ORCHESTRATION

```bash
# 1. Créer session
tmux new-session -d -s palm-oil-final -n main

# 2. Créer windows
tmux new-window -t palm-oil-final:1 -n amp
tmux new-window -t palm-oil-final:2 -n codex
tmux new-window -t palm-oil-final:3 -n anti
tmux new-window -t palm-oil-final:4 -n apex
tmux new-window -t palm-oil-final:5 -n infra

# 3. Lancer agents (à faire manuellement)
tmux send-keys -t palm-oil-final:1 "amp" Enter
tmux send-keys -t palm-oil-final:2 "# Codex ready" Enter
tmux send-keys -t palm-oil-final:3 "# Antigravity ready" Enter

# 4. Distribuer tâches Phase 1 (parallèle)
# Voir section PROMPTS ci-dessous
```

---

## 📝 PROMPTS À ENVOYER

### Window 4 - Apex (TASK-PROD-001)

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

Écris ton avancement dans orchestratoragent/APEX_RESPONSE.md
```

### Window 2 - Codex (TASK-PROD-002)

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

Écris ton avancement dans orchestratoragent/CODEX_RESPONSE.md
```

### Window 5 - Infrastructure (TASK-PROD-003)

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

Écris ton avancement dans orchestratoragent/INFRA_RESPONSE.md
```

---

## 📊 SURVEILLANCE

```bash
# Vérifier tous les agents (toutes les 2 min)
for agent in amp codex anti apex infra; do
    echo "=== $agent ==="
    tmux capture-pane -t palm-oil-final:$agent -p 2>/dev/null | tail -10
done

# Vérifier fichiers de réponse
ls -la orchestratoragent/*_RESPONSE.md 2>/dev/null
cat orchestratoragent/APEX_RESPONSE.md
```

---

## ✅ CRITÈRES DE SUCCÈS

Phase 1 complète quand:
- [ ] oauth.rs implémenté avec tests
- [ ] tls_verification_test.rs créé
- [ ] Dockerfile build réussi

Phase 2 complète quand:
- [ ] circuit_breakers_live_test.rs créé
- [ ] position_reconciliation_network_test.rs créé

---

**Créé par**: Claude Orchestrator  
**Next**: Lancer tmux session + distribuer prompts
