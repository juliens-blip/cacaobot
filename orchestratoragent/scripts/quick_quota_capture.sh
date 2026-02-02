#!/bin/bash
# ============================================================================
# quick_quota_capture.sh - Capture rapide du quota après /config
# ============================================================================
# Usage: Lance ce script PUIS tape /config dans Claude
#        Le script capture automatiquement le quota affiché
# ============================================================================

SESSION="orchestration-palm-oil-bot"
WINDOW="claude"
ALERT_THRESHOLD=${1:-93}

echo "⏳ En attente du quota... (tape /config dans Claude)"
echo "   Seuil d'alerte: ${ALERT_THRESHOLD}%"
echo ""

# Boucle de capture pendant 30 secondes
for i in {1..30}; do
    content=$(tmux capture-pane -t "$SESSION:$WINDOW" -p 2>/dev/null)

    # Chercher "Current session" suivi d'un pourcentage
    quota=$(echo "$content" | grep -A2 "Current session" | grep -oE "[0-9]+%" | head -1 | tr -d '%')

    if [[ -n "$quota" ]] && [[ "$quota" =~ ^[0-9]+$ ]]; then
        echo "╔═══════════════════════════════════════════════╗"
        echo "║  📊 QUOTA CLAUDE CAPTURÉ                      ║"
        echo "╠═══════════════════════════════════════════════╣"
        echo "║  Current session: ${quota}%                      ║"

        if [[ "$quota" -ge "$ALERT_THRESHOLD" ]]; then
            echo "║  ⚠️  ALERTE: Seuil ${ALERT_THRESHOLD}% dépassé!             ║"
            echo "║  🔄 HANDOFF RECOMMANDÉ                        ║"
        elif [[ "$quota" -ge 75 ]]; then
            echo "║  ⚡ Attention: Quota élevé                    ║"
        else
            echo "║  ✅ Quota OK                                  ║"
        fi

        echo "╚═══════════════════════════════════════════════╝"

        # Sauvegarder pour référence
        echo "$(date '+%Y-%m-%d %H:%M:%S') - Session quota: ${quota}%" >> /tmp/claude_quota.log

        # Exporter pour utilisation programmatique
        echo ""
        echo "QUOTA=$quota"
        exit 0
    fi

    sleep 1
done

echo "❌ Timeout: Quota non détecté. As-tu tapé /config ?"
exit 1
