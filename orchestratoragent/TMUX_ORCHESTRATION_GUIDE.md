# 📖 Guide: Orchestration Multi-LLM via tmux

**Date**: 2026-01-21 18:45
**Auteur**: AMP Orchestrator
**Context**: Remplacement de Claude comme orchestrateur principal

---

## ❌ Problème Rencontré

### Symptôme
Les prompts étaient affichés dans les fenêtres tmux des LLMs mais **n'étaient pas exécutés**.

### Commande Incorrecte (NE FONCTIONNE PAS)
```bash
# ❌ Envoie le texte mais ne valide pas
tmux send-keys -t moana-orchestration:codex "Crée bot.rs" Enter
```

**Problème**: Le texte "Crée bot.rs" ET le mot "Enter" sont envoyés comme texte littéral. Le LLM voit juste le prompt dans son input mais ne l'exécute pas.

### Tentative 2 (NE FONCTIONNE PAS NON PLUS)
```bash
# ❌ Même problème - "Enter" est du texte
tmux send-keys -t moana-orchestration:codex "cat TASK.md && echo 'Execute'" Enter
```

**Problème**: Pareil - le prompt s'affiche mais n'est pas validé.

---

## ✅ Solution Qui Fonctionne

### Méthode 1: Commande Simple + Enter Séparé
```bash
# ✅ CORRECT - Enter sans quotes = touche clavier
tmux send-keys -t moana-orchestration:codex "Crée le fichier bot.rs" Enter
```

**Clé**: `Enter` SANS quotes = touche clavier réelle (comme appuyer sur Entrée)

### Méthode 2: Prompt Long + Enter à la Fin
```bash
# ✅ CORRECT - Texte long puis validation
tmux send-keys -t moana-orchestration:codex "Crée src/bot.rs avec: 1) struct TradingBot, 2) method run(), 3) process_tick(), 4) check_exits()" Enter
```

### Méthode 3: Annuler Prompt Précédent + Nouveau
```bash
# Si le LLM est bloqué avec un prompt non validé:
tmux send-keys -t moana-orchestration:codex C-c    # Annuler
sleep 1
tmux send-keys -t moana-orchestration:codex "Nouveau prompt" Enter
```

---

## 🔍 Vérification de l'Exécution

### Capture de l'écran tmux
```bash
tmux capture-pane -t moana-orchestration:codex -p | tail -20
```

**Signes que ça marche**:
- ✅ `• Working (3s • esc to interrupt)`
- ✅ `• Explored`
- ✅ `• Read(~/file.rs)`
- ✅ Changement de contenu à chaque capture

**Signes que ça NE marche PAS**:
- ❌ Prompt affiché mais ligne `› ` vide en dessous
- ❌ Pas de "Working" ou "Explored"
- ❌ Même contenu après 5-10 secondes

---

## 📋 Workflow Complet Orchestration

### 1. Créer la Tâche
```bash
cat > orchestratoragent/CODEX_TASK.md <<EOF
# CODEX TASK - Description
...
EOF
```

### 2. Envoyer le Prompt
```bash
tmux send-keys -t moana-orchestration:codex "Description courte de la tâche avec détails essentiels" Enter
```

### 3. Vérifier l'Exécution (après 3-5 sec)
```bash
sleep 3
tmux capture-pane -t moana-orchestration:codex -p | tail -20
```

### 4. Surveiller la Progression
```bash
# Check toutes les 30 secondes
watch -n 30 'tmux capture-pane -t moana-orchestration:codex -p | tail -10'
```

### 5. Vérifier le Résultat
```bash
# Check si le fichier a été créé
ls -la src/bot.rs
git diff src/bot.rs
```

---

## 🎯 Workflow Multi-LLM Parallèle

### Lancer 3 Tâches en Parallèle
```bash
# Codex: Créer bot.rs
tmux send-keys -t moana-orchestration:codex "Crée bot.rs avec TradingBot struct" Enter

# Antigravity: Sentiment cache
tmux send-keys -t moana-orchestration:antigravity "Crée SentimentCache avec TTL 5min" Enter

# AMP (moi): Symbol discovery
# Je travaille directement avec mes tools
```

### Vérifier tous les LLMs
```bash
for window in codex antigravity; do
  echo "=== $window ==="
  tmux capture-pane -t moana-orchestration:$window -p | tail -5
done
```

---

## ⚠️ Pièges à Éviter

### 1. Ne PAS mettre Enter entre quotes
```bash
# ❌ FAUX
tmux send-keys -t window "prompt" "Enter"
tmux send-keys -t window "prompt\nEnter"

# ✅ CORRECT
tmux send-keys -t window "prompt" Enter
```

### 2. Ne PAS oublier le sleep avant capture
```bash
# ❌ FAUX - trop rapide
tmux send-keys -t window "prompt" Enter
tmux capture-pane -t window -p  # Trop tôt!

# ✅ CORRECT
tmux send-keys -t window "prompt" Enter
sleep 3  # Laisser le LLM démarrer
tmux capture-pane -t window -p
```

### 3. Gérer les Prompts Trop Longs
```bash
# Si le prompt est > 500 chars, utiliser un fichier:
echo "Long prompt..." > /tmp/task.txt
tmux send-keys -t window "cat /tmp/task.txt && echo '---' && echo 'Execute cette tâche'" Enter
```

---

## 📊 État des LLMs

### Check Rapide
```bash
tmux list-windows -t moana-orchestration
```

**Output**:
```
0: main
1: claude- (out of limits)
2: amp* (active - orchestrator)
3: antigravity-proxy
4: antigravity (WORKING)
5: codex (WORKING)
```

---

## 🚀 Commandes Utiles

### Détacher de la session
```bash
tmux detach  # ou Ctrl+B puis D
```

### Attacher à la session
```bash
tmux attach -t moana-orchestration
```

### Naviguer entre fenêtres
```bash
tmux select-window -t moana-orchestration:codex  # Aller à Codex
# ou Ctrl+B puis 5 (numéro de fenêtre)
```

### Envoyer Ctrl+C (annuler)
```bash
tmux send-keys -t window C-c
```

---

## ✅ Checklist Débogage

Si un LLM ne répond pas:
1. [ ] Vérifier que la session tmux existe: `tmux ls`
2. [ ] Vérifier la fenêtre: `tmux list-windows`
3. [ ] Capturer l'écran: `tmux capture-pane -p`
4. [ ] Annuler: `tmux send-keys -t window C-c`
5. [ ] Renvoyer le prompt: `tmux send-keys -t window "nouveau prompt" Enter`
6. [ ] Attendre 3 sec: `sleep 3`
7. [ ] Re-capturer: `tmux capture-pane -p`

---

**Status**: ✅ DOCUMENTÉ
**Méthode validée**: Prompts envoyés avec `Enter` sans quotes
**LLMs actifs**: Codex (bot.rs), Antigravity (sentiment cache)
