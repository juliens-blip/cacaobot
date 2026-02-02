# TASK-SEC-002: Position Reconciliation Network Tests

**Assigné à**: Backend Architect (via agents_library)  
**Priorité**: HAUTE  
**ETA**: 30min

## Objectif

Créer tests de reconciliation de positions avec connexions réseau instables.

## Fichier

`tests/position_reconciliation_network_test.rs`

## Scénarios à Tester

### 1. test_network_disconnect_during_trade()
- Ouvrir position
- Simuler déconnexion réseau
- Reconnect
- Vérifier position réconciliée depuis persistence

### 2. test_missing_execution_event()
- Envoyer ordre
- Drop execution event (simulé)
- Vérifier reconciliation détecte position manquante
- Sync depuis broker

### 3. test_orphaned_position_cleanup()
- Créer position locale
- Pas de position sur cTrader (broker)
- Vérifier cleanup orphaned position

### 4. test_concurrent_reconciliation()
- Lancer 3 reconciliations simultanées
- Vérifier pas de race conditions
- Utiliser Arc<RwLock<>> pour thread safety

## Utiliser

`position_manager.rs` (déjà implémenté):
- `PersistentPositionManager::with_persistence(path)`
- `reconcile_with_broker(broker_positions)`
- `sync_missing_position()`
- `remove_orphaned_position()`

## Tests Structure

```rust
use palm_oil_bot::modules::trading::position_manager::{
    PersistentPositionManager, Position, BrokerPosition
};
use tokio::time::{sleep, Duration};
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_network_disconnect_during_trade() {
    // Implementation
}
```

## Livrables

- `tests/position_reconciliation_network_test.rs` complet
- 4+ tests async
- Documentation comportements edge cases
- Rapport dans `orchestratoragent/BACKEND_RESPONSE.md`

## Agent

Utilise **backend-architect.md** de agents_library (expert async, architecture)
