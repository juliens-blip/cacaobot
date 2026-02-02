#!/bin/bash
# ============================================================================
# check_claude_quota.sh - Vérifie le quota de session Claude
# ============================================================================
# Parse le footer de la session Claude pour extraire le quota
# Usage: ./check_claude_quota.sh [session] [window]
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

# Capturer l'écran et parser le quota
get_quota() {
    local content=$(tmux capture-pane -t "$SESSION:$WINDOW" -p 2>/dev/null)

    # Pattern: "You've used XX% of your session limit"
    # ou: "used XX% of"
    local quota=$(echo "$content" | grep -oE "used [0-9]+%" | grep -oE "[0-9]+" | tail -1)

    echo "$quota"
}

# Récupérer le quota
QUOTA=$(get_quota)

if [[ -z "$QUOTA" ]] || ! [[ "$QUOTA" =~ ^[0-9]+$ ]]; then
    echo -e "${RED}❌ Impossible de lire le quota${NC}"
    echo "Vérifiez que la session $SESSION:$WINDOW existe"
    exit 1
fi

# Barre de progression
BAR_WIDTH=40
FILLED=$((QUOTA * BAR_WIDTH / 100))
EMPTY=$((BAR_WIDTH - FILLED))
BAR=$(printf "%${FILLED}s" | tr ' ' '█')$(printf "%${EMPTY}s" | tr ' ' '░')

# Couleur selon le niveau
if [[ "$QUOTA" -lt 50 ]]; then
    COLOR=$GREEN
    STATUS="✅ OK"
elif [[ "$QUOTA" -lt 75 ]]; then
    COLOR=$YELLOW
    STATUS="⚡ Attention"
elif [[ "$QUOTA" -lt "$ALERT_THRESHOLD" ]]; then
    COLOR=$YELLOW
    STATUS="⚠️ Élevé"
else
    COLOR=$RED
    STATUS="🚨 ALERTE - Handoff recommandé"
fi

# Affichage
echo -e "${CYAN}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}      📊 CLAUDE SESSION QUOTA                      ${CYAN}║${NC}"
echo -e "${CYAN}╠═══════════════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║${NC} Session: ${YELLOW}$SESSION:$WINDOW${NC}"
echo -e "${CYAN}║${NC} Quota:   ${COLOR}${QUOTA}%${NC} of session limit"
echo -e "${CYAN}║${NC} [${COLOR}${BAR}${NC}]"
echo -e "${CYAN}║${NC} Status:  ${COLOR}${STATUS}${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════╝${NC}"

# Export pour utilisation programmatique
echo ""
echo "QUOTA=$QUOTA"
echo "ALERT=$([[ $QUOTA -ge $ALERT_THRESHOLD ]] && echo "true" || echo "false")"

# Code de sortie basé sur l'alerte
[[ "$QUOTA" -ge "$ALERT_THRESHOLD" ]] && exit 2 || exit 0
