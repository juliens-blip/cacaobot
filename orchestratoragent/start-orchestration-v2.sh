#!/bin/bash
# =============================================================================
# start-orchestration-v2.sh - Lance le systeme d'orchestration multi-LLM v2
# =============================================================================
# Ce script:
# 1. Cree une session tmux avec 5 fenetres (Claude, AMP, Proxy, Antigravity, Codex)
# 2. Lance chaque LLM avec son prompt initial
# 3. Initialise la queue des taches
# 4. Lance le monitoring en arriere-plan
# =============================================================================

set -e

# Configuration
PROJECT_DIR="/home/julien/Documents/palm-oil-bot"
ORCHESTRATION_DIR="$PROJECT_DIR/orchestratoragent"
SESSION_NAME="palm-oil-orchestration"
LOG_DIR="$ORCHESTRATION_DIR/logs"

# Couleurs pour output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Creer les repertoires necessaires
mkdir -p "$LOG_DIR" "$ORCHESTRATION_DIR/handoff" "$ORCHESTRATION_DIR/tasks"

# Fonction de log
log() {
    local level="$1"
    local message="$2"
    local color=""
    case $level in
        "INFO") color="$GREEN" ;;
        "WARN") color="$YELLOW" ;;
        "ERROR") color="$RED" ;;
        *) color="$BLUE" ;;
    esac
    echo -e "${color}[$level]${NC} $message"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [$level] $message" >> "$LOG_DIR/orchestration.log"
}

# Verifier les prerequis
check_prerequisites() {
    log "INFO" "Verification des prerequis..."

    # tmux
    if ! command -v tmux &> /dev/null; then
        log "ERROR" "tmux n'est pas installe. Installez-le avec: sudo apt install tmux"
        exit 1
    fi

    # claude
    if ! command -v claude &> /dev/null; then
        log "WARN" "claude CLI non trouve. Certains agents ne fonctionneront pas."
    fi

    # amp
    if ! command -v amp &> /dev/null; then
        log "WARN" "amp non trouve. L'agent AMP ne fonctionnera pas."
    fi

    # codex
    if ! command -v codex &> /dev/null; then
        log "WARN" "codex non trouve. L'agent Codex ne fonctionnera pas."
    fi

    log "INFO" "Prerequis verifies."
}

# Tuer une session existante si elle existe
kill_existing_session() {
    if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
        log "WARN" "Session existante trouvee. Fermeture..."
        tmux kill-session -t "$SESSION_NAME"
        sleep 1
    fi
}

# Creer la session tmux
create_tmux_session() {
    log "INFO" "Creation de la session tmux '$SESSION_NAME'..."

    # Creer la session avec la premiere fenetre (Claude)
    tmux new-session -d -s "$SESSION_NAME" -n "Claude" -c "$PROJECT_DIR"

    # Creer les autres fenetres
    tmux new-window -t "$SESSION_NAME" -n "AMP" -c "$PROJECT_DIR"
    tmux new-window -t "$SESSION_NAME" -n "Proxy" -c "$PROJECT_DIR"
    tmux new-window -t "$SESSION_NAME" -n "Antigravity" -c "$PROJECT_DIR"
    tmux new-window -t "$SESSION_NAME" -n "Codex" -c "$PROJECT_DIR"

    log "INFO" "Session tmux creee avec 5 fenetres."
}

# Lancer Claude (Orchestrateur)
launch_claude() {
    log "INFO" "Lancement de Claude (Orchestrateur)..."

    tmux send-keys -t "$SESSION_NAME:0" "cd $PROJECT_DIR && clear" Enter
    sleep 0.5

    # Afficher le prompt initial
    tmux send-keys -t "$SESSION_NAME:0" "echo '=== CLAUDE ORCHESTRATOR v2 ===' && echo 'Projet: Palm Oil Trading Bot' && echo 'Role: Orchestrateur Principal' && echo ''" Enter
    sleep 0.5

    # Lancer Claude avec le prompt orchestrateur (mode autonome)
    tmux send-keys -t "$SESSION_NAME:0" "claude --dangerously-skip-permissions" Enter
    sleep 2

    # Envoyer le prompt initial
    local prompt=$(cat "$ORCHESTRATION_DIR/prompts/claude-orchestrator-v2.md")
    tmux send-keys -t "$SESSION_NAME:0" "$prompt" Enter

    log "INFO" "Claude lance avec prompt orchestrateur v2."
}

# Lancer AMP (Worker)
launch_amp() {
    log "INFO" "Lancement de AMP (Worker)..."

    tmux send-keys -t "$SESSION_NAME:1" "cd $PROJECT_DIR && clear" Enter
    sleep 0.5

    tmux send-keys -t "$SESSION_NAME:1" "echo '=== AMP WORKER v2 ===' && echo 'Projet: Palm Oil Trading Bot' && echo 'Role: Implementation Specialist' && echo ''" Enter
    sleep 0.5

    # Lancer AMP
    if command -v amp &> /dev/null; then
        tmux send-keys -t "$SESSION_NAME:1" "amp -m large --dangerously-allow-all" Enter
        sleep 2

        # Envoyer le prompt initial
        local prompt=$(cat "$ORCHESTRATION_DIR/prompts/amp-worker-v2.md")
        tmux send-keys -t "$SESSION_NAME:1" "$prompt" Enter

        log "INFO" "AMP lance avec prompt worker v2."
    else
        tmux send-keys -t "$SESSION_NAME:1" "echo 'AMP non installe. En attente...'" Enter
        log "WARN" "AMP non disponible."
    fi
}

# Lancer le proxy Antigravity
launch_proxy() {
    log "INFO" "Lancement du proxy Antigravity..."

    tmux send-keys -t "$SESSION_NAME:2" "cd $PROJECT_DIR && clear" Enter
    sleep 0.5

    tmux send-keys -t "$SESSION_NAME:2" "echo '=== ANTIGRAVITY PROXY ===' && echo 'En attente de demarrage...' && echo ''" Enter

    # Le proxy est lance separement si necessaire
    log "INFO" "Fenetre proxy prete."
}

# Lancer Antigravity (Worker)
launch_antigravity() {
    log "INFO" "Lancement de Antigravity (Worker)..."

    tmux send-keys -t "$SESSION_NAME:3" "cd $PROJECT_DIR && clear" Enter
    sleep 0.5

    tmux send-keys -t "$SESSION_NAME:3" "echo '=== ANTIGRAVITY WORKER v2 ===' && echo 'Projet: Palm Oil Trading Bot' && echo 'Role: Deep Analysis Expert' && echo ''" Enter
    sleep 0.5

    # Lancer Claude avec extended thinking pour Antigravity (mode autonome)
    if command -v claude &> /dev/null; then
        tmux send-keys -t "$SESSION_NAME:3" "claude --dangerously-skip-permissions" Enter
        sleep 2

        # Envoyer le prompt initial
        local prompt=$(cat "$ORCHESTRATION_DIR/prompts/antigravity-worker-v2.md")
        tmux send-keys -t "$SESSION_NAME:3" "$prompt" Enter

        log "INFO" "Antigravity lance avec prompt worker v2."
    else
        tmux send-keys -t "$SESSION_NAME:3" "echo 'Claude non installe. En attente...'" Enter
        log "WARN" "Claude non disponible pour Antigravity."
    fi
}

# Lancer Codex (Worker)
launch_codex() {
    log "INFO" "Lancement de Codex (Worker)..."

    tmux send-keys -t "$SESSION_NAME:4" "cd $PROJECT_DIR && clear" Enter
    sleep 0.5

    tmux send-keys -t "$SESSION_NAME:4" "echo '=== CODEX WORKER v2 ===' && echo 'Projet: Palm Oil Trading Bot' && echo 'Role: Code Generation Expert' && echo ''" Enter
    sleep 0.5

    # Lancer Codex
    if command -v codex &> /dev/null; then
        tmux send-keys -t "$SESSION_NAME:4" "codex --dangerously-bypass-approvals-and-sandbox" Enter
        sleep 2

        # Envoyer le prompt initial
        local prompt=$(cat "$ORCHESTRATION_DIR/prompts/codex-worker-v2.md")
        tmux send-keys -t "$SESSION_NAME:4" "$prompt" Enter

        log "INFO" "Codex lance avec prompt worker v2."
    else
        tmux send-keys -t "$SESSION_NAME:4" "echo 'Codex non installe. En attente...'" Enter
        log "WARN" "Codex non disponible."
    fi
}

# Initialiser le fichier de statut
init_status_file() {
    log "INFO" "Initialisation du fichier de statut..."

    cat > "$PROJECT_DIR/ORCHESTRATION_STATUS.md" << 'STATUS_EOF'
# Multi-LLM Orchestration Status v2

**Orchestrateur**: Claude
**Session tmux**: `palm-oil-orchestration`
**Date de demarrage**: TIMESTAMP_PLACEHOLDER

---

## Agent Status

| Agent | Fenetre tmux | Tache | Status | Last Update |
|-------|--------------|-------|--------|-------------|
| **Claude** | 0-Claude | Orchestration | STARTING | TIMESTAMP_PLACEHOLDER |
| **AMP** | 1-AMP | En attente | STARTING | TIMESTAMP_PLACEHOLDER |
| **Proxy** | 2-Proxy | Communication | STANDBY | TIMESTAMP_PLACEHOLDER |
| **Antigravity** | 3-Antigravity | En attente | STARTING | TIMESTAMP_PLACEHOLDER |
| **Codex** | 4-Codex | En attente | STARTING | TIMESTAMP_PLACEHOLDER |

---

## Queue des Taches

### En Attente (Pending)
_Aucune tache en attente_

### En Cours (In Progress)
_Aucune tache en cours_

### Completees (Completed)
_Aucune tache completee_

---

## Log des Actions

| Heure | Agent | Action | Status |
|-------|-------|--------|--------|
| TIMESTAMP_PLACEHOLDER | System | Demarrage orchestration v2 | OK |

---

**Last Update**: TIMESTAMP_PLACEHOLDER
STATUS_EOF

    # Remplacer les placeholders par la date actuelle
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    sed -i "s/TIMESTAMP_PLACEHOLDER/$timestamp/g" "$PROJECT_DIR/ORCHESTRATION_STATUS.md"

    log "INFO" "Fichier de statut initialise."
}

# Afficher les instructions
show_instructions() {
    echo ""
    echo "=============================================="
    echo "   ORCHESTRATION MULTI-LLM v2 DEMARREE"
    echo "=============================================="
    echo ""
    echo "Session tmux: $SESSION_NAME"
    echo ""
    echo "Pour attacher la session:"
    echo "  tmux attach -t $SESSION_NAME"
    echo ""
    echo "Navigation tmux:"
    echo "  Ctrl+b puis 0-4  : Changer de fenetre"
    echo "  Ctrl+b puis d    : Detacher la session"
    echo "  Ctrl+b puis z    : Zoom sur fenetre"
    echo ""
    echo "Fenetres:"
    echo "  0: Claude (Orchestrateur)"
    echo "  1: AMP (Worker)"
    echo "  2: Proxy"
    echo "  3: Antigravity (Worker)"
    echo "  4: Codex (Worker)"
    echo ""
    echo "Pour arreter:"
    echo "  ./stop-orchestration.sh"
    echo "  ou: tmux kill-session -t $SESSION_NAME"
    echo ""
    echo "=============================================="
}

# Point d'entree principal
main() {
    log "INFO" "=========================================="
    log "INFO" "Demarrage Orchestration Multi-LLM v2"
    log "INFO" "=========================================="

    check_prerequisites
    kill_existing_session
    create_tmux_session
    init_status_file

    # Lancer les agents avec un delai entre chaque
    launch_claude
    sleep 3

    launch_amp
    sleep 2

    launch_proxy
    sleep 1

    launch_antigravity
    sleep 2

    launch_codex
    sleep 2

    show_instructions

    log "INFO" "Orchestration demarree avec succes."
    log "INFO" "Attachez la session avec: tmux attach -t $SESSION_NAME"
}

main "$@"
