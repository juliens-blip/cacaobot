# 🎯 Skill: Orchestration Multi-Agents Complète

## Vue d'ensemble

Cette skill permet à un agent Claude d'orchestrer une équipe de LLMs (AMP, Codex, Antigravity) via tmux, de surveiller son propre quota, et de transférer automatiquement l'orchestration à AMP quand nécessaire.

**Capacités couvertes :**
1. Communication inter-agents via tmux
2. Monitoring du quota de session Claude
3. Handoff automatique vers AMP à 93%
4. Reprise de l'orchestration par AMP

---

# PARTIE A : COMMUNICATION INTER-AGENTS

## A.1 Architecture Tmux

```
┌─────────────────────────────────────────────────────────────┐
│              Session Tmux: orchestration-palm-oil-bot       │
├─────────────────────────────────────────────────────────────┤
│  Window 0: main         │ bash (scripts utilitaires)        │
│  Window 1: claude       │ Claude Code (orchestrateur)       │
│  Window 2: amp          │ AMP CLI (backup orchestrateur)    │
│  Window 3: antigravity  │ Claude via proxy (worker)         │
│  Window 4: codex        │ OpenAI Codex (worker)             │
└─────────────────────────────────────────────────────────────┘
```

## A.2 Commandes de base

### Découvrir l'environnement

```bash
# Lister les sessions tmux
tmux list-sessions

# Lister les fenêtres d'une session
tmux list-windows -t orchestration-palm-oil-bot

# Lister les panes avec les processus
tmux list-panes -t orchestration-palm-oil-bot -a -F "#{window_name}: #{pane_current_command}"
```

### Envoyer un message à un agent

```bash
# Syntaxe complète
tmux send-keys -t <session>:<window> "<message>" Enter

# Exemples concrets
tmux send-keys -t orchestration-palm-oil-bot:amp "Bonjour, quel est ton status ?" Enter
tmux send-keys -t orchestration-palm-oil-bot:antigravity "Continue TASK-PO-011" Enter
tmux send-keys -t orchestration-palm-oil-bot:codex "Lance cargo check" Enter
```

### Lire la réponse d'un agent

```bash
# Capturer les 30 dernières lignes
tmux capture-pane -t orchestration-palm-oil-bot:amp -p | tail -30

# Capturer tout l'historique visible
tmux capture-pane -t orchestration-palm-oil-bot:amp -p -S -500
```

### Soumettre un message en attente

Parfois le message est dans le buffer mais pas soumis. Envoyer Enter séparément :

```bash
tmux send-keys -t orchestration-palm-oil-bot:amp Enter
```

## A.3 Patterns de communication

### Envoyer à tous les agents

```bash
#!/bin/bash
SESSION="orchestration-palm-oil-bot"
AGENTS=("amp" "antigravity" "codex")
MESSAGE="$1"

for agent in "${AGENTS[@]}"; do
    echo "Envoi à $agent..."
    tmux send-keys -t "$SESSION:$agent" "$MESSAGE" Enter
done
```

### Vérifier toutes les réponses

```bash
#!/bin/bash
SESSION="orchestration-palm-oil-bot"
AGENTS=("amp" "antigravity" "codex")

for agent in "${AGENTS[@]}"; do
    echo "=== $agent ==="
    tmux capture-pane -t "$SESSION:$agent" -p | tail -15
    echo ""
done
```

### Communication via fichier partagé

Certains agents n'ont pas `/memory`. Utiliser un fichier partagé :

```bash
# Demander aux agents d'écrire dans un fichier
tmux send-keys -t orchestration-palm-oil-bot:amp \
    "Écris ton status dans orchestratoragent/AGENT_RESPONSES.md" Enter

# Vérifier les réponses
cat orchestratoragent/AGENT_RESPONSES.md
```

**Format du fichier `AGENT_RESPONSES.md` :**
```markdown
# Agent Responses

## AMP - 2026-01-22 10:00
Bonjour, je suis prêt.

## Codex - 2026-01-22 10:01
Bonjour, cargo check en cours.

## Antigravity - 2026-01-22 10:02
Bonjour, TASK-PO-011 à 50%.
```

---

# PARTIE B : MONITORING DU QUOTA

## B.1 Où trouver le quota

Le quota de session Claude est affiché dans le **footer** de la fenêtre tmux :

```
You've used 93% of your session limit · resets 2pm (Europe/Paris)
```

**Important** : Cette info est côté serveur Anthropic, pas stockée localement.

## B.2 Script de vérification : `check_claude_quota.sh`

**Chemin** : `orchestratoragent/scripts/check_claude_quota.sh`

```bash
#!/bin/bash
# ============================================================================
# check_claude_quota.sh - Vérifie le quota de session Claude
# ============================================================================
# Usage: ./check_claude_quota.sh [session] [window] [alert_threshold]
# ============================================================================

SESSION="${1:-orchestration-palm-oil-bot}"
WINDOW="${2:-claude}"
ALERT_THRESHOLD="${3:-93}"

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Fonction de capture du quota
get_quota() {
    local content=$(tmux capture-pane -t "$SESSION:$WINDOW" -p 2>/dev/null)

    # Pattern: "used XX% of your session" ou "used XX%"
    local quota=$(echo "$content" | grep -oE "used [0-9]+%" | grep -oE "[0-9]+" | tail -1)

    echo "$quota"
}

QUOTA=$(get_quota)

if [[ -z "$QUOTA" ]] || ! [[ "$QUOTA" =~ ^[0-9]+$ ]]; then
    echo -e "${RED}❌ Impossible de lire le quota${NC}"
    exit 1
fi

# Barre de progression
BAR_WIDTH=40
FILLED=$((QUOTA * BAR_WIDTH / 100))
EMPTY=$((BAR_WIDTH - FILLED))
BAR=$(printf "%${FILLED}s" | tr ' ' '█')$(printf "%${EMPTY}s" | tr ' ' '░')

# Couleur selon le niveau
if [[ "$QUOTA" -lt 50 ]]; then
    COLOR=$GREEN; STATUS="✅ OK"
elif [[ "$QUOTA" -lt 75 ]]; then
    COLOR=$YELLOW; STATUS="⚡ Attention"
elif [[ "$QUOTA" -lt "$ALERT_THRESHOLD" ]]; then
    COLOR=$YELLOW; STATUS="⚠️ Élevé"
else
    COLOR=$RED; STATUS="🚨 ALERTE - Handoff recommandé"
fi

# Affichage
echo -e "${CYAN}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}      📊 CLAUDE SESSION QUOTA                      ${CYAN}║${NC}"
echo -e "${CYAN}╠═══════════════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║${NC} Quota:   ${COLOR}${QUOTA}%${NC} of session limit"
echo -e "${CYAN}║${NC} [${COLOR}${BAR}${NC}]"
echo -e "${CYAN}║${NC} Status:  ${COLOR}${STATUS}${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════╝${NC}"

# Export pour utilisation programmatique
echo "QUOTA=$QUOTA"
echo "ALERT=$([[ $QUOTA -ge $ALERT_THRESHOLD ]] && echo "true" || echo "false")"

# Code de sortie
[[ "$QUOTA" -ge "$ALERT_THRESHOLD" ]] && exit 2 || exit 0
```

### Utilisation

```bash
# Vérification simple
./orchestratoragent/scripts/check_claude_quota.sh

# Avec paramètres personnalisés
./orchestratoragent/scripts/check_claude_quota.sh ma-session claude 90

# Dans un script, récupérer le quota
QUOTA=$(./check_claude_quota.sh 2>/dev/null | grep "^QUOTA=" | cut -d= -f2)
echo "Quota actuel: $QUOTA%"
```

## B.3 Watchdog continu : `quota_watchdog.sh`

**Chemin** : `orchestratoragent/scripts/quota_watchdog.sh`

```bash
#!/bin/bash
# ============================================================================
# quota_watchdog.sh - Surveillance continue du quota avec alerte à 93%
# ============================================================================
# Usage: nohup ./quota_watchdog.sh &
# ============================================================================

SESSION="${1:-orchestration-palm-oil-bot}"
WINDOW="${2:-claude}"
ALERT_THRESHOLD="${3:-93}"
CHECK_INTERVAL="${4:-30}"

LOG_FILE="/tmp/claude_quota_watchdog.log"
QUOTA_FILE="/tmp/claude_current_quota"
ALERT_TRIGGERED="/tmp/claude_quota_alert_triggered"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"; }

get_quota() {
    tmux capture-pane -t "$SESSION:$WINDOW" -p 2>/dev/null | \
        grep -oE "used [0-9]+%" | grep -oE "[0-9]+" | tail -1
}

send_alert() {
    local quota=$1

    # Éviter alertes répétées
    [[ -f "$ALERT_TRIGGERED" ]] && [[ "$(cat $ALERT_TRIGGERED)" == "$quota" ]] && return
    echo "$quota" > "$ALERT_TRIGGERED"

    log "🚨 ALERTE: Quota à ${quota}%!"

    # Notification desktop
    notify-send -u critical "🚨 Claude Quota" "Session à ${quota}%!" 2>/dev/null

    # Son d'alerte
    paplay /usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga 2>/dev/null &

    # Notifier AMP
    tmux send-keys -t "$SESSION:amp" \
        "⚠️ ALERTE: Claude est à ${quota}%. Prépare-toi pour le handoff." Enter
}

log "Watchdog démarré (seuil: ${ALERT_THRESHOLD}%, intervalle: ${CHECK_INTERVAL}s)"

while true; do
    QUOTA=$(get_quota)

    if [[ -n "$QUOTA" ]] && [[ "$QUOTA" =~ ^[0-9]+$ ]]; then
        echo "$QUOTA" > "$QUOTA_FILE"

        if [[ "$QUOTA" -ge "$ALERT_THRESHOLD" ]]; then
            log "🚨 ${QUOTA}%"
            send_alert "$QUOTA"
        else
            log "✅ ${QUOTA}%"
        fi
    fi

    sleep "$CHECK_INTERVAL"
done
```

### Lancer le watchdog

```bash
# En premier plan (test)
./orchestratoragent/scripts/quota_watchdog.sh

# En background (production)
nohup ./orchestratoragent/scripts/quota_watchdog.sh > /tmp/watchdog.out 2>&1 &

# Vérifier qu'il tourne
ps aux | grep quota_watchdog

# Voir les logs
tail -f /tmp/claude_quota_watchdog.log

# Lire le quota actuel
cat /tmp/claude_current_quota
```

---

# PARTIE C : HANDOFF VERS AMP

## C.1 Quand faire un handoff

| Quota | Action |
|-------|--------|
| < 75% | ✅ Travail normal |
| 75-84% | ⚡ Surveiller, éviter nouvelles grosses tâches |
| 85-92% | ⚠️ Terminer tâches en cours, préparer handoff |
| ≥ 93% | 🚨 **HANDOFF IMMÉDIAT** |

## C.2 Procédure de handoff complète

### Étape 1 : Vérifier le quota

```bash
./orchestratoragent/scripts/check_claude_quota.sh
# Si ALERT=true, continuer avec le handoff
```

### Étape 2 : Créer le fichier de handoff

Claude doit créer le fichier `orchestratoragent/HANDOFF_TO_AMP.md` :

```markdown
# 🔄 HANDOFF ORCHESTRATION: Claude → AMP

**Date**: 2026-01-22 12:00 CET
**Raison**: Quota session Claude à 93%
**Nouveau orchestrateur**: AMP

---

## 📋 CONTEXTE

### Ce qui a été fait
1. [Tâche 1 complétée]
2. [Tâche 2 en cours - 80%]
3. [Communication avec agents établie]

### Status des agents

| Agent | Status | Tâche en cours |
|-------|--------|----------------|
| Antigravity | 🟢 Actif | TASK-PO-011 (Strategy analysis) |
| Codex | 🟢 Actif | TASK-PO-013 (Code review) |

---

## 🎯 TÂCHES À REPRENDRE

| Priorité | Tâche | Description |
|----------|-------|-------------|
| 🔴 HIGH | Terminer TASK-PO-011 | Vérifier l'avancement d'Antigravity |
| 🟡 MED | Valider compilation | Attendre résultat de Codex |
| 🟢 LOW | Documentation | Mettre à jour CLAUDE.md |

---

## 📚 FICHIERS ESSENTIELS À LIRE

```bash
# Instructions du projet
cat /home/julien/Documents/palm-oil-bot/CLAUDE.md

# Réponses des agents
cat /home/julien/Documents/palm-oil-bot/orchestratoragent/AGENT_RESPONSES.md

# Skills disponibles
ls /home/julien/Documents/palm-oil-bot/orchestratoragent/skills/
```

---

## 🔧 COMMANDES UTILES

```bash
# Envoyer message à un agent
tmux send-keys -t orchestration-palm-oil-bot:<agent> "<message>" Enter

# Voir sortie d'un agent
tmux capture-pane -t orchestration-palm-oil-bot:<agent> -p | tail -30

# Soumettre message en attente
tmux send-keys -t orchestration-palm-oil-bot:<agent> Enter

# Vérifier quota de Claude (pour savoir quand reprendre)
./orchestratoragent/scripts/check_claude_quota.sh
```

---

## ⚡ ACTIONS IMMÉDIATES POUR AMP

1. ✅ Lire ce fichier
2. 📖 Lire CLAUDE.md
3. 📊 Vérifier status des agents
4. 🔄 Reprendre la coordination
5. 📝 Mettre à jour AGENT_RESPONSES.md avec "AMP a pris le relais"
```

### Étape 3 : Envoyer le message de handoff à AMP

```bash
tmux send-keys -t orchestration-palm-oil-bot:amp "🔄 HANDOFF ORCHESTRATION: Tu prends le relais comme orchestrateur.

ACTIONS IMMÉDIATES:
1. Lis le fichier de handoff:
   cat /home/julien/Documents/palm-oil-bot/orchestratoragent/HANDOFF_TO_AMP.md

2. Lis les instructions du projet:
   cat /home/julien/Documents/palm-oil-bot/CLAUDE.md

3. Vérifie le status des agents:
   tmux capture-pane -t orchestration-palm-oil-bot:antigravity -p | tail -20
   tmux capture-pane -t orchestration-palm-oil-bot:codex -p | tail -20

4. Tes tâches:
   - Coordonner Antigravity et Codex
   - Vérifier leurs avancements
   - Distribuer les nouvelles tâches

Commence maintenant par lire le fichier HANDOFF_TO_AMP.md" Enter
```

### Étape 4 : Vérifier que AMP a pris le relais

```bash
# Attendre 15-20 secondes
sleep 15

# Vérifier la réponse d'AMP
tmux capture-pane -t orchestration-palm-oil-bot:amp -p | tail -30

# Vérifier qu'il travaille (doit voir des actions)
tmux capture-pane -t orchestration-palm-oil-bot:amp -p | grep -i "read\|cat\|create"
```

## C.3 Script automatisé : `auto_handoff_to_amp.sh`

```bash
#!/bin/bash
# ============================================================================
# auto_handoff_to_amp.sh - Handoff automatique Claude → AMP
# ============================================================================

SESSION="orchestration-palm-oil-bot"
PROJECT_DIR="/home/julien/Documents/palm-oil-bot"
HANDOFF_FILE="$PROJECT_DIR/orchestratoragent/HANDOFF_TO_AMP.md"

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║        🔄 AUTO HANDOFF CLAUDE → AMP                       ║"
echo "╚═══════════════════════════════════════════════════════════╝"

# 1. Vérifier quota
QUOTA=$(tmux capture-pane -t "$SESSION:claude" -p | grep -oE "used [0-9]+%" | grep -oE "[0-9]+" | tail -1)
echo "[1/5] Quota actuel: ${QUOTA:-inconnu}%"

# 2. Créer fichier de handoff
echo "[2/5] Création fichier de handoff..."
cat > "$HANDOFF_FILE" << EOF
# 🔄 HANDOFF: Claude → AMP

**Date**: $(date '+%Y-%m-%d %H:%M') CET
**Quota Claude**: ${QUOTA}%

## Actions immédiates
1. Lire ce fichier ✅
2. Lire CLAUDE.md: cat $PROJECT_DIR/CLAUDE.md
3. Vérifier agents: tmux capture-pane -t $SESSION:<agent> -p | tail -20
4. Reprendre coordination
EOF
echo "   Fichier créé: $HANDOFF_FILE"

# 3. Envoyer message à AMP
echo "[3/5] Notification à AMP..."
tmux send-keys -t "$SESSION:amp" "🔄 HANDOFF: Tu prends le relais. Lis: cat $HANDOFF_FILE puis cat $PROJECT_DIR/CLAUDE.md" Enter

# 4. Attendre
echo "[4/5] Attente prise en charge (15s)..."
sleep 15

# 5. Vérifier
echo "[5/5] Vérification réponse AMP..."
tmux capture-pane -t "$SESSION:amp" -p | tail -15

echo ""
echo "✅ Handoff envoyé. Vérifier que AMP travaille."
```

---

# PARTIE D : GUIDE POUR AMP (NOUVEL ORCHESTRATEUR)

## D.1 À la réception du handoff

```bash
# 1. Lire le fichier de handoff
cat /home/julien/Documents/palm-oil-bot/orchestratoragent/HANDOFF_TO_AMP.md

# 2. Lire les instructions du projet
cat /home/julien/Documents/palm-oil-bot/CLAUDE.md

# 3. Lister les agents
tmux list-windows -t orchestration-palm-oil-bot

# 4. Vérifier chaque agent
for agent in antigravity codex claude; do
    echo "=== $agent ==="
    tmux capture-pane -t orchestration-palm-oil-bot:$agent -p | tail -15
done
```

## D.2 Coordonner les agents

```bash
# Demander un status
tmux send-keys -t orchestration-palm-oil-bot:antigravity "Quel est ton avancement sur ta tâche actuelle ?" Enter

# Assigner une tâche
tmux send-keys -t orchestration-palm-oil-bot:codex "Lance cargo test et rapporte les résultats" Enter

# Vérifier la réponse (après 30s)
sleep 30
tmux capture-pane -t orchestration-palm-oil-bot:codex -p | tail -20
```

## D.3 Vérifier si Claude peut reprendre

```bash
# Vérifier le quota de Claude
./orchestratoragent/scripts/check_claude_quota.sh

# Si quota < 50%, Claude peut reprendre
# Le quota se reset à 14h (Europe/Paris)
```

## D.4 Rendre le contrôle à Claude

```bash
# 1. Créer fichier de retour
cat > orchestratoragent/HANDOFF_TO_CLAUDE.md << 'EOF'
# 🔄 HANDOFF RETOUR: AMP → Claude

**Date**: $(date)
**Raison**: Quota Claude réinitialisé

## Fait pendant le handoff
- [Actions réalisées]

## Status agents
- Antigravity: [status]
- Codex: [status]
EOF

# 2. Notifier Claude
tmux send-keys -t orchestration-palm-oil-bot:claude "🔄 RETOUR: Tu reprends l'orchestration. Lis orchestratoragent/HANDOFF_TO_CLAUDE.md" Enter
```

---

# PARTIE E : RÉFÉRENCE RAPIDE

## E.1 Commandes essentielles

```bash
# === COMMUNICATION ===
# Envoyer message
tmux send-keys -t orchestration-palm-oil-bot:<agent> "<message>" Enter

# Lire réponse
tmux capture-pane -t orchestration-palm-oil-bot:<agent> -p | tail -30

# Soumettre message en attente
tmux send-keys -t orchestration-palm-oil-bot:<agent> Enter

# === QUOTA ===
# Vérifier quota
./orchestratoragent/scripts/check_claude_quota.sh

# Lancer watchdog
nohup ./orchestratoragent/scripts/quota_watchdog.sh &

# Lire quota (si watchdog actif)
cat /tmp/claude_current_quota

# === HANDOFF ===
# Handoff automatique
./orchestratoragent/scripts/auto_handoff_to_amp.sh

# Vérifier si AMP a pris le relais
tmux capture-pane -t orchestration-palm-oil-bot:amp -p | tail -20
```

## E.2 Fichiers importants

| Fichier | Description |
|---------|-------------|
| `CLAUDE.md` | Instructions du projet |
| `orchestratoragent/AGENT_RESPONSES.md` | Réponses des agents |
| `orchestratoragent/HANDOFF_TO_AMP.md` | Fichier de handoff |
| `orchestratoragent/skills/ORCHESTRATION_COMPLETE.md` | Ce fichier |
| `/tmp/claude_current_quota` | Quota actuel |
| `/tmp/claude_quota_watchdog.log` | Log du watchdog |

## E.3 Seuils de quota

| Quota | Status | Action |
|-------|--------|--------|
| < 50% | 🟢 OK | Travail normal |
| 50-74% | 🟡 Attention | Surveiller |
| 75-92% | 🟠 Élevé | Préparer handoff |
| ≥ 93% | 🔴 ALERTE | **Handoff immédiat** |

## E.4 Troubleshooting

| Problème | Solution |
|----------|----------|
| Quota non visible | `tmux capture-pane -p -S -100 \| grep used` |
| Agent ne répond pas | `tmux send-keys Enter` puis attendre |
| Message non soumis | Envoyer `Enter` séparément |
| Session introuvable | `tmux list-sessions` pour vérifier le nom |

---

# PARTIE F : SESSION DÉMO (22 janvier 2026)

## F.1 Ce qui a été découvert

1. **Le quota n'est pas stocké localement** - Il est côté serveur Anthropic
2. **Le quota est visible dans le footer tmux** - Pattern: `used XX%`
3. **Envoyer /config à soi-même crée une boucle** - Le message revient comme input
4. **Solution : parser le footer avec grep**

## F.2 Commandes exactes utilisées

```bash
# Découverte du quota dans le footer
tmux capture-pane -t orchestration-palm-oil-bot:claude -p | grep -oE "used [0-9]+%"
# Résultat: used 93%

# Test du script de quota
./orchestratoragent/scripts/check_claude_quota.sh
# Résultat: QUOTA=93, ALERT=true

# Envoi du handoff à AMP
tmux send-keys -t orchestration-palm-oil-bot:amp "🔄 HANDOFF..." Enter

# Vérification réponse AMP
tmux capture-pane -t orchestration-palm-oil-bot:amp -p | tail -30
# Résultat: AMP a commencé à créer QUOTA_MONITORING.md
```

## F.3 Leçons apprises

1. **Toujours vérifier que le message est soumis** - Parfois Enter ne passe pas
2. **Attendre 15-30s pour les réponses** - Les LLMs ont besoin de temps
3. **Utiliser un fichier partagé** - Plus fiable que /memory
4. **Le watchdog doit tourner en background** - Pour alertes automatiques

---

**Auteur**: Claude (Orchestrateur)
**Version**: 2.0
**Dernière mise à jour**: 2026-01-22 12:15 CET
**Fichier**: `orchestratoragent/skills/ORCHESTRATION_COMPLETE.md`
