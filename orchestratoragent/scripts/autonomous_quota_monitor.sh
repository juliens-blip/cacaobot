#!/bin/bash
# ============================================================================
# autonomous_quota_monitor.sh - Moniteur autonome de quota Claude
# ============================================================================
# Lance une session Claude dédiée pour vérifier le quota et le rapporter
# ============================================================================

ALERT_THRESHOLD=${1:-93}
CHECK_INTERVAL=${2:-120}  # 2 minutes par défaut
SESSION="orchestration-palm-oil-bot"
MAIN_WINDOW="claude"
QUOTA_FILE="/tmp/claude_current_quota"
ALERT_FILE="/tmp/claude_quota_alert"

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║   AUTONOMOUS QUOTA MONITOR - DÉMARRAGE                    ║"
echo "╠═══════════════════════════════════════════════════════════╣"
echo "║ Seuil d'alerte  : ${ALERT_THRESHOLD}%                                      ║"
echo "║ Intervalle      : ${CHECK_INTERVAL}s                                       ║"
echo "║ Fichier quota   : ${QUOTA_FILE}                   ║"
echo "╚═══════════════════════════════════════════════════════════╝"

# Créer une fenêtre dédiée si elle n'existe pas
setup_quota_window() {
    if ! tmux list-windows -t "$SESSION" 2>/dev/null | grep -q "quota-monitor"; then
        echo "[SETUP] Création de la fenêtre quota-monitor..."
        tmux new-window -t "$SESSION" -n "quota-monitor" -d
        sleep 2

        # Lancer Claude dans cette fenêtre
        tmux send-keys -t "$SESSION:quota-monitor" "cd /home/julien/Documents/palm-oil-bot && claude" Enter
        sleep 5
        echo "[SETUP] Session Claude quota-monitor prête"
    fi
}

# Fonction pour obtenir le quota via la session dédiée
get_quota_from_dedicated_session() {
    # Envoyer /config à la session dédiée
    tmux send-keys -t "$SESSION:quota-monitor" "/config" Enter
    sleep 3

    # Capturer l'écran
    local content=$(tmux capture-pane -t "$SESSION:quota-monitor" -p 2>/dev/null)

    # Parser le quota
    local quota=$(echo "$content" | grep -A2 "Current session" | grep -oE "[0-9]+%" | head -1 | tr -d '%')

    # Fermer le menu
    tmux send-keys -t "$SESSION:quota-monitor" Escape
    sleep 1

    echo "$quota"
}

# Fonction pour rapporter le quota à la session principale
report_quota_to_main() {
    local quota=$1
    local timestamp=$(date '+%H:%M:%S')

    # Écrire dans le fichier partagé
    echo "$quota" > "$QUOTA_FILE"
    echo "$(date '+%Y-%m-%d %H:%M:%S') $quota%" >> /tmp/claude_quota_history.log

    # Si alerte, notifier la session principale
    if [[ "$quota" -ge "$ALERT_THRESHOLD" ]]; then
        echo "🚨 ALERTE: Quota à ${quota}%!"
        echo "$(date '+%Y-%m-%d %H:%M:%S') ALERT: ${quota}%" >> "$ALERT_FILE"

        # Envoyer une alerte à la session principale Claude
        tmux send-keys -t "$SESSION:$MAIN_WINDOW" "# ⚠️ QUOTA ALERT: Session à ${quota}% - Envisager handoff vers AMP" Enter

        # Notifier AMP aussi
        tmux send-keys -t "$SESSION:amp" "Le quota de Claude est à ${quota}%. Prépare-toi pour prendre le relais si nécessaire." Enter

        # Notification système
        notify-send -u critical "Claude Quota Alert" "Session at ${quota}%!" 2>/dev/null || true
    fi
}

# Fonction d'affichage
display_status() {
    local quota=$1
    local bar_filled=$((quota * 30 / 100))
    local bar_empty=$((30 - bar_filled))
    local bar=$(printf "%${bar_filled}s" | tr ' ' '█')$(printf "%${bar_empty}s" | tr ' ' '░')

    if [[ "$quota" -lt 50 ]]; then
        color="\033[0;32m"  # vert
    elif [[ "$quota" -lt 75 ]]; then
        color="\033[1;33m"  # jaune
    elif [[ "$quota" -lt "$ALERT_THRESHOLD" ]]; then
        color="\033[0;33m"  # orange
    else
        color="\033[0;31m"  # rouge
    fi

    echo -e "[$(date '+%H:%M:%S')] Session: ${color}${quota}%\033[0m [${bar}]"
}

# Boucle principale
main_loop() {
    setup_quota_window

    while true; do
        quota=$(get_quota_from_dedicated_session)

        if [[ -n "$quota" ]] && [[ "$quota" =~ ^[0-9]+$ ]]; then
            display_status "$quota"
            report_quota_to_main "$quota"
        else
            echo "[$(date '+%H:%M:%S')] ⚠️ Impossible de lire le quota"
        fi

        sleep "$CHECK_INTERVAL"
    done
}

# Gestion du signal d'arrêt
cleanup() {
    echo ""
    echo "[CLEANUP] Arrêt du moniteur..."
    tmux kill-window -t "$SESSION:quota-monitor" 2>/dev/null || true
    exit 0
}

trap cleanup SIGINT SIGTERM

# Démarrer
main_loop
