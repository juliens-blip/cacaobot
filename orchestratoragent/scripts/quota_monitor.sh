#!/bin/bash
# ============================================================================
# quota_monitor.sh - Moniteur de quota Claude en temps réel
# ============================================================================
# Ce script capture le quota "Current session" depuis /config
# et déclenche une alarme à 93%
# ============================================================================

set -e

ALERT_THRESHOLD=${1:-93}
CHECK_INTERVAL=${2:-60}  # secondes entre chaque vérification
SESSION="orchestration-palm-oil-bot"
WINDOW="claude"
LOG_FILE="/tmp/claude_quota.log"
ALERT_FILE="/tmp/claude_quota_alert"

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║        CLAUDE QUOTA MONITOR - DÉMARRAGE                   ║"
echo "╠═══════════════════════════════════════════════════════════╣"
echo "║ Seuil d'alerte : ${ALERT_THRESHOLD}%                                       ║"
echo "║ Intervalle     : ${CHECK_INTERVAL}s                                        ║"
echo "║ Log file       : ${LOG_FILE}                        ║"
echo "╚═══════════════════════════════════════════════════════════╝"

# Fonction pour extraire le quota depuis l'écran tmux
extract_quota() {
    local pane_content=$(tmux capture-pane -t "$SESSION:$WINDOW" -p 2>/dev/null)

    # Chercher le pattern "XX% used" pour Current session
    local quota=$(echo "$pane_content" | grep -oE "[0-9]+% used" | head -1 | grep -oE "[0-9]+")

    echo "$quota"
}

# Fonction pour envoyer /config et capturer
trigger_config_display() {
    # Envoyer Escape d'abord pour annuler toute saisie en cours
    tmux send-keys -t "$SESSION:$WINDOW" Escape
    sleep 0.5

    # Envoyer /config
    tmux send-keys -t "$SESSION:$WINDOW" "/config" Enter
    sleep 2

    # Capturer l'écran
    local content=$(tmux capture-pane -t "$SESSION:$WINDOW" -p 2>/dev/null)

    # Chercher "Current session" suivi de XX% used
    local session_quota=$(echo "$content" | grep -A1 "Current session" | grep -oE "[0-9]+%" | head -1 | tr -d '%')

    # Fermer le menu config avec Escape
    tmux send-keys -t "$SESSION:$WINDOW" Escape
    sleep 0.5

    echo "$session_quota"
}

# Fonction d'alerte
trigger_alert() {
    local quota=$1
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    echo -e "${RED}🚨 ALERTE: Quota à ${quota}% - Seuil ${ALERT_THRESHOLD}% dépassé!${NC}"

    # Écrire dans le fichier d'alerte
    echo "$timestamp - ALERT: Quota at ${quota}%" >> "$ALERT_FILE"

    # Notification sonore (si disponible)
    if command -v paplay &> /dev/null; then
        paplay /usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga 2>/dev/null &
    elif command -v aplay &> /dev/null; then
        aplay -q /usr/share/sounds/alsa/Front_Center.wav 2>/dev/null &
    fi

    # Notification desktop (si disponible)
    if command -v notify-send &> /dev/null; then
        notify-send -u critical "Claude Quota Alert" "Session quota at ${quota}%! Consider handoff to AMP."
    fi

    # Envoyer un message dans la fenêtre AMP pour préparer le handoff
    tmux send-keys -t "$SESSION:amp" "⚠️ ALERTE: Le quota de Claude est à ${quota}%. Prépare-toi pour un handoff potentiel." Enter 2>/dev/null || true
}

# Fonction pour logger
log_quota() {
    local quota=$1
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "$timestamp - Session quota: ${quota}%" >> "$LOG_FILE"
}

# Boucle principale
monitor_loop() {
    local last_alert_quota=0

    while true; do
        # Essayer d'abord de lire depuis l'écran existant
        local quota=$(extract_quota)

        # Si pas trouvé, déclencher /config
        if [[ -z "$quota" ]] || [[ "$quota" == "0" ]]; then
            echo "[$(date '+%H:%M:%S')] Quota non visible, trigger /config..."
            quota=$(trigger_config_display)
        fi

        if [[ -n "$quota" ]] && [[ "$quota" =~ ^[0-9]+$ ]]; then
            echo -e "[$(date '+%H:%M:%S')] Current session: ${quota}%"
            log_quota "$quota"

            # Vérifier le seuil
            if [[ "$quota" -ge "$ALERT_THRESHOLD" ]] && [[ "$quota" -ne "$last_alert_quota" ]]; then
                trigger_alert "$quota"
                last_alert_quota=$quota
            fi

            # Afficher barre de progression
            local bar_filled=$((quota * 40 / 100))
            local bar_empty=$((40 - bar_filled))
            local bar=$(printf "%${bar_filled}s" | tr ' ' '█')$(printf "%${bar_empty}s" | tr ' ' '░')

            if [[ "$quota" -lt 50 ]]; then
                echo -e "         [${GREEN}${bar}${NC}]"
            elif [[ "$quota" -lt 75 ]]; then
                echo -e "         [${YELLOW}${bar}${NC}]"
            else
                echo -e "         [${RED}${bar}${NC}]"
            fi
        else
            echo "[$(date '+%H:%M:%S')] Impossible de lire le quota"
        fi

        sleep "$CHECK_INTERVAL"
    done
}

# Démarrer le monitoring
monitor_loop
