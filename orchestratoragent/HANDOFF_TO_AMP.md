# 🔄 HANDOFF ORCHESTRATION: Claude → AMP

**Date**: 2026-01-22 11:35 CET
**Raison**: Quota session Claude à 93%
**Nouveau orchestrateur**: AMP

---

## 📋 CONTEXTE DE LA SESSION

### Ce qui a été fait aujourd'hui

1. **Communication inter-agents** ✅
   - Envoyé "bonjour" à AMP, Antigravity, Codex via tmux
   - Vérifié les réponses dans `orchestratoragent/AGENT_RESPONSES.md`
   - Envoyé "comment ça va ?" - tous ont répondu

2. **Documentation skills** ✅
   - Créé `orchestratoragent/skills/COMMUNICATION_INTER_AGENTS.md`
   - Documente comment communiquer entre agents via tmux

3. **Monitoring quota** ✅
   - Créé scripts de surveillance quota dans `orchestratoragent/scripts/`
   - `check_claude_quota.sh` - Vérification ponctuelle
   - `quota_watchdog.sh` - Surveillance continue avec alerte à 93%

---

## 📚 MÉMOIRE CLAUDE À LIRE

### Fichiers essentiels

```bash
# Instructions du projet
cat /home/julien/Documents/palm-oil-bot/CLAUDE.md

# Réponses des agents
cat /home/julien/Documents/palm-oil-bot/orchestratoragent/AGENT_RESPONSES.md

# Skills de communication
cat /home/julien/Documents/palm-oil-bot/orchestratoragent/skills/COMMUNICATION_INTER_AGENTS.md

# Scripts de monitoring
ls /home/julien/Documents/palm-oil-bot/orchestratoragent/scripts/
```

### Structure du projet

```
palm-oil-bot/
├── CLAUDE.md                    # Instructions principales du projet
├── src/                         # Code Rust du bot
├── orchestratoragent/
│   ├── AGENT_RESPONSES.md       # Réponses des agents
│   ├── HANDOFF_TO_AMP.md        # CE FICHIER
│   ├── skills/
│   │   └── COMMUNICATION_INTER_AGENTS.md
│   └── scripts/
│       ├── check_claude_quota.sh
│       ├── quota_watchdog.sh
│       └── check_claude_tokens.sh
```

---

## 🎯 TÂCHES EN COURS À REPRENDRE

### Pour toi (AMP) - Orchestrateur

| Priorité | Tâche | Description |
|----------|-------|-------------|
| 🔴 HIGH | Documenter skill quota | Créer `orchestratoragent/skills/QUOTA_MONITORING.md` avec tout ce qu'on a fait |
| 🟡 MED | Surveiller les agents | Vérifier que Antigravity et Codex travaillent sur leurs tâches |
| 🟡 MED | Coordonner | Distribuer les tâches selon CLAUDE.md |

### Pour Antigravity (TASK-PO-011)

| Status | Tâche |
|--------|-------|
| 🔄 IN_PROGRESS | Strategy analysis - Analyser et améliorer la stratégie de trading |

**Action requise**: Vérifier son avancement
```bash
tmux send-keys -t orchestration-palm-oil-bot:antigravity "Quel est ton avancement sur TASK-PO-011 ?" Enter
```

### Pour Codex (TASK-PO-013)

| Status | Tâche |
|--------|-------|
| 🔄 IN_PROGRESS | Code review + compilation check |

**Action requise**: Vérifier son avancement
```bash
tmux send-keys -t orchestration-palm-oil-bot:codex "Quel est ton avancement sur TASK-PO-013 ?" Enter
```

### Tâches globales du projet (depuis CLAUDE.md)

| ID | Tâche | Agent | Status |
|----|-------|-------|--------|
| TASK-PO-011 | Strategy analysis | Antigravity | 🔄 IN_PROGRESS |
| TASK-PO-012 | Tests unitaires | test-engineer | PENDING |
| TASK-PO-013 | Code review + compilation | Codex | 🔄 IN_PROGRESS |

---

## 🔧 COMMANDES TMUX ESSENTIELLES

### Voir les agents actifs

```bash
tmux list-windows -t orchestration-palm-oil-bot
```

### Envoyer un message à un agent

```bash
# Syntaxe
tmux send-keys -t orchestration-palm-oil-bot:<window> "<message>" Enter

# Exemples
tmux send-keys -t orchestration-palm-oil-bot:antigravity "Status update ?" Enter
tmux send-keys -t orchestration-palm-oil-bot:codex "Avancement ?" Enter
tmux send-keys -t orchestration-palm-oil-bot:claude "Message à Claude" Enter
```

### Voir la sortie d'un agent

```bash
tmux capture-pane -t orchestration-palm-oil-bot:<window> -p | tail -30
```

### Soumettre un message en attente (Enter seul)

```bash
tmux send-keys -t orchestration-palm-oil-bot:<window> Enter
```

---

## 📊 MONITORING QUOTA

### Vérifier le quota de Claude

```bash
/home/julien/Documents/palm-oil-bot/orchestratoragent/scripts/check_claude_quota.sh
```

### Lancer le watchdog (surveillance continue)

```bash
nohup /home/julien/Documents/palm-oil-bot/orchestratoragent/scripts/quota_watchdog.sh &
```

### Lire le quota actuel

```bash
cat /tmp/claude_current_quota
```

---

## ⚡ ACTIONS IMMÉDIATES POUR AMP

1. **Lire ce fichier** ✅ (tu es en train de le faire)

2. **Lire CLAUDE.md pour le contexte complet**
   ```bash
   cat /home/julien/Documents/palm-oil-bot/CLAUDE.md
   ```

3. **Vérifier le status des agents**
   ```bash
   tmux send-keys -t orchestration-palm-oil-bot:antigravity "Status TASK-PO-011 ?" Enter
   tmux send-keys -t orchestration-palm-oil-bot:codex "Status TASK-PO-013 ?" Enter
   ```

4. **Créer la documentation skill quota**
   - Fichier: `orchestratoragent/skills/QUOTA_MONITORING.md`
   - Documenter: scripts créés, comment ça marche, comment l'utiliser

5. **Mettre à jour le log dans CLAUDE.md**
   - Ajouter une entrée dans "Log des Actions LLM"
   - Marquer le handoff Claude → AMP

---

## 📝 TEMPLATE POUR DOCUMENTER LE SKILL QUOTA

```markdown
# Skill: Quota Monitoring

## Vue d'ensemble
[Expliquer le problème et la solution]

## Scripts créés
- check_claude_quota.sh : [description]
- quota_watchdog.sh : [description]

## Comment ça marche
[Expliquer le parsing du footer tmux]

## Utilisation
[Exemples de commandes]

## Intégration avec le handoff
[Comment utiliser pour déclencher un handoff automatique]
```

---

## 🚨 EN CAS DE PROBLÈME

### Si un agent ne répond pas

```bash
# Vérifier s'il est actif
tmux capture-pane -t orchestration-palm-oil-bot:<window> -p | tail -10

# Essayer de soumettre avec Enter
tmux send-keys -t orchestration-palm-oil-bot:<window> Enter
```

### Si tu as besoin de Claude

```bash
tmux send-keys -t orchestration-palm-oil-bot:claude "Message pour Claude" Enter
```

### Contact utilisateur

Si blocage majeur, demander à l'utilisateur (Julien) dans le chat.

---

**Handoff préparé par**: Claude (Orchestrateur sortant)
**À**: AMP (Nouvel orchestrateur)
**Timestamp**: 2026-01-22 11:35 CET
