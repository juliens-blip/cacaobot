#!/bin/bash
# =============================================================================
# send-initial-tasks.sh - Envoie les taches initiales a tous les agents
# =============================================================================
# Ce script envoie des taches longues et autonomes a chaque agent
# pour qu'ils commencent a travailler immediatement
# Usage: ./send-initial-tasks.sh
# =============================================================================

set -e

# Configuration
PROJECT_DIR="/home/julien/Documents/palm-oil-bot"
ORCHESTRATION_DIR="$PROJECT_DIR/orchestratoragent"
SESSION_NAME="palm-oil-orchestration"
LOG_FILE="$ORCHESTRATION_DIR/logs/initial_tasks.log"

# Creer le repertoire de logs
mkdir -p "$(dirname "$LOG_FILE")"

# Fonction de log
log() {
    local message="$1"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] $message" | tee -a "$LOG_FILE"
}

# Verifier que la session existe
check_session() {
    if ! tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
        log "ERREUR: Session tmux '$SESSION_NAME' non trouvee"
        log "Lancez d'abord: ./start-orchestration-v2.sh"
        exit 1
    fi
}

# Envoyer tache a AMP
send_task_to_amp() {
    log "Envoi tache a AMP..."

    tmux send-keys -t "$SESSION_NAME:1" "
[TACHE ORCHESTRATEUR - AUTONOMIE MAXIMALE]

AGENT: AMP
TASK_ID: TASK-PO-020
PRIORITE: HAUTE
TIMESTAMP: $(date '+%Y-%m-%d %H:%M:%S')

=== CONTEXTE ===
Projet: Palm Oil Trading Bot (Rust)
Tu travailles sur le projet situe dans: $PROJECT_DIR
Ce bot de trading automatise utilise RSI et sentiment analysis pour trader le FCPO.
L'objectif est 2-3% de rentabilite journaliere via scalping.

=== OBJECTIF ===
Implementer un systeme complet de logging et monitoring pour le bot de trading.
Le systeme doit permettre de suivre toutes les operations en temps reel et
de debugger les problemes rapidement.

=== FICHIERS A CREER/MODIFIER ===

1. src/modules/monitoring/logging.rs (NOUVEAU)
   - Systeme de logging structure avec tracing
   - Niveaux: DEBUG, INFO, WARN, ERROR
   - Rotation des logs quotidienne
   - Format JSON pour analyse

2. src/modules/monitoring/alerts.rs (NOUVEAU)
   - Systeme d'alertes pour events critiques
   - Alertes: connexion perdue, grosse perte, erreur API
   - Seuils configurables

3. src/modules/monitoring/mod.rs
   - Ajouter les exports des nouveaux modules
   - Integration avec le systeme existant

4. tests/monitoring_test.rs (NOUVEAU)
   - Tests pour le logging
   - Tests pour les alertes

=== INSTRUCTIONS DETAILLEES ===

1. LIRE d'abord les fichiers existants:
   - src/modules/monitoring/metrics.rs
   - src/modules/monitoring/dashboard.rs
   - Cargo.toml (verifier les deps)

2. CREER logging.rs avec:
   - struct Logger avec configuration
   - fn init_logging() -> initialise tracing
   - fn log_trade(trade: &Trade) -> log un trade
   - fn log_signal(signal: &Signal) -> log un signal
   - fn log_error(error: &Error) -> log une erreur
   - Rotation quotidienne des fichiers

3. CREER alerts.rs avec:
   - struct AlertConfig (seuils configurables)
   - struct AlertManager
   - enum AlertType (ConnectionLost, BigLoss, APIError, HighDrawdown)
   - fn check_and_alert() -> verifie les conditions
   - fn send_alert(alert: Alert) -> envoie l'alerte

4. MODIFIER mod.rs pour exporter les nouveaux modules

5. CREER les tests:
   - test_logging_initialization
   - test_trade_logging
   - test_alert_thresholds
   - test_alert_triggering

6. VERIFIER la compilation: cargo build
7. LANCER les tests: cargo test monitoring

=== CRITERES DE VALIDATION ===
- [ ] Code compile sans erreur
- [ ] Tous les tests passent
- [ ] Logging fonctionne en mode DEBUG
- [ ] Alertes se declenchent correctement
- [ ] Documentation des fonctions publiques

=== APRES COMPLETION ===

1. Mettre a jour ORCHESTRATION_STATUS.md:
   - Statut: COMPLETED
   - Fichiers: logging.rs, alerts.rs, tests
   - Duree reelle

2. Mettre a jour CLAUDE.md section 'Log des Actions LLM':
   | [HEURE] | AMP | TASK-PO-020: Logging + Alerts system | COMPLETED |

3. Si tu finis AVANT les autres:
   - Lis ORCHESTRATION_STATUS.md pour nouvelles taches
   - Ou propose des ameliorations au code existant

COMMENCE IMMEDIATEMENT - NE POSE PAS DE QUESTIONS - AGIS AVEC AUTONOMIE
" Enter

    log "Tache envoyee a AMP"
}

# Envoyer tache a Antigravity
send_task_to_antigravity() {
    log "Envoi tache a Antigravity..."

    tmux send-keys -t "$SESSION_NAME:3" "
[TACHE ORCHESTRATEUR - ANALYSE APPROFONDIE]

AGENT: ANTIGRAVITY
TASK_ID: TASK-PO-021
PRIORITE: HAUTE
TIMESTAMP: $(date '+%Y-%m-%d %H:%M:%S')

=== CONTEXTE ===
Projet: Palm Oil Trading Bot (Rust)
Chemin: $PROJECT_DIR
Le bot utilise RSI + Sentiment pour trader le FCPO (Palm Oil CFD).
Objectif: 2-3% rentabilite journaliere.

=== OBJECTIF ===
Effectuer une analyse COMPLETE et APPROFONDIE de la strategie de trading
actuelle et proposer des ameliorations concretes et implementables.

=== FICHIERS A ANALYSER ===

1. src/modules/trading/strategy.rs
   - Logique de decision BUY/SELL
   - Seuils RSI et sentiment

2. src/modules/trading/indicators.rs
   - Calcul RSI
   - EMA Calculator

3. src/modules/trading/orders.rs
   - Gestion des positions
   - Trailing stop

4. src/bin/backtest.rs
   - Logique de backtesting
   - Generation de donnees

5. docs/strategy_improvements.md (si existe)
   - Ameliorations precedemment proposees

=== LIVRABLES ATTENDUS ===

1. docs/strategy_analysis_v2.md (NOUVEAU - minimum 1500 mots)
   Contenu requis:

   ## Resume Executif
   [3-5 phrases cles]

   ## Analyse des Parametres Actuels
   ### RSI
   - Seuils actuels (30/70)
   - Efficacite estimee
   - Recommandations

   ### Sentiment
   - Score range (-100 to +100)
   - Correlation avec prix
   - Fiabilite de Perplexity API

   ### Risk/Reward
   - Ratio actuel
   - Breakeven win rate
   - Comparaison avec standards

   ## Backtesting Theorique
   - Scenarios testes
   - Resultats estimes
   - Conditions de marche

   ## Identification des Faiblesses
   - Liste numerotee
   - Impact de chaque faiblesse
   - Solutions proposees

   ## Recommandations d'Optimisation
   ### Priorite Haute
   [Avec justification detaillee]

   ### Priorite Moyenne
   [Avec justification]

   ### Priorite Basse
   [Optionnel mais benefique]

   ## Plan d'Implementation
   - Phase 1: Quick wins
   - Phase 2: Ameliorations moyennes
   - Phase 3: Optimisations avancees

   ## Risques et Mitigations
   [Tableau avec risque/impact/mitigation]

   ## Metriques de Succes
   [KPIs mesurables]

   ## Conclusion
   [Synthese et prochaines etapes]

=== INSTRUCTIONS DETAILLEES ===

1. LIRE tous les fichiers mentionnes ci-dessus
2. UTILISER ton extended thinking pour analyser en profondeur
3. CONSIDERER plusieurs angles:
   - Performance historique (si donnees disponibles)
   - Robustesse aux conditions de marche
   - Simplicite vs complexite
   - Cout d'implementation
4. DOCUMENTER toutes tes reflexions
5. PROPOSER des ameliorations CONCRETES avec code exemple

=== CRITERES DE VALIDATION ===
- [ ] Rapport minimum 1500 mots
- [ ] Toutes les sections presentes
- [ ] Recommandations actionnables
- [ ] Code exemple quand pertinent
- [ ] Risques identifies

=== APRES COMPLETION ===

1. Mettre a jour ORCHESTRATION_STATUS.md
2. Mettre a jour CLAUDE.md
3. Si tu vois des problemes CRITIQUES:
   - Les documenter immediatement
   - Proposer des fixes urgents

COMMENCE IMMEDIATEMENT - UTILISE TON EXTENDED THINKING - ANALYSE EN PROFONDEUR
" Enter

    log "Tache envoyee a Antigravity"
}

# Envoyer tache a Codex
send_task_to_codex() {
    log "Envoi tache a Codex..."

    tmux send-keys -t "$SESSION_NAME:4" "
[TACHE ORCHESTRATEUR - GENERATION DE CODE]

AGENT: CODEX
TASK_ID: TASK-PO-022
PRIORITE: HAUTE
TIMESTAMP: $(date '+%Y-%m-%d %H:%M:%S')

=== CONTEXTE ===
Projet: Palm Oil Trading Bot (Rust)
Chemin: $PROJECT_DIR
Bot de trading automatise en Rust pour FCPO.

=== OBJECTIF ===
Generer un systeme complet de types et validations pour renforcer
la robustesse du bot de trading. Tous les types doivent etre
fortement types avec des validations strictes.

=== FICHIERS A CREER ===

1. src/modules/trading/types.rs (NOUVEAU - ~300 lignes)

   Types a creer:

   a) MarketTick
      - symbol: String
      - bid: f64
      - ask: f64
      - timestamp: i64
      - volume: Option<f64>
      + Methodes: spread(), mid_price(), is_valid()

   b) OrderRequest
      - id: String
      - symbol: String
      - side: OrderSide
      - order_type: OrderType
      - volume: f64
      - price: Option<f64>
      - take_profit: Option<f64>
      - stop_loss: Option<f64>
      + Builder pattern

   c) OrderResponse
      - id: String
      - status: OrderStatus
      - filled_volume: f64
      - fill_price: f64
      - timestamp: i64
      - error: Option<String>

   d) TradeRecord
      - id: String
      - order_id: String
      - symbol: String
      - side: OrderSide
      - entry_price: f64
      - exit_price: Option<f64>
      - volume: f64
      - pnl: f64
      - opened_at: i64
      - closed_at: Option<i64>
      - close_reason: Option<CloseReason>

   e) Enums:
      - OrderSide { Buy, Sell }
      - OrderType { Market, Limit, Stop, StopLimit }
      - OrderStatus { Pending, Filled, PartiallyFilled, Cancelled, Rejected }
      - CloseReason { TakeProfit, StopLoss, TrailingStop, Manual, Timeout }

   Tous les types doivent avoir:
   - #[derive(Debug, Clone, Serialize, Deserialize)]
   - impl Display pour les types principaux
   - impl Default quand pertinent
   - Documentation /// pour chaque struct/enum

2. src/modules/trading/validation.rs (NOUVEAU - ~200 lignes)

   Fonctions a creer:

   a) validate_order_request(order: &OrderRequest) -> Result<(), ValidationError>
      - Verifie volume > 0
      - Verifie prix valides
      - Verifie TP > entry pour BUY
      - Verifie SL < entry pour BUY

   b) validate_market_tick(tick: &MarketTick) -> Result<(), ValidationError>
      - Verifie bid > 0
      - Verifie ask >= bid
      - Verifie symbol non vide

   c) validate_position_limits(
        current_positions: &[Position],
        max_positions: usize,
        max_exposure: f64
      ) -> Result<(), ValidationError>

   d) validate_daily_loss(
        current_loss: f64,
        max_loss_percent: f64,
        initial_balance: f64
      ) -> Result<(), ValidationError>

   ValidationError enum avec:
   - InvalidVolume(f64)
   - InvalidPrice(f64)
   - InvalidStopLoss(String)
   - InvalidTakeProfit(String)
   - PositionLimitExceeded(usize, usize)
   - DailyLossLimitExceeded(f64, f64)

3. tests/types_validation_test.rs (NOUVEAU - ~150 lignes)

   Tests requis:
   - test_market_tick_creation
   - test_market_tick_spread
   - test_order_request_builder
   - test_order_request_validation_success
   - test_order_request_validation_invalid_volume
   - test_order_request_validation_invalid_sl
   - test_trade_record_pnl
   - test_position_limits
   - test_daily_loss_limit

=== INSTRUCTIONS DETAILLEES ===

1. LIRE les fichiers existants:
   - src/modules/trading/orders.rs
   - src/modules/trading/mod.rs
   - src/lib.rs

2. CREER types.rs avec TOUS les types listes
   - Utiliser les derives appropriees
   - Ajouter documentation
   - Implementer les traits necessaires

3. CREER validation.rs avec TOUTES les fonctions
   - Gestion d'erreurs avec thiserror
   - Messages d'erreur clairs

4. CREER les tests avec TOUS les cas listes
   - Tests positifs et negatifs
   - Edge cases

5. MODIFIER mod.rs pour exporter les nouveaux modules:
   pub mod types;
   pub mod validation;
   pub use types::*;
   pub use validation::*;

6. VERIFIER:
   cargo build
   cargo test types_validation

=== CODE EXEMPLE POUR REFERENCE ===

\`\`\`rust
// types.rs exemple
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderSide::Buy => write!(f, \"BUY\"),
            OrderSide::Sell => write!(f, \"SELL\"),
        }
    }
}
\`\`\`

=== CRITERES DE VALIDATION ===
- [ ] Tous les types crees avec derives corrects
- [ ] Toutes les validations implementees
- [ ] Tous les tests ecrits et passent
- [ ] Documentation presente
- [ ] Code compile sans warnings

=== APRES COMPLETION ===

1. Mettre a jour ORCHESTRATION_STATUS.md
2. Mettre a jour CLAUDE.md
3. Si tu finis tot, propose des types additionnels utiles

COMMENCE IMMEDIATEMENT - CODE PROPRE - TESTS COMPLETS - PAS DE QUESTIONS
" Enter

    log "Tache envoyee a Codex"
}

# Mettre a jour le statut
update_status() {
    log "Mise a jour du statut..."

    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    # Mettre a jour ORCHESTRATION_STATUS.md
    cat > "$PROJECT_DIR/ORCHESTRATION_STATUS.md" << STATUSEOF
# Multi-LLM Orchestration Status v2

**Orchestrateur**: Claude
**Session tmux**: \`$SESSION_NAME\`
**Date de demarrage**: $timestamp

---

## Agent Status

| Agent | Fenetre | Tache | Status | Last Update |
|-------|---------|-------|--------|-------------|
| **Claude** | 0-Claude | Orchestration | ACTIVE | $timestamp |
| **AMP** | 1-AMP | TASK-PO-020: Logging + Alerts | WORKING | $timestamp |
| **Proxy** | 2-Proxy | Communication | STANDBY | $timestamp |
| **Antigravity** | 3-Antigravity | TASK-PO-021: Strategy Analysis v2 | WORKING | $timestamp |
| **Codex** | 4-Codex | TASK-PO-022: Types + Validation | WORKING | $timestamp |

---

## Queue des Taches

### En Cours (In Progress)

| ID | Tache | Agent | Debut | ETA |
|----|-------|-------|-------|-----|
| TASK-PO-020 | Logging + Alerts system | AMP | $timestamp | 20 min |
| TASK-PO-021 | Strategy Analysis v2 | Antigravity | $timestamp | 15 min |
| TASK-PO-022 | Types + Validation | Codex | $timestamp | 15 min |

### En Attente (Pending)

| ID | Tache | Agent Cible | Priorite |
|----|-------|-------------|----------|
| TASK-PO-023 | Integration tests complets | AMP | MOYENNE |
| TASK-PO-024 | Performance profiling | Antigravity | MOYENNE |
| TASK-PO-025 | Error handling improvement | Codex | BASSE |

### Completees (Completed)

_En attente des premieres completions_

---

## Log des Actions

| Heure | Agent | Action | Status |
|-------|-------|--------|--------|
| $timestamp | System | Demarrage orchestration v2 | OK |
| $timestamp | System | Envoi taches initiales | OK |
| $timestamp | AMP | Reception TASK-PO-020 | STARTED |
| $timestamp | Antigravity | Reception TASK-PO-021 | STARTED |
| $timestamp | Codex | Reception TASK-PO-022 | STARTED |

---

## Notes d'Orchestration

- Tous les agents ont recu leurs taches initiales
- Surveillance active requise
- Prochaine verification dans 5 minutes

**Last Update**: $timestamp
STATUSEOF

    log "Statut mis a jour"
}

# Point d'entree
main() {
    log "=========================================="
    log "Envoi des taches initiales aux agents"
    log "=========================================="

    check_session

    # Attendre que les agents soient prets
    log "Attente de 5 secondes pour que les agents soient prets..."
    sleep 5

    # Envoyer les taches
    send_task_to_amp
    sleep 2

    send_task_to_antigravity
    sleep 2

    send_task_to_codex
    sleep 2

    # Mettre a jour le statut
    update_status

    log "=========================================="
    log "Toutes les taches initiales ont ete envoyees"
    log "=========================================="
    log ""
    log "Verifiez la progression avec:"
    log "  cat $PROJECT_DIR/ORCHESTRATION_STATUS.md"
    log ""
    log "Ou attachez la session tmux:"
    log "  tmux attach -t $SESSION_NAME"
}

main "$@"
