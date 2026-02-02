# 🤖 Workflow Autonome - Orchestration AMP

## Principe

En mode autonome, l'orchestrateur (AMP) :
1. **Distribue** les tâches aux LLMs disponibles
2. **Surveille** leur progression toutes les 30-60s
3. **Détecte** automatiquement la fin d'une tâche
4. **Redistribue** immédiatement la prochaine tâche
5. **Compile** et teste après chaque vague
6. **Boucle** jusqu'à 100% completion

## Cycle de Surveillance

```bash
# Boucle principale
while [ completion < 100% ]; do
    # 1. Vérifier agents (capture-pane)
    check_codex_status
    check_antigravity_status
    
    # 2. Détecter fin de tâche
    if agent_finished; then
        # 3. Compiler
        cargo build --release
        
        # 4. Assigner nouvelle tâche
        assign_next_task
    fi
    
    # 5. Attendre
    sleep 30
done
```

## Détection de Fin de Tâche

### Indicateurs Codex
```
✔ Task completed
files changed +X ~Y -Z
```

### Indicateurs Antigravity
```
La tâche [ID] est complète
Fichier créé avec succès
```

### Indicateurs génériques
- Prompt vide `❯` sans activité
- Barre de progression absente
- Message "Terminé" / "Fait" / "Complété"

## Assignation Automatique

### Queue de Tâches

```markdown
| Priorité | ID | Tâche | Agent | Statut |
|----------|----|-------------------------------------------------|-----------|--------|
| 🔴 HIGH  | 1  | Fix cTrader framing                             | Codex     | 🔄 IN_PROGRESS |
| 🔴 HIGH  | 2  | Position manager + persistence                  | Antigravity | 🔄 IN_PROGRESS |
| 🟡 MED   | 3  | Consolidate main.rs → TradingBot                | Codex     | QUEUED |
| 🟡 MED   | 4  | Integration tests end-to-end                    | AMP       | QUEUED |
| 🟡 MED   | 5  | Structured logging (tracing)                    | Antigravity | QUEUED |
| 🟢 LOW   | 6  | README update (new features)                    | AMP       | QUEUED |
| 🟢 LOW   | 7  | Docker multi-stage build optimization           | Codex     | QUEUED |
```

### Règles d'Assignation

1. **Codex** (OpenAI) : 
   - Refactoring complexe
   - Architecture fixes
   - Code review

2. **Antigravity** (Claude proxy) :
   - Nouveaux modules
   - Feature implementation
   - Tests unitaires

3. **AMP** (moi-même) :
   - Documentation
   - Scripts
   - Coordination

## Commandes de Monitoring

### Vérification rapide (1 agent)
```bash
tmux capture-pane -t orchestration-palm-oil-bot:5 -p | tail -15
```

### Vérification complète (tous agents)
```bash
echo "=== CODEX ===" && tmux capture-pane -t orchestration-palm-oil-bot:5 -p | tail -10
echo "=== ANTIGRAVITY ===" && tmux capture-pane -t orchestration-palm-oil-bot:4 -p | tail -10
```

### Check compilation
```bash
cargo build --release 2>&1 | grep -E "(Finished|error)"
```

### Check tests
```bash
cargo test 2>&1 | grep -E "(test result|running)"
```

## Soumission de Nouvelle Tâche

### Template
```bash
tmux send-keys -t orchestration-palm-oil-bot:<window> \
  "TASK-XX: [Description claire avec fichiers + objectifs + critères de succès]" Enter

# Vérifier soumission (3s)
sleep 3
tmux capture-pane -t orchestration-palm-oil-bot:<window> -p | tail -5

# Si pas soumis, Enter seul
tmux send-keys -t orchestration-palm-oil-bot:<window> Enter
```

### Exemple réel
```bash
# Tâche pour Codex
tmux send-keys -t orchestration-palm-oil-bot:5 \
  "Crée /home/julien/Documents/palm-oil-bot/tests/integration_ctrader_test.rs avec tests end-to-end: 1) connect, 2) auth, 3) subscribe FCPO, 4) receive spot event, 5) place order, 6) disconnect. Utilise tokio::test et mock si nécessaire." Enter

sleep 3
tmux send-keys -t orchestration-palm-oil-bot:5 Enter
```

## Logs et Traçabilité

### Fichier de log principal
`ORCHESTRATION_LOG.md` - Mis à jour après chaque action

### Format des entrées
```markdown
| 12:05 | AMP | Tâche assignée: integration tests | Codex window 5 | ✅ |
| 12:06 | Codex | Tâche reçue, démarrage | - | 🔄 |
| 12:09 | Codex | Tâche terminée | tests/integration_ctrader_test.rs | ✅ |
| 12:09 | AMP | Compilation check | cargo build | 🔄 |
```

## Gestion des Erreurs

### Agent ne répond pas
```bash
# 1. Vérifier status
tmux capture-pane -t orchestration-palm-oil-bot:<window> -p | tail -20

# 2. Si bypass permissions visible
tmux send-keys -t orchestration-palm-oil-bot:<window> Enter

# 3. Si bloqué sur erreur, skip et assigner ailleurs
# Log: "Agent <X> bloqué sur erreur, tâche réassignée à <Y>"
```

### Compilation échoue
```bash
# 1. Identifier erreur
cargo build --release 2>&1 | grep "error\["

# 2. Assigner fix à Codex
tmux send-keys -t orchestration-palm-oil-bot:5 \
  "URGENT: Fix compilation error dans <fichier>: <erreur>. Corrige-le immédiatement." Enter
```

### Test échoue
```bash
# 1. Run tests
cargo test 2>&1 | tee /tmp/test_output.log

# 2. Parser erreurs
grep "FAILED" /tmp/test_output.log

# 3. Assigner fix
tmux send-keys -t orchestration-palm-oil-bot:4 \
  "Fix test failure dans <module>::<test>. Erreur: <message>." Enter
```

## Critères de Succès Final

Le workflow autonome est terminé quand :
- ✅ `cargo build --release` → Finished (no errors)
- ✅ `cargo test` → test result: ok (0 failed)
- ✅ `cargo clippy` → 0 warnings
- ✅ Tous les fichiers TASK-* ont status COMPLETED dans CLAUDE.md
- ✅ README.md à jour
- ✅ ORCHESTRATION_LOG.md documente toutes les actions

---

**Auteur**: AMP  
**Date**: 2026-01-22  
**Version**: 1.0
