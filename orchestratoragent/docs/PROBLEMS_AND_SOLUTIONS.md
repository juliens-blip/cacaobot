# Problèmes Rencontrés & Solutions

## Problème 1: Prompts écrits mais pas exécutés

### Symptôme
Le texte du prompt apparaît dans le chat du LLM mais le LLM ne fait rien. Il attend.

### Cause
Le `Enter` envoyé avec `send-keys "texte" Enter` ne suffit pas toujours. Le LLM est en mode interactif et attend une confirmation supplémentaire.

### Solution
```bash
# Étape 1: Envoyer le prompt
tmux send-keys -t moana-orchestration:4 "Mon prompt ici" Enter

# Étape 2: Vérifier si exécuté (attendre 2-3 secondes)
sleep 3
tmux capture-pane -t moana-orchestration:4 -p | tail -5

# Étape 3: Si le prompt est visible mais pas exécuté, envoyer Enter
tmux send-keys -t moana-orchestration:4 Enter
```

### Indicateurs que le prompt n'est pas exécuté
- `↵ send` visible à côté du prompt
- Le prompt complet est visible dans la dernière ligne
- Pas de `Running tools...` ou `Thinking...`

---

## Problème 2: Mauvais noms de fenêtres tmux

### Symptôme
```
can't find window: codex-
```

### Cause
Les noms de fenêtres peuvent avoir des caractères spéciaux ou être différents de ce qu'on attend.

### Solution
**Toujours utiliser le numéro de fenêtre au lieu du nom:**

```bash
# MAUVAIS
tmux send-keys -t moana-orchestration:codex- "..."

# BON
tmux send-keys -t moana-orchestration:5 "..."
```

### Commande pour trouver les numéros
```bash
tmux list-windows -t moana-orchestration
# Output:
# 0: main (1 panes)
# 1: claude* (1 panes)
# 2: amp (1 panes)
# 3: antigravity-proxy (1 panes)
# 4: antigravity (1 panes)
# 5: codex- (1 panes)
```

---

## Problème 3: Commandes bash au lieu de prompts LLM

### Symptôme
Le LLM reçoit une commande bash et l'exécute littéralement au lieu de comprendre la tâche.

### Cause
On envoie des commandes bash comme `cd /path && cat file.md` au lieu de prompts en langage naturel.

### Solution

```bash
# MAUVAIS - Commande bash
tmux send-keys -t moana-orchestration:2 "cd /home/julien/Documents/palm-oil-bot && cat orchestratoragent/CODEX_TASK.md && echo 'Execute'" Enter

# BON - Prompt en langage naturel
tmux send-keys -t moana-orchestration:2 "Crée le fichier /home/julien/Documents/palm-oil-bot/src/modules/trading/circuit_breakers.rs avec un struct CircuitBreakers qui implémente daily_loss_limit, consecutive_losses, et volatility_threshold. Inclus des tests unitaires." Enter
```

### Règle
Les LLMs comprennent le langage naturel. Décris la tâche clairement avec:
- Le fichier à créer/modifier (chemin complet)
- Ce que le code doit faire
- Les fonctions/structs attendus
- Si des tests sont nécessaires

---

## Problème 4: LLMs en mode "bypass permissions"

### Symptôme
Le LLM affiche `⏵⏵ bypass permissions on (shift+tab to cycle)` et ne fait rien.

### Cause
Claude Code et certains LLMs ont un mode de permissions qui peut bloquer l'exécution automatique.

### Solution
```bash
# Vérifier l'état
tmux capture-pane -t moana-orchestration:4 -p | tail -10

# Si en attente, envoyer Enter pour confirmer
tmux send-keys -t moana-orchestration:4 Enter
```

### Note
Ce mode est normal. Il suffit d'envoyer Enter après le prompt pour que le LLM commence à travailler.

---

## Problème 5: Pas de vérification de progression

### Symptôme
On soumet des tâches mais on ne sait pas si les LLMs les exécutent vraiment.

### Cause
Pas de monitoring après soumission.

### Solution
**Boucle de monitoring:**

```bash
# Vérifier après 15-20 secondes
sleep 15

# Capturer l'état de chaque LLM
echo "=== AMP ===" && tmux capture-pane -t moana-orchestration:2 -p | tail -10
echo "=== ANTIGRAVITY ===" && tmux capture-pane -t moana-orchestration:4 -p | tail -10
echo "=== CODEX ===" && tmux capture-pane -t moana-orchestration:5 -p | tail -10
```

### Fréquence recommandée
- Vérifier toutes les 15-30 secondes
- Plus fréquent au début pour s'assurer que les prompts sont exécutés

---

## Problème 6: Oubli de distribuer les tâches en parallèle

### Symptôme
L'orchestrateur fait ses propres tâches et oublie de distribuer aux autres LLMs. Perte de temps.

### Cause
Réflexe de faire soi-même au lieu de déléguer.

### Solution
**Ordre des opérations:**

```
1. DISTRIBUER D'ABORD
   - Identifier les tâches pour chaque LLM
   - Envoyer les prompts via tmux
   - Vérifier qu'ils sont exécutés (Enter si nécessaire)

2. ENSUITE faire ses propres tâches
   - Pendant que les LLMs travaillent en parallèle

3. SURVEILLER périodiquement
   - Vérifier la progression
   - Redistribuer si terminé
```

---

## Problème 7: Pas de nouvelles tâches quand LLM termine

### Symptôme
Un LLM termine sa tâche et reste inactif pendant que d'autres tâches sont en attente.

### Cause
L'orchestrateur ne surveille pas assez fréquemment ou ne redistribue pas.

### Solution
```bash
# Vérifier si terminé
output=$(tmux capture-pane -t moana-orchestration:2 -p | tail -15)

# Indicateurs de fin:
# - "files changed +X ~Y -Z"
# - "Brewed for Xm Ys" suivi de prompt vide
# - "test result: ok"

# Si terminé, nouvelle tâche immédiatement
tmux send-keys -t moana-orchestration:2 "Nouvelle tâche: ..." Enter
```

---

## Problème 8: Session tmux différente du projet

### Symptôme
La session tmux est `moana-orchestration` mais le projet est `palm-oil-bot`.

### Cause
Session tmux créée pour un autre projet.

### Solution
```bash
# Option 1: Utiliser la session existante
tmux send-keys -t moana-orchestration:2 "cd /home/julien/Documents/palm-oil-bot && ..." Enter

# Option 2: Créer une nouvelle session
tmux new-session -d -s palm-oil-orchestration
tmux new-window -t palm-oil-orchestration -n amp
tmux new-window -t palm-oil-orchestration -n antigravity
tmux new-window -t palm-oil-orchestration -n codex
```

### Recommandation
Réutiliser la session existante en spécifiant le chemin complet dans les prompts.
