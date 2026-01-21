# Multi-LLM Orchestration System v2

Systeme d'orchestration ameliore pour coordonner 4 LLMs (Claude, AMP, Antigravity, Codex) sur le projet Palm Oil Trading Bot.

## Nouveautes v2

- **Handoff automatique** Claude -> AMP quand 98% des tokens utilises
- **Prompts longs et autonomes** pour maximiser le travail en parallele
- **Monitoring continu** des agents avec alertes
- **Dispatch automatique** des nouvelles taches
- **Queue de taches** persistante en JSON

---

## Architecture v2

```
orchestratoragent/
├── start-orchestration-v2.sh   # Lance le systeme v2
├── stop-orchestration.sh       # Arrete proprement
├── check-health.sh             # Verifie l'etat
├── config/
│   ├── orchestration.conf      # Config generale
│   └── task_queue.json         # Queue des taches (NEW)
├── prompts/
│   ├── claude-orchestrator-v2.md   # Prompt orchestrateur v2 (NEW)
│   ├── amp-orchestrator.md         # Prompt AMP en mode orchestrateur (NEW)
│   ├── amp-worker-v2.md            # Prompt worker v2 (NEW)
│   ├── antigravity-worker-v2.md    # Prompt worker v2 (NEW)
│   └── codex-worker-v2.md          # Prompt worker v2 (NEW)
├── scripts/
│   ├── monitor-agents.sh       # Surveillance continue (NEW)
│   ├── dispatch-tasks.sh       # Dispatch automatique (NEW)
│   └── send-initial-tasks.sh   # Taches initiales (NEW)
├── handoff/                    # Fichiers de handoff (NEW)
│   └── claude_to_amp.md        # Contexte pour handoff
├── tasks/                      # Fichiers de taches (NEW)
└── logs/
    ├── orchestration.log
    ├── monitor.log
    └── dispatch.log
```

---

## Demarrage Rapide

### 1. Lancer l'orchestration

```bash
cd /home/julien/Documents/palm-oil-bot/orchestratoragent
./start-orchestration-v2.sh
```

### 2. Envoyer les taches initiales

```bash
./scripts/send-initial-tasks.sh
```

### 3. Attacher la session tmux

```bash
tmux attach -t palm-oil-orchestration
```

---

## Mecanisme de Handoff

Quand Claude approche 98% de ses tokens, il doit:

1. **Creer le fichier de handoff**
   ```bash
   # Ecrire le contexte dans handoff/claude_to_amp.md
   ```

2. **Mettre a jour la config**
   ```bash
   # Changer current_orchestrator dans task_queue.json
   ```

3. **Envoyer le prompt a AMP**
   ```bash
   tmux send-keys -t palm-oil-orchestration:1 "[HANDOFF...]" Enter
   ```

### Fichier de Handoff

Le fichier `handoff/claude_to_amp.md` contient:
- Resume du contexte actuel
- Taches en cours avec leur etat
- Taches restantes a faire
- Decisions techniques importantes
- Instructions speciales pour AMP

---

## Agents et Roles

| Agent | Window | Role | Specialites |
|-------|--------|------|-------------|
| **Claude** | 0 | Orchestrateur | Architecture, decisions critiques, code complexe |
| **AMP** | 1 | Worker + Backup Orchestrateur | Implementation, features, tests, CRUD |
| **Antigravity** | 3 | Worker | Analyse profonde, optimisation, strategie |
| **Codex** | 4 | Worker | Generation code, types, boilerplate |

---

## Format des Taches (v2)

Les taches v2 sont LONGUES et AUTONOMES:

```
[TACHE ORCHESTRATEUR - AUTONOMIE MAXIMALE]

AGENT: [NOM]
TASK_ID: TASK-PO-[XXX]
PRIORITE: HAUTE/MOYENNE/BASSE
TIMESTAMP: [DATE]

=== CONTEXTE ===
[5-10 lignes de contexte]

=== OBJECTIF ===
[Description detaillee]

=== FICHIERS A CREER/MODIFIER ===
[Liste complete avec descriptions]

=== INSTRUCTIONS DETAILLEES ===
[10+ etapes numerotees]

=== TESTS A ECRIRE ===
[Liste des tests]

=== CRITERES DE VALIDATION ===
[Checklist]

=== APRES COMPLETION ===
[Instructions de mise a jour]

COMMENCE IMMEDIATEMENT - NE POSE PAS DE QUESTIONS
```

---

## Scripts Utiles

### Monitoring

```bash
# Surveillance continue (toutes les 60s)
./scripts/monitor-agents.sh

# Une seule verification
./scripts/monitor-agents.sh --once

# Intervalle personnalise
./scripts/monitor-agents.sh --interval 30
```

### Dispatch

```bash
# Dispatcher les taches predefinies
./scripts/dispatch-tasks.sh --predefined

# Dispatcher une tache specifique
./scripts/dispatch-tasks.sh --agent amp --task-id TASK-PO-025
```

---

## Queue des Taches

La queue est stockee dans `config/task_queue.json`:

```json
{
  "task_queue": {
    "pending": [...],
    "in_progress": [...],
    "completed": [...],
    "blocked": [...]
  }
}
```

---

## Navigation tmux

| Commande | Action |
|----------|--------|
| `Ctrl+b` puis `0-4` | Changer de fenetre |
| `Ctrl+b` puis `d` | Detacher la session |
| `Ctrl+b` puis `z` | Zoom sur fenetre |
| `Ctrl+b` puis `[` | Mode scroll |
| `Ctrl+b` puis `x` | Fermer pane |

---

## Regles d'Or v2

1. **TOUS les agents doivent TOUJOURS avoir une tache**
2. **Prompts minimum 20 lignes**
3. **Handoff AVANT la limite de tokens (90% = prepare, 98% = execute)**
4. **Surveillance active (check toutes les 2-3 min)**
5. **Documentation continue (CLAUDE.md + ORCHESTRATION_STATUS.md)**

---

## Depannage

### Agent inactif

```bash
# Envoyer un rappel
tmux send-keys -t palm-oil-orchestration:[WINDOW] "
[RAPPEL] Tu as une tache en cours. Continue le travail.
" Enter
```

### Session perdue

```bash
# Verifier les sessions
tmux ls

# Rattacher
tmux attach -t palm-oil-orchestration
```

### Handoff echoue

1. Verifier le fichier `handoff/claude_to_amp.md`
2. Verifier que AMP est actif
3. Relancer manuellement le prompt a AMP

---

## Logs

```bash
# Log principal
tail -f logs/orchestration.log

# Log monitoring
tail -f logs/monitor.log

# Log dispatch
tail -f logs/dispatch.log
```

---

## Prochaines Etapes

Apres demarrage:
1. Verifier que tous les agents ont recu leurs taches
2. Surveiller ORCHESTRATION_STATUS.md
3. Assigner de nouvelles taches quand completees
4. Preparer le handoff si Claude approche ses limites

---

**Version**: 2.0
**Date**: 2026-01-20
**Projet**: Palm Oil Trading Bot
