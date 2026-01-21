# CODEX TASK - Palm Oil Bot - URGENT FIX

**Date**: 2026-01-21 13:05
**Priority**: CRITICAL
**Status**: ASSIGNED

---

## TASK: Fix Compilation Errors (Blocking)

Le projet a une erreur de compilation bloquante à corriger IMMÉDIATEMENT.

### Erreur dans `src/modules/trading/ctrader.rs` ligne 415

**Code actuel (ERREUR):**
```rust
self.handle_spot_event(spot_event).await;
```

**Code corrigé:**
```rust
Self::handle_spot_event(spot_event, &self.prices).await;
```

### Actions:

1. Ouvrir `/home/julien/Documents/palm-oil-bot/src/modules/trading/ctrader.rs`
2. Aller à la ligne 415
3. Remplacer l'appel par la version corrigée
4. Vérifier: `cd /home/julien/Documents/palm-oil-bot && cargo check`

### Validation:
- [ ] `cargo check` = 0 erreurs
- [ ] `cargo test` passe

---

**Execute maintenant.**
