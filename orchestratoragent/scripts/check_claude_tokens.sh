#!/bin/bash
# ============================================================================
# check_claude_tokens.sh - Vérifie les tokens utilisés dans une session Claude
# ============================================================================
# Usage: ./check_claude_tokens.sh [session_id]
#        ./check_claude_tokens.sh              # Session la plus récente
#        ./check_claude_tokens.sh abc123       # Session spécifique
# ============================================================================

set -e

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
CLAUDE_DIR="$HOME/.claude"
PROJECT_DIR="$CLAUDE_DIR/projects/-home-julien-Documents-palm-oil-bot"
MAX_CONTEXT_TOKENS=200000  # Claude Opus 4.5 context window

# Fonction: Afficher l'aide
show_help() {
    echo "Usage: $0 [session_id]"
    echo ""
    echo "Options:"
    echo "  -h, --help     Afficher cette aide"
    echo "  -l, --list     Lister toutes les sessions"
    echo "  -a, --all      Afficher les stats de toutes les sessions"
    echo ""
    echo "Sans argument: affiche la session la plus récente"
}

# Fonction: Lister les sessions
list_sessions() {
    echo -e "${CYAN}Sessions disponibles:${NC}"
    echo "----------------------------------------"
    ls -lt "$PROJECT_DIR"/*.jsonl 2>/dev/null | head -10 | while read line; do
        file=$(echo "$line" | awk '{print $NF}')
        session_id=$(basename "$file" .jsonl)
        size=$(echo "$line" | awk '{print $5}')
        date=$(echo "$line" | awk '{print $6, $7, $8}')
        echo -e "${BLUE}$session_id${NC} - $size bytes - $date"
    done
}

# Fonction: Extraire les stats d'une session
get_session_stats() {
    local session_file="$1"
    local session_id=$(basename "$session_file" .jsonl)

    if [[ ! -f "$session_file" ]]; then
        echo -e "${RED}Erreur: Session non trouvée${NC}"
        return 1
    fi

    # Extraire les dernières infos de tokens
    local last_usage=$(tail -20 "$session_file" | grep -o '"usage":{[^}]*}' | tail -1)

    # Parser avec jq si disponible, sinon avec grep/sed
    if command -v jq &> /dev/null; then
        local stats=$(tail -1 "$session_file" | jq -r '
            .message.usage // empty |
            "input_tokens:\(.input_tokens // 0)\n" +
            "output_tokens:\(.output_tokens // 0)\n" +
            "cache_read:\(.cache_read_input_tokens // 0)\n" +
            "cache_creation:\(.cache_creation_input_tokens // 0)"
        ' 2>/dev/null)

        if [[ -n "$stats" ]]; then
            local input_tokens=$(echo "$stats" | grep "input_tokens:" | cut -d: -f2)
            local output_tokens=$(echo "$stats" | grep "output_tokens:" | cut -d: -f2)
            local cache_read=$(echo "$stats" | grep "cache_read:" | cut -d: -f2)
            local cache_creation=$(echo "$stats" | grep "cache_creation:" | cut -d: -f2)
        fi
    else
        # Fallback sans jq
        local input_tokens=$(echo "$last_usage" | grep -o '"input_tokens":[0-9]*' | grep -o '[0-9]*' | tail -1)
        local output_tokens=$(echo "$last_usage" | grep -o '"output_tokens":[0-9]*' | grep -o '[0-9]*' | tail -1)
        local cache_read=$(echo "$last_usage" | grep -o '"cache_read_input_tokens":[0-9]*' | grep -o '[0-9]*' | tail -1)
        local cache_creation=$(echo "$last_usage" | grep -o '"cache_creation_input_tokens":[0-9]*' | grep -o '[0-9]*' | tail -1)
    fi

    # Valeurs par défaut
    input_tokens=${input_tokens:-0}
    output_tokens=${output_tokens:-0}
    cache_read=${cache_read:-0}
    cache_creation=${cache_creation:-0}

    # Calculer le total de contexte utilisé
    local total_context=$((cache_read + cache_creation + input_tokens))
    local remaining=$((MAX_CONTEXT_TOKENS - total_context))
    local percentage_used=$((total_context * 100 / MAX_CONTEXT_TOKENS))
    local percentage_remaining=$((100 - percentage_used))

    # Barre de progression
    local bar_width=40
    local filled=$((percentage_used * bar_width / 100))
    local empty=$((bar_width - filled))
    local bar=$(printf "%${filled}s" | tr ' ' '█')$(printf "%${empty}s" | tr ' ' '░')

    # Affichage
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}        ${YELLOW}📊 CLAUDE SESSION TOKEN USAGE${NC}                        ${CYAN}║${NC}"
    echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${CYAN}║${NC} Session ID: ${BLUE}${session_id:0:36}${NC}    ${CYAN}║${NC}"
    echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${CYAN}║${NC} ${GREEN}Context Window:${NC} $MAX_CONTEXT_TOKENS tokens (200k)              ${CYAN}║${NC}"
    echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${CYAN}║${NC} Input tokens:          $(printf "%'10d" $input_tokens)                       ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC} Output tokens:         $(printf "%'10d" $output_tokens)                       ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC} Cache read:            $(printf "%'10d" $cache_read)                       ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC} Cache creation:        $(printf "%'10d" $cache_creation)                       ${CYAN}║${NC}"
    echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${CYAN}║${NC} ${YELLOW}TOTAL CONTEXT USED:${NC}    $(printf "%'10d" $total_context) tokens              ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC} ${GREEN}REMAINING:${NC}             $(printf "%'10d" $remaining) tokens              ${CYAN}║${NC}"
    echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"

    # Couleur selon le niveau d'utilisation
    if [[ $percentage_used -lt 50 ]]; then
        bar_color=$GREEN
    elif [[ $percentage_used -lt 75 ]]; then
        bar_color=$YELLOW
    else
        bar_color=$RED
    fi

    echo -e "${CYAN}║${NC} [${bar_color}${bar}${NC}] ${percentage_used}% used        ${CYAN}║${NC}"
    echo -e "${CYAN}╠══════════════════════════════════════════════════════════════╣${NC}"

    # Recommandation
    if [[ $percentage_used -lt 50 ]]; then
        echo -e "${CYAN}║${NC} ${GREEN}✓ Session saine - Contexte suffisant${NC}                        ${CYAN}║${NC}"
    elif [[ $percentage_used -lt 75 ]]; then
        echo -e "${CYAN}║${NC} ${YELLOW}⚠ Attention - Contexte à surveiller${NC}                         ${CYAN}║${NC}"
    elif [[ $percentage_used -lt 90 ]]; then
        echo -e "${CYAN}║${NC} ${RED}⚠ Alerte - Envisager un handoff${NC}                             ${CYAN}║${NC}"
    else
        echo -e "${CYAN}║${NC} ${RED}🚨 CRITIQUE - Handoff recommandé immédiatement${NC}               ${CYAN}║${NC}"
    fi

    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Export pour utilisation programmatique
    echo "TOKENS_USED=$total_context"
    echo "TOKENS_REMAINING=$remaining"
    echo "PERCENTAGE_USED=$percentage_used"
}

# Fonction: Trouver la session la plus récente
get_latest_session() {
    ls -t "$PROJECT_DIR"/*.jsonl 2>/dev/null | head -1
}

# Fonction: Trouver une session active dans tmux
get_active_session_from_tmux() {
    local window_name="${1:-claude}"
    local session_name="${2:-orchestration-palm-oil-bot}"

    # Capturer la sortie de la fenêtre Claude pour trouver le session ID
    local pane_content=$(tmux capture-pane -t "$session_name:$window_name" -p 2>/dev/null)

    # Chercher un pattern de session ID dans la sortie
    local session_id=$(echo "$pane_content" | grep -oE '[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}' | tail -1)

    if [[ -n "$session_id" ]] && [[ -f "$PROJECT_DIR/$session_id.jsonl" ]]; then
        echo "$PROJECT_DIR/$session_id.jsonl"
    else
        get_latest_session
    fi
}

# Main
case "${1:-}" in
    -h|--help)
        show_help
        ;;
    -l|--list)
        list_sessions
        ;;
    -a|--all)
        for session in "$PROJECT_DIR"/*.jsonl; do
            get_session_stats "$session"
            echo ""
        done
        ;;
    "")
        # Session la plus récente
        latest=$(get_latest_session)
        if [[ -n "$latest" ]]; then
            get_session_stats "$latest"
        else
            echo -e "${RED}Aucune session trouvée${NC}"
            exit 1
        fi
        ;;
    *)
        # Session spécifique
        if [[ -f "$PROJECT_DIR/$1.jsonl" ]]; then
            get_session_stats "$PROJECT_DIR/$1.jsonl"
        elif [[ -f "$1" ]]; then
            get_session_stats "$1"
        else
            echo -e "${RED}Session non trouvée: $1${NC}"
            list_sessions
            exit 1
        fi
        ;;
esac
