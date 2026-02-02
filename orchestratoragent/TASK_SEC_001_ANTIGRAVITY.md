# TASK-SEC-001: Circuit Breakers Live Validation

**Assigné à**: Antigravity  
**Priorité**: HAUTE  
**ETA**: 25min

## Objectif

Créer tests de validation LIVE pour circuit breakers.

## Fichier

`tests/circuit_breakers_live_test.rs`

## Scénarios à Tester

### 1. Daily Loss Limit (-5%)
- Simuler 5 trades perdants
- Vérifier bot s'arrête
- Vérifier persistence de l'état

### 2. Consecutive Losses (3x)
- Simuler 3 pertes consécutives
- Vérifier cooldown activé
- Tester recovery après cooldown

### 3. Volatility Spike Detector
- Injecter volatilité > 3%
- Vérifier pause trading
- Documenter seuils

### 4. Recovery After Circuit Break
- Vérifier bot reprend après cooldown
- Vérifier état persiste

## Utiliser

`position_manager.rs` (déjà implémenté) pour persistence validation

## Livrable

- `circuit_breakers_live_test.rs` avec 4+ tests
- Documentation comportements
- Rapport edge cases découverts

## Rapport

Écris ton avancement dans `orchestratoragent/ANTIGRAVITY_RESPONSE.md`
