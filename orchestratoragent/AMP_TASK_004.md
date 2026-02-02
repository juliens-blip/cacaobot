# AMP TASK 004 - Symbol ID Discovery

**Date**: 2026-01-21 18:40
**Priority**: HAUTE  
**Status**: ACTIVE
**Assigné**: AMP (moi-même)

## Objectif

Remplacer la constante hardcodée `FCPO_SYMBOL_ID = 1` par une découverte automatique via cTrader API.

## Implémentation

### 1. Ajouter méthode dans ctrader.rs

```rust
/// Get symbol ID by name
pub async fn get_symbol_id(&self, symbol_name: &str) -> Result<i64> {
    // Send ProtoOASymbolsListReq
    let req = ProtoOASymbolsListReq {
        ctid_trader_account_id: self.account_id,
        include_archived_symbols: Some(false),
    };
    
    self.send_message(req).await?;
    
    // Wait for ProtoOASymbolsListRes
    // Parse response and find symbol by name
    // Return symbol_id
}
```

### 2. Update main.rs initialization

Remplacer:
```rust
const FCPO_SYMBOL_ID: i64 = 1; // TODO
```

Par:
```rust
// In initialize()
let fcpo_id = self.ctrader.get_symbol_id("FCPO").await?;
self.fcpo_symbol_id = fcpo_id;
```

### 3. Tests

`tests/symbol_discovery_test.rs`:
- Mock ProtoOASymbolsListRes
- Test parsing
- Test error handling

---
**Start**: Maintenant
**ETA**: 20 min
