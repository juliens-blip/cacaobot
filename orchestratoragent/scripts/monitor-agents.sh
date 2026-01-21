#!/bin/bash
# =============================================================================
# monitor-agents.sh - Surveillance continue des agents LLM
# =============================================================================
# Ce script surveille l'etat des agents et envoie des rappels si necessaire
# Usage: ./monitor-agents.sh [--once] [--interval SECONDS]
# =============================================================================

set -e

# Configuration
PROJECT_DIR="/home/julien/Documents/palm-oil-bot"
ORCHESTRATION_DIR="$PROJECT_DIR/orchestratoragent"
STATUS_FILE="$PROJECT_DIR/ORCHESTRATION_STATUS.md"
CLAUDE_MD="$PROJECT_DIR/CLAUDE.md"
CONFIG_FILE="$ORCHESTRATION_DIR/config/task_queue.json"
LOG_FILE="$ORCHESTRATION_DIR/logs/monitor.log"
SESSION_NAME="palm-oil-orchestration"

# Parametres
INTERVAL=60  # Intervalle de verification en secondes
IDLE_THRESHOLD=300  # Seuil d'inactivite en secondes (5 min)
BLOCKED_THRESHOLD=600  # Seuil de blocage en secondes (10 min)
RUN_ONCE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --once)
            RUN_ONCE=true
            shift
            ;;
        --interval)
            INTERVAL="$2"
            shift 2
            ;;
        *)
            echo "Usage: $0 [--once] [--interval SECONDS]"
            exit 1
            ;;
    esac
done

# Creer le repertoire de logs si necessaire
mkdir -p "$(dirname "$LOG_FILE")"

# Fonction de log
log() {
    local level="$1"
    local message="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

# Verifier si tmux session existe
check_session() {
    if ! tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
        log "ERROR" "Session tmux '$SESSION_NAME' non trouvee"
        return 1
    fi
    return 0
}

# Obtenir le statut d'un agent depuis ORCHESTRATION_STATUS.md
get_agent_status() {
    local agent="$1"
    if [[ -f "$STATUS_FILE" ]]; then
        grep -i "| \*\*$agent\*\*" "$STATUS_FILE" 2>/dev/null | head -1 || echo "UNKNOWN"
    else
        echo "NO_STATUS_FILE"
    fi
}

# Verifier si un agent est actif dans tmux
check_agent_activity() {
    local window="$1"
    # Capture les dernieres lignes du pane
    local output=$(tmux capture-pane -t "$SESSION_NAME:$window" -p -S -20 2>/dev/null | tail -10)

    if [[ -z "$output" ]]; then
        echo "EMPTY"
    elif echo "$output" | grep -qE "(Waiting|Ready|IDLE|attends|Enter)"; then
        echo "IDLE"
    elif echo "$output" | grep -qE "(Error|error|FAILED|failed|panic)"; then
        echo "ERROR"
    else
        echo "ACTIVE"
    fi
}

# Envoyer un rappel a un agent
send_reminder() {
    local window="$1"
    local agent="$2"
    local message="$3"

    log "INFO" "Envoi rappel a $agent (Window $window)"

    tmux send-keys -t "$SESSION_NAME:$window" "
[RAPPEL AUTOMATIQUE DU MONITEUR]

$message

ACTIONS REQUISES:
1. Si tu travailles sur une tache, continue
2. Si tu es bloque, indique le probleme dans ORCHESTRATION_STATUS.md
3. Si tu as fini, mets a jour ton statut a IDLE et attends

TIMESTAMP: $(date '+%Y-%m-%d %H:%M:%S')
" Enter
}

# Verifier la queue des taches
check_task_queue() {
    if [[ -f "$CONFIG_FILE" ]]; then
        local pending=$(jq -r '.task_queue.pending | length' "$CONFIG_FILE" 2>/dev/null || echo "0")
        local in_progress=$(jq -r '.task_queue.in_progress | length' "$CONFIG_FILE" 2>/dev/null || echo "0")
        echo "Pending: $pending, In Progress: $in_progress"
    else
        echo "No config file"
    fi
}

# Generer un rapport de statut
generate_status_report() {
    log "INFO" "=== RAPPORT DE STATUT ==="
    log "INFO" "Queue: $(check_task_queue)"

    # Verifier chaque agent
    for agent_info in "Claude:0" "AMP:1" "Antigravity:3" "Codex:4"; do
        local agent=$(echo "$agent_info" | cut -d: -f1)
        local window=$(echo "$agent_info" | cut -d: -f2)

        local status=$(get_agent_status "$agent")
        local activity=$(check_agent_activity "$window")

        log "INFO" "$agent (Window $window): Status=$status, Activity=$activity"
    done
}

# Boucle principale de monitoring
monitor_loop() {
    log "INFO" "Demarrage du monitoring (Intervalle: ${INTERVAL}s)"

    while true; do
        if ! check_session; then
            log "ERROR" "Session tmux perdue, arret du monitoring"
            exit 1
        fi

        generate_status_report

        # Verifier les agents potentiellement bloques
        for agent_info in "AMP:1" "Antigravity:3" "Codex:4"; do
            local agent=$(echo "$agent_info" | cut -d: -f1)
            local window=$(echo "$agent_info" | cut -d: -f2)

            local activity=$(check_agent_activity "$window")

            if [[ "$activity" == "IDLE" ]]; then
                log "WARN" "$agent semble inactif, envoi d'un rappel"
                send_reminder "$window" "$agent" "Tu sembles inactif. As-tu une tache en cours?"
            elif [[ "$activity" == "ERROR" ]]; then
                log "ERROR" "$agent a possiblement une erreur"
                send_reminder "$window" "$agent" "Une erreur a ete detectee. Verifie ton travail et indique le probleme."
            fi
        done

        if $RUN_ONCE; then
            log "INFO" "Mode --once, arret apres une iteration"
            break
        fi

        log "INFO" "Prochaine verification dans ${INTERVAL}s"
        sleep "$INTERVAL"
    done
}

# Point d'entree
main() {
    log "INFO" "=========================================="
    log "INFO" "Demarrage Monitor Agents - Palm Oil Bot"
    log "INFO" "=========================================="

    if ! command -v jq &> /dev/null; then
        log "WARN" "jq n'est pas installe, certaines fonctionnalites seront limitees"
    fi

    if ! command -v tmux &> /dev/null; then
        log "ERROR" "tmux n'est pas installe"
        exit 1
    fi

    monitor_loop
}

main "$@"
