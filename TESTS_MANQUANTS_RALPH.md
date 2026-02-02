# 🧪 TESTS MANQUANTS - Méthode RALPH
**Date**: 2026-01-27  
**Orchestrator**: Claude → Distribution Codex  
**Méthode**: RALPH (Run, Analyze, Lint, Polish, Handoff)

---

## 📋 Tests Manquants Identifiés

### 1. Test Connexion Réelle cTrader DEMO
**Fichier**: `tests/ctrader_connection_test.rs`  
**Priorité**: HAUTE  
**Assigné**: Codex

**Objectif**: Valider connexion TLS + authentification sur serveur DEMO réel

**Tests à créer**:
- `test_demo_connection_successful()` - Connexion TLS réussie
- `test_demo_authentication_flow()` - App auth + account auth
- `test_demo_invalid_credentials()` - Rejet credentials invalides
- `test_demo_reconnect_after_disconnect()` - Reconnexion automatique
- `test_demo_heartbeat_keepalive()` - Maintien connexion via heartbeat

**Méthode RALPH**:
1. **R (Run)**: `cargo test --test ctrader_connection_test`
2. **A (Analyze)**: Vérifier PASSED/FAILED + logs
3. **L (Lint)**: `cargo clippy` sur le fichier test
4. **P (Polish)**: Ajouter docs + assertions claires
5. **H (Handoff)**: Report dans `RALPH_CONNECTION_TESTS.md`

---

### 2. Test Sentiment Perplexity API Production
**Fichier**: `tests/perplexity_integration_test.rs`  
**Priorité**: HAUTE  
**Assigné**: Codex

**Objectif**: Valider sentiment API Perplexity avec vraie clé

**Tests à créer**:
- `test_perplexity_fcpo_sentiment_fetch()` - Requête réelle
- `test_perplexity_rate_limit_429()` - Gestion rate limit
- `test_perplexity_cache_hit()` - Cache TTL 5min
- `test_perplexity_fallback_twitter()` - Fallback si échec
- `test_perplexity_invalid_api_key()` - Erreur auth

**Méthode RALPH**:
1. **R (Run)**: `cargo test --test perplexity_integration_test`
2. **A (Analyze)**: Vérifier sentiment score -100 à +100
3. **L (Lint)**: `cargo clippy`
4. **P (Polish)**: Mock pour tests sans clé API
5. **H (Handoff)**: Report dans `RALPH_PERPLEXITY_TESTS.md`

---

### 3. Test Reconciliation Après Reconnexion
**Fichier**: `tests/position_reconciliation_reconnect_test.rs`  
**Priorité**: MOYENNE  
**Assigné**: Codex

**Objectif**: Valider sync positions après perte connexion cTrader

**Tests à créer**:
- `test_reconcile_after_disconnect()` - Sync au reconnect
- `test_orphaned_positions_cleanup()` - Suppression orphelins
- `test_missing_positions_detection()` - Détection manquants
- `test_position_mismatch_volume()` - Détection écarts volume
- `test_audit_log_reconciliation()` - Logs audit complets

**Méthode RALPH**:
1. **R (Run)**: `cargo test --test position_reconciliation_reconnect_test`
2. **A (Analyze)**: Vérifier audit trail timestamps
3. **L (Lint)**: `cargo clippy`
4. **P (Polish)**: Ajouter scénarios edge cases
5. **H (Handoff)**: Report dans `RALPH_RECONCILIATION_TESTS.md`

---

### 4. Test Symbol ID Resolution (FCPO)
**Fichier**: `tests/symbol_resolution_test.rs`  
**Priorité**: BASSE  
**Assigné**: Codex

**Objectif**: Valider résolution symbol_id pour FCPO

**Tests à créer**:
- `test_fcpo_symbol_id_lookup()` - Résolution FCPO → symbol_id
- `test_symbol_cache_persistence()` - Cache entre redémarrages
- `test_invalid_symbol_handling()` - Erreur symbol invalide
- `test_multiple_symbols_concurrent()` - Lookup parallèle

**Méthode RALPH**:
1. **R (Run)**: `cargo test --test symbol_resolution_test`
2. **A (Analyze)**: Vérifier symbol_id non-null
3. **L (Lint)**: `cargo clippy`
4. **P (Polish)**: Documenter mapping symbols
5. **H (Handoff)**: Report dans `RALPH_SYMBOL_TESTS.md`

---

## 🤖 Distribution Codex (Orchestrateur Universel)

### Session tmux: `orchestration-palm-oil-bot`

**Windows**:
- Window 0: main (monitoring)
- Window 1: claude (orchestrator)
- Window 2: amp (backup orchestrator)
- Window 5: codex (test executor)

### Commandes Distribution

```bash
# Initialiser session
SESSION="orchestration-palm-oil-bot"
tmux new-session -d -s $SESSION -n main
tmux new-window -t $SESSION:1 -n claude
tmux new-window -t $SESSION:5 -n codex

# Lancer Codex dans window 5
tmux send-keys -t $SESSION:5 "cd /home/julien/Documents/palm-oil-bot" Enter
tmux send-keys -t $SESSION:5 "aider --model openai/codex --no-auto-commits" Enter
sleep 3

# Task 1: cTrader Connection Test (PRIORITÉ 1)
tmux send-keys -t $SESSION:5 "
TODO CODEX-TEST-001: Crée tests/ctrader_connection_test.rs avec 5 tests:
1. test_demo_connection_successful() - Connexion TLS à demo.ctraderapi.com:5035
2. test_demo_authentication_flow() - ProtoOaApplicationAuthReq + ProtoOaAccountAuthReq
3. test_demo_invalid_credentials() - Vérifie rejet client_id/secret invalides
4. test_demo_reconnect_after_disconnect() - Reconnexion automatique
5. test_demo_heartbeat_keepalive() - Heartbeat maintient connexion

Utilise tokio::time::timeout pour éviter tests infinis. Credentials depuis .env.
Méthode RALPH: cargo test --test ctrader_connection_test puis report erreurs.
" Enter

sleep 60  # Attendre exécution

# Vérifier statut Codex
tmux capture-pane -t $SESSION:5 -p | tail -30

# Task 2: Perplexity Integration Test (PRIORITÉ 2)
tmux send-keys -t $SESSION:5 "
TODO CODEX-TEST-002: Crée tests/perplexity_integration_test.rs avec 5 tests:
1. test_perplexity_fcpo_sentiment_fetch() - Requête réelle Perplexity API
2. test_perplexity_rate_limit_429() - Gestion HTTP 429
3. test_perplexity_cache_hit() - Cache sentiment 5min TTL
4. test_perplexity_fallback_twitter() - Fallback Twitter si échec
5. test_perplexity_invalid_api_key() - Erreur auth

Skip tests si PERPLEXITY_API_KEY absente. Méthode RALPH: cargo test puis fix.
" Enter

sleep 60

# Task 3: Reconciliation Reconnect Test (PRIORITÉ 3)
tmux send-keys -t $SESSION:5 "
TODO CODEX-TEST-003: Crée tests/position_reconciliation_reconnect_test.rs avec 5 tests:
1. test_reconcile_after_disconnect() - Sync positions au reconnect
2. test_orphaned_positions_cleanup() - Suppression positions orphelines
3. test_missing_positions_detection() - Détection positions manquantes
4. test_position_mismatch_volume() - Détection écarts volume
5. test_audit_log_reconciliation() - Logs audit avec timestamps

Utilise src/modules/trading/position_reconciliation.rs. RALPH: test + fix.
" Enter

sleep 60

# Task 4: Symbol Resolution Test (PRIORITÉ 4)
tmux send-keys -t $SESSION:5 "
TODO CODEX-TEST-004: Crée tests/symbol_resolution_test.rs avec 4 tests:
1. test_fcpo_symbol_id_lookup() - Résolution FCPO → symbol_id
2. test_symbol_cache_persistence() - Cache entre redémarrages
3. test_invalid_symbol_handling() - Erreur symbol invalide
4. test_multiple_symbols_concurrent() - Lookup parallèle

Référence: SYMBOL_ID_RESOLUTION_REPORT.md. RALPH: test + fix.
" Enter
```

### Monitoring Codex (Boucle RALPH)

```bash
# Boucle de vérification (toutes les 60s)
while true; do
  echo "=== CODEX STATUS $(date +%H:%M:%S) ==="
  
  # Capturer output Codex
  OUTPUT=$(tmux capture-pane -t $SESSION:5 -p | tail -50)
  
  # Vérifier si task terminée
  if echo "$OUTPUT" | grep -q "test result: ok"; then
    echo "✅ Tests PASSED - Task terminée"
    
    # Exécuter cargo clippy
    tmux send-keys -t $SESSION:5 "cargo clippy --test $(basename $TEST_FILE .rs)" Enter
    sleep 10
    
    # Logger résultat
    echo "$OUTPUT" >> RALPH_CODEX_$(date +%Y%m%d_%H%M).log
    
  elif echo "$OUTPUT" | grep -q "test result: FAILED"; then
    echo "❌ Tests FAILED - Debug nécessaire"
    
    # Demander analyse erreur
    tmux send-keys -t $SESSION:5 "Analyse cette erreur et propose un fix" Enter
    
  fi
  
  sleep 60
done
```

---

## 📊 Tableau de Bord Tests

| Test | Fichier | Status | Tests | Pass | Fail | Codex |
|------|---------|--------|-------|------|------|-------|
| 1. cTrader Connection | `ctrader_connection_test.rs` | 🔄 TODO | 5 | - | - | ASSIGNED |
| 2. Perplexity API | `perplexity_integration_test.rs` | 🔄 TODO | 5 | - | - | ASSIGNED |
| 3. Reconciliation Reconnect | `position_reconciliation_reconnect_test.rs` | 🔄 TODO | 5 | - | - | ASSIGNED |
| 4. Symbol Resolution | `symbol_resolution_test.rs` | 🔄 TODO | 4 | - | - | ASSIGNED |

**Total**: 19 tests manquants

---

## 🎯 Critères de Succès RALPH

Pour chaque test:

1. **R (Run)**: `cargo test --test <nom>` → ALL PASS
2. **A (Analyze)**: Assertions claires, coverage > 80%
3. **L (Lint)**: `cargo clippy` → 0 warnings
4. **P (Polish)**: Docs, edge cases, mocks
5. **H (Handoff)**: Report MD avec métriques

**Output Final**: `RALPH_TESTS_FINAL_REPORT.md`

---

## 📝 Template Report RALPH

```markdown
# RALPH Test Report - [TEST_NAME]

**Date**: [DATE]
**Agent**: Codex
**Fichier**: [FILE]

## R - RUN
- Command: `cargo test --test [name]`
- Result: ✅ PASS / ❌ FAIL
- Tests: [X] passed, [Y] failed
- Duration: [TIME]

## A - ANALYZE
- Coverage: [X]%
- Assertions: [COUNT]
- Edge cases: [LIST]

## L - LINT
- Command: `cargo clippy --test [name]`
- Warnings: [COUNT]
- Auto-fix: [YES/NO]

## P - POLISH
- Documentation: ✅ / ❌
- Mocks: ✅ / ❌
- Error messages: ✅ / ❌

## H - HANDOFF
- Status: READY / BLOCKED
- Next steps: [ACTIONS]
```
