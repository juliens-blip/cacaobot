#!/bin/bash
# =============================================================================
# dispatch-tasks.sh - Dispatch automatique des taches aux agents LLM
# =============================================================================
# Ce script lit la queue des taches et les envoie aux agents disponibles
# Usage: ./dispatch-tasks.sh [--agent AGENT] [--task-id TASK_ID]
# =============================================================================

set -e

# Configuration
PROJECT_DIR="/home/julien/Documents/palm-oil-bot"
ORCHESTRATION_DIR="$PROJECT_DIR/orchestratoragent"
CONFIG_FILE="$ORCHESTRATION_DIR/config/task_queue.json"
TASKS_DIR="$ORCHESTRATION_DIR/tasks"
LOG_FILE="$ORCHESTRATION_DIR/logs/dispatch.log"
SESSION_NAME="palm-oil-orchestration"

# Creer les repertoires necessaires
mkdir -p "$TASKS_DIR" "$(dirname "$LOG_FILE")"

# Fonction de log
log() {
    local level="$1"
    local message="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

# Mapping agent -> window
get_window() {
    local agent="$1"
    case "$agent" in
        claude) echo 0 ;;
        amp) echo 1 ;;
        antigravity) echo 3 ;;
        codex) echo 4 ;;
        *) echo -1 ;;
    esac
}

# Envoyer une tache a un agent
dispatch_to_agent() {
    local agent="$1"
    local task_file="$2"

    local window=$(get_window "$agent")
    if [[ "$window" == "-1" ]]; then
        log "ERROR" "Agent inconnu: $agent"
        return 1
    fi

    if [[ ! -f "$task_file" ]]; then
        log "ERROR" "Fichier de tache non trouve: $task_file"
        return 1
    fi

    local task_content=$(cat "$task_file")

    log "INFO" "Dispatch tache vers $agent (Window $window)"

    tmux send-keys -t "$SESSION_NAME:$window" "$task_content" Enter

    log "INFO" "Tache envoyee avec succes a $agent"
    return 0
}

# Creer un fichier de tache a partir d'un template
create_task_file() {
    local task_id="$1"
    local agent="$2"
    local title="$3"
    local description="$4"
    local files="$5"
    local instructions="$6"

    local task_file="$TASKS_DIR/${task_id}_${agent}.txt"

    cat > "$task_file" << TASK_EOF
[TACHE ORCHESTRATEUR - AUTONOMIE MAXIMALE]

AGENT: $(echo "$agent" | tr '[:lower:]' '[:upper:]')
TASK_ID: $task_id
PRIORITE: HAUTE
TIMESTAMP: $(date '+%Y-%m-%d %H:%M:%S')

=== CONTEXTE ===
Projet: Palm Oil Trading Bot (Rust)
Tu travailles sur le projet situe dans: $PROJECT_DIR
Memoire partagee: $PROJECT_DIR/CLAUDE.md
Statut orchestration: $PROJECT_DIR/ORCHESTRATION_STATUS.md

=== OBJECTIF ===
$title

$description

=== FICHIERS A CREER/MODIFIER ===
$files

=== INSTRUCTIONS DETAILLEES ===
$instructions

=== CRITERES DE VALIDATION ===
- [ ] Code compile sans erreur (cargo build)
- [ ] Tests passent (cargo test)
- [ ] Documentation mise a jour si necessaire
- [ ] CLAUDE.md mis a jour avec tes actions
- [ ] ORCHESTRATION_STATUS.md mis a jour avec ton statut

=== APRES COMPLETION ===
1. Mets a jour ORCHESTRATION_STATUS.md:
   - Change ton statut de WORKING a IDLE
   - Ajoute la duree reelle de la tache
   - Liste les fichiers modifies

2. Mets a jour CLAUDE.md section 'Log des Actions LLM':
   | [HEURE] | $(echo "$agent" | tr '[:lower:]' '[:upper:]') | $task_id | [DUREE] | COMPLETED |

3. Si tu as fini et qu'il reste des taches dans la queue:
   Lis $CONFIG_FILE pour voir les taches pending

4. Attends la prochaine instruction de l'orchestrateur

=== IMPORTANT ===
- NE POSE PAS DE QUESTIONS - Agis avec autonomie
- Si tu es bloque, documente le probleme et passe a autre chose
- Utilise les outils a ta disposition (Read, Write, Edit, Bash, Grep, Glob)

COMMENCE IMMEDIATEMENT
TASK_EOF

    echo "$task_file"
}

# Exemple d'utilisation avec des taches predefinies
dispatch_predefined_tasks() {
    log "INFO" "Dispatch des taches predefinies pour Palm Oil Bot"

    # Tache pour AMP: Tests d'integration complets
    local amp_task=$(create_task_file \
        "TASK-PO-016" \
        "amp" \
        "Tests d'integration complets pour le trading module" \
        "Creer une suite complete de tests d'integration pour verifier que tous les modules trading fonctionnent ensemble correctement. Les tests doivent couvrir le flux complet: connexion cTrader -> reception prix -> calcul RSI -> decision trading -> execution ordre." \
        "1. tests/trading_integration_test.rs
   - Tests de connexion mock cTrader
   - Tests de flux de prix
   - Tests de signaux trading
   - Tests d'execution ordres

2. src/modules/trading/mod.rs
   - Ajouter les exports necessaires pour les tests

3. Cargo.toml
   - Ajouter les dependances de test si necessaires" \
        "1. Lire les fichiers existants dans src/modules/trading/
2. Comprendre l'architecture du module trading
3. Creer les mocks necessaires pour cTrader
4. Ecrire les tests d'integration couvrant:
   - Connexion et authentification
   - Souscription aux prix
   - Calcul RSI sur flux de prix
   - Generation de signaux BUY/SELL
   - Gestion des positions
5. Verifier que tous les tests passent
6. Ajouter des tests de regression si bugs trouves")

    dispatch_to_agent "amp" "$amp_task"

    # Tache pour Antigravity: Analyse de performance
    local antigravity_task=$(create_task_file \
        "TASK-PO-017" \
        "antigravity" \
        "Analyse de performance et optimisation du bot" \
        "Effectuer une analyse approfondie des performances du bot de trading. Identifier les goulots d'etranglement potentiels, les problemes de latence, et proposer des optimisations concretes pour atteindre l'objectif de 2-3% de rentabilite journaliere." \
        "1. docs/performance_analysis.md
   - Rapport complet d'analyse
   - Metriques cles identifiees
   - Recommandations d'optimisation

2. docs/latency_optimization.md
   - Analyse de la latence par composant
   - Strategies de reduction de latence" \
        "1. Lire tous les fichiers du module trading
2. Analyser le flux de donnees de bout en bout
3. Identifier les operations potentiellement lentes:
   - Appels API Perplexity (sentiment)
   - Connexion cTrader WebSocket
   - Calculs RSI/EMA
   - Decisions de trading
4. Proposer des optimisations:
   - Caching des donnees sentiment
   - Pooling de connexions
   - Calculs incrementaux
   - Parallelisation
5. Estimer l'impact de chaque optimisation
6. Creer un plan d'implementation priorise
7. Documenter les risques et mitigations")

    dispatch_to_agent "antigravity" "$antigravity_task"

    # Tache pour Codex: Types et validation
    local codex_task=$(create_task_file \
        "TASK-PO-018" \
        "codex" \
        "Amelioration des types et validation dans le bot" \
        "Renforcer le typage Rust et ajouter des validations pour rendre le bot plus robuste. Ajouter des types personnalises pour les donnees de marche, les ordres, et les metriques. Implementer des validations strictes pour prevenir les erreurs de trading." \
        "1. src/modules/trading/types.rs (nouveau)
   - Types pour MarketData, OrderRequest, TradeResult
   - Enums pour OrderSide, OrderType, PositionStatus
   - Validation des prix et volumes

2. src/modules/trading/validation.rs (nouveau)
   - Fonctions de validation pour ordres
   - Verification des limites (max position, max loss)
   - Validation des donnees de marche

3. src/modules/trading/mod.rs
   - Ajouter les exports des nouveaux modules" \
        "1. Lire les fichiers existants pour comprendre les structures actuelles
2. Identifier tous les types utilises pour le trading
3. Creer types.rs avec:
   - MarketData { symbol, bid, ask, timestamp }
   - OrderRequest { side, volume, take_profit, stop_loss }
   - TradeResult { success, order_id, error }
   - PositionStatus enum
4. Creer validation.rs avec:
   - validate_order() - verifie volume, TP/SL
   - validate_market_data() - verifie prix coherents
   - check_risk_limits() - verifie limites journalieres
5. Ajouter des tests unitaires pour chaque validation
6. Integrer les nouveaux types dans le code existant")

    dispatch_to_agent "codex" "$codex_task"

    log "INFO" "Toutes les taches predefinies ont ete dispatched"
}

# Point d'entree
main() {
    local specific_agent=""
    local specific_task=""

    while [[ $# -gt 0 ]]; do
        case $1 in
            --agent)
                specific_agent="$2"
                shift 2
                ;;
            --task-id)
                specific_task="$2"
                shift 2
                ;;
            --predefined)
                dispatch_predefined_tasks
                exit 0
                ;;
            *)
                echo "Usage: $0 [--agent AGENT] [--task-id TASK_ID] [--predefined]"
                exit 1
                ;;
        esac
    done

    if [[ -n "$specific_agent" && -n "$specific_task" ]]; then
        local task_file="$TASKS_DIR/${specific_task}_${specific_agent}.txt"
        if [[ -f "$task_file" ]]; then
            dispatch_to_agent "$specific_agent" "$task_file"
        else
            log "ERROR" "Tache non trouvee: $task_file"
            exit 1
        fi
    else
        log "INFO" "Utiliser --predefined pour dispatcher les taches predefinies"
        log "INFO" "Ou --agent AGENT --task-id TASK_ID pour une tache specifique"
    fi
}

main "$@"
