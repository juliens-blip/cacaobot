# Guide d'Orchestration Multi-LLM

## Vue d'ensemble

Ce guide documente les problèmes rencontrés et solutions pour orchestrer plusieurs LLMs (Claude, AMP, Antigravity, Codex) via tmux.

**Session tmux**: `moana-orchestration` (ou `palm-oil-orchestration`)
**Fenêtres**: 0=main, 1=claude, 2=amp, 3=antigravity-proxy, 4=antigravity, 5=codex

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ORCHESTRATEUR (Claude)                    │
│                                                             │
│  1. Explore le projet                                       │
│  2. Identifie les tâches                                    │
│  3. Distribue aux LLMs                                      │
│  4. Surveille la progression                                │
│  5. Redistribue quand terminé                               │
└─────────────────────────────────────────────────────────────┘
           │              │              │
           ▼              ▼              ▼
     ┌─────────┐    ┌─────────┐    ┌─────────┐
     │   AMP   │    │Antigrav.│    │  Codex  │
     │ (win 2) │    │ (win 4) │    │ (win 5) │
     └─────────┘    └─────────┘    └─────────┘
```

---

## Flux d'Orchestration

```
1. EXPLORER
   └─> Lire CLAUDE.md, NEXT_STEPS.md, fichiers de status
   └─> Identifier ce qui reste à faire

2. DISTRIBUER (IMMÉDIATEMENT)
   └─> Créer fichiers de tâches (optionnel)
   └─> Envoyer prompts aux LLMs via tmux
   └─> Vérifier que les prompts sont exécutés (Enter si nécessaire)

3. TRAVAILLER EN PARALLÈLE
   └─> Faire ses propres tâches pendant que les LLMs travaillent

4. SURVEILLER (toutes les 15-30 secondes)
   └─> tmux capture-pane pour voir l'état
   └─> Identifier les LLMs qui ont terminé

5. REDISTRIBUER
   └─> Nouvelle tâche aux LLMs libres
   └─> Répéter jusqu'à fin des tâches
```

---

## Commandes Essentielles

### Lister les fenêtres tmux
```bash
tmux list-windows -t moana-orchestration
```

### Envoyer un prompt à un LLM
```bash
# Par numéro de fenêtre (RECOMMANDÉ)
tmux send-keys -t moana-orchestration:2 "Ton prompt ici" Enter

# Par nom de fenêtre
tmux send-keys -t moana-orchestration:amp "Ton prompt ici" Enter
```

### Vérifier l'état d'un LLM
```bash
tmux capture-pane -t moana-orchestration:2 -p | tail -15
```

### Forcer l'exécution (si prompt visible mais pas exécuté)
```bash
tmux send-keys -t moana-orchestration:2 Enter
```

---

## Mapping des Fenêtres

| Fenêtre | Nom | LLM | Usage |
|---------|-----|-----|-------|
| 0 | main | - | Terminal principal |
| 1 | claude | Claude Code | Orchestrateur principal |
| 2 | amp | AMP/Claude | Implémentation code |
| 3 | antigravity-proxy | Proxy | Communication Antigravity |
| 4 | antigravity | Antigravity | Analyse complexe, architecture |
| 5 | codex | OpenAI Codex | Code review, génération |

---

## Indicateurs d'État

### LLM en cours de travail
- `Running tools...`
- `Streaming response...`
- `Thinking...`
- `Burrowing...`
- `Embellishing...`
- `* Brewed for Xs`

### LLM terminé
- `✓ CLAUDE.md • X files changed +Y ~Z -W`
- `✻ Brewed for Xm Ys` (suivi d'un prompt vide)
- `test result: ok. X passed`
- Prompt `❯` ou `›` vide sans activité

### LLM en attente d'exécution
- `↵ send` visible
- Prompt visible dans le chat mais pas de réponse
- `⏵⏵ bypass permissions on`
