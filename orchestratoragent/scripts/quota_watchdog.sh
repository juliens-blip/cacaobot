#!/bin/bash
# ============================================================================
# quota_watchdog.sh - Watchdog autonome pour quota Claude
# ============================================================================
# Surveille le quota en parsant le footer et alerte à 93%
# Lance en background: nohup ./quota_watchdog.sh &
# ============================================================================

SESSION="${1:-orchestration-palm-oil-bot}"
WINDOW="${2:-claude}"
ALERT_THRESHOLD="${3:-93}"
CHECK_INTERVAL="${4:-30}"  # secondes

LOG_FILE="/tmp/claude_quota_watchdog.log"
QUOTA_FILE="/tmp/claude_current_quota"
ALERT_TRIGGERED="/tmp/claude_quota_alert_triggered"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

get_quota() {
    local content=$(tmux capture-pane -t "$SESSION:$WINDOW" -p 2>/dev/null)
    echo "$content" | grep -oE "used [0-9]+%" | grep -oE "[0-9]+" | tail -1
}

send_alert() {
    local quota=$1

    # Éviter les alertes répétées
    if [[ -f "$ALERT_TRIGGERED" ]]; then
        local last_alert=$(cat "$ALERT_TRIGGERED")
        if [[ "$last_alert" == "$quota" ]]; then
            return
        fi
    fi

    echo "$quota" > "$ALERT_TRIGGERED"

    log "🚨 ALERTE: Quota à ${quota}% - Seuil ${ALERT_THRESHOLD}% dépassé!"

    # Notification desktop
    notify-send -u critical "🚨 Claude Quota Alert" \
        "Session à ${quota}%!\nHandoff vers AMP recommandé." 2>/dev/null || true

    # Son d'alerte
    paplay /usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga 2>/dev/null &

    # Notifier AMP dans tmux
    tmux send-keys -t "$SESSION:amp" \
        "⚠️ ALERTE QUOTA: La session Claude est à ${quota}%. Prépare-toi à prendre le relais de l'orchestration." Enter 2>/dev/null || true

    # Écrire dans un fichier que Claude peut lire
    cat > /tmp/HANDOFF_ALERT.md << EOF
# 🚨 ALERTE QUOTA CLAUDE

**Timestamp**: $(date '+%Y-%m-%d %H:%M:%S')
**Quota actuel**: ${quota}%
**Seuil d'alerte**: ${ALERT_THRESHOLD}%

## Action requise

Le quota de session Claude approche de sa limite.
Un handoff vers AMP est recommandé.

## Instructions pour AMP

1. Lire le fichier HANDOFF_TO_AMP.md si disponible
2. Reprendre le contexte de l'orchestration
3. Continuer les tâches en cours
EOF

    log "Fichier d'alerte créé: /tmp/HANDOFF_ALERT.md"
}

display_bar() {
    local quota=$1
    local width=30
    local filled=$((quota * width / 100))
    local empty=$((width - filled))
    printf "%${filled}s" | tr ' ' '█'
    printf "%${empty}s" | tr ' ' '░'
}

# Démarrage
log "═══════════════════════════════════════════════════"
log "QUOTA WATCHDOG DÉMARRÉ"
log "Session: $SESSION:$WINDOW"
log "Seuil d'alerte: ${ALERT_THRESHOLD}%"
log "Intervalle: ${CHECK_INTERVAL}s"
log "═══════════════════════════════════════════════════"

# Boucle principale
while true; do
    QUOTA=$(get_quota)

    if [[ -n "$QUOTA" ]] && [[ "$QUOTA" =~ ^[0-9]+$ ]]; then
        # Sauvegarder le quota
        echo "$QUOTA" > "$QUOTA_FILE"

        # Afficher le status
        BAR=$(display_bar "$QUOTA")
        if [[ "$QUOTA" -ge "$ALERT_THRESHOLD" ]]; then
            log "🚨 ${QUOTA}% [${BAR}] ALERTE!"
            send_alert "$QUOTA"
        elif [[ "$QUOTA" -ge 75 ]]; then
            log "⚠️ ${QUOTA}% [${BAR}]"
        else
            log "✅ ${QUOTA}% [${BAR}]"
        fi
    else
        log "⚠️ Quota non lisible"
    fi

    sleep "$CHECK_INTERVAL"
done
