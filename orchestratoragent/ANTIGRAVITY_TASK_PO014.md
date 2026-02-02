# ANTIGRAVITY - TASK-PO-014: Fix FCPO Symbol ID Hardcodé

**Agent**: ANTIGRAVITY (Claude Opus 4.5 via proxy)
**Date**: 2026-01-23
**Priorité**: 🔴 CRITIQUE
**Durée estimée**: 25-35 min
**Status**: READY TO EXECUTE

---

## 📋 OBJECTIF

Résoudre le hardcoding du symbol ID FCPO dans `main.rs` en implémentant une résolution dynamique via cTrader Protobuf API.

---

## 🎯 CONTEXTE DU PROBLÈME

**Fichier**: `src/main.rs:22`

```rust
const FCPO_SYMBOL_ID: i64 = 1; // TODO: Get actual symbol ID
```

**Problème**: 
- Le symbol ID `1` est hardcodé et peut être incorrect
- cTrader peut retourner un ID différent selon le serveur (DEMO vs LIVE)
- Risque de trader le mauvais symbole

**Solution**: 
- Implémenter `get_symbol_id()` pour query dynamiquement via `ProtoOASymbolsListReq`

---

## 🎯 TÂCHES À EXÉCUTER

### ÉTAPE 1: Vérifier Proto Definitions

**Fichier**: `proto/ctrader.proto`

Vérifier si ces messages existent:

```protobuf
// Request symbols list
message ProtoOASymbolsListReq {
    required uint32 payload_type = 1; // 2121
    required string client_msg_id = 2;
    required int64 ctid_trader_account_id = 3;
}

// Response with symbols list
message ProtoOASymbolsListRes {
    required uint32 payload_type = 1; // 2122
    required string client_msg_id = 2;
    repeated ProtoOASymbol symbols = 3;
}

// Symbol definition
message ProtoOASymbol {
    required int64 symbol_id = 1;
    required string name = 2;
    optional bool enabled = 3;
    optional int32 digits = 4;
    // ... other fields
}
```

**Si absents**, ajouter ces définitions basées sur la doc cTrader:
- Consulter: https://help.ctrader.com/open-api/messages/
- Chercher `ProtoOASymbolsListReq` et `ProtoOASymbolsListRes`

### ÉTAPE 2: Implémenter `get_symbol_id()` dans CTraderClient

**Fichier**: `src/modules/trading/ctrader.rs`

Ajouter cette méthode à l'impl CTraderClient:

```rust
/// Query symbol ID by name (e.g., "FCPO")
/// 
/// # Arguments
/// * `symbol_name` - Symbol name to search for (e.g., "FCPO", "EURUSD")
/// 
/// # Returns
/// * `Ok(i64)` - Symbol ID if found
/// * `Err` - If symbol not found or request failed
pub async fn get_symbol_id(&mut self, symbol_name: &str) -> Result<i64> {
    tracing::info!("Querying symbol ID for: {}", symbol_name);
    
    // 1. Create ProtoOASymbolsListReq
    let msg_id = self.next_msg_id();
    let request = ProtoOASymbolsListReq {
        payload_type: 2121, // ProtoOASymbolsListReq type
        client_msg_id: msg_id.to_string(),
        ctid_trader_account_id: self.account_id,
    };
    
    // 2. Send request
    self.send_message(&request).await
        .context("Failed to send ProtoOASymbolsListReq")?;
    
    // 3. Wait for ProtoOASymbolsListRes (type 2122)
    let timeout = Duration::from_secs(10);
    let response: ProtoOASymbolsListRes = self
        .wait_for_message(2122, timeout)
        .await
        .context("Failed to receive ProtoOASymbolsListRes")?;
    
    tracing::debug!("Received {} symbols from cTrader", response.symbols.len());
    
    // 4. Find symbol by name (case-insensitive)
    let symbol = response.symbols.iter()
        .find(|s| s.name.eq_ignore_ascii_case(symbol_name))
        .ok_or_else(|| anyhow::anyhow!(
            "Symbol '{}' not found in {} available symbols. Available: {:?}",
            symbol_name,
            response.symbols.len(),
            response.symbols.iter()
                .take(10)
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
        ))?;
    
    tracing::info!("✅ Resolved symbol '{}' -> ID {}", symbol_name, symbol.symbol_id);
    
    Ok(symbol.symbol_id)
}
```

**Points clés**:
- ✅ Case-insensitive search (`eq_ignore_ascii_case`)
- ✅ Timeout de 10s pour éviter hang
- ✅ Logging détaillé pour debug
- ✅ Error message montre symbols disponibles si not found

### ÉTAPE 3: Modifier `main.rs` pour utiliser la résolution dynamique

**Fichier**: `src/main.rs`

**Avant** (lignes ~20-25):
```rust
const FCPO_SYMBOL_ID: i64 = 1; // TODO: Get actual symbol ID

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize...
    let mut ctrader_client = CTraderClient::connect(&config).await?;
```

**Après**:
```rust
// Remove const FCPO_SYMBOL_ID

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging, config...
    tracing_subscriber::fmt::init();
    let config = Config::from_env()?;
    
    // Connect to cTrader
    let mut ctrader_client = CTraderClient::connect(&config).await
        .context("Failed to connect to cTrader")?;
    
    // Authenticate
    ctrader_client.authenticate().await
        .context("Failed to authenticate cTrader")?;
    
    // ✅ Resolve FCPO symbol ID dynamically
    let fcpo_symbol_id = ctrader_client
        .get_symbol_id("FCPO")
        .await
        .context("Failed to resolve FCPO symbol ID - check symbol name and account permissions")?;
    
    tracing::info!("🌴 Trading FCPO with symbol ID: {}", fcpo_symbol_id);
    
    // Use fcpo_symbol_id in trading loop...
```

**Ensuite**, remplacer toutes les références à `FCPO_SYMBOL_ID` par `fcpo_symbol_id` dans main.rs.

### ÉTAPE 4: Ajouter Configuration

**Fichier**: `src/config.rs`

Ajouter champ pour symbol name configurable:

```rust
pub struct Config {
    // ... existing fields
    
    /// Symbol name to trade (e.g., "FCPO")
    pub symbol_name: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        
        Ok(Config {
            // ... existing fields
            symbol_name: env::var("SYMBOL_NAME")
                .unwrap_or_else(|_| "FCPO".to_string()),
        })
    }
}
```

**Fichier**: `.env.example`

Ajouter:
```env
# Trading Symbol
SYMBOL_NAME=FCPO
```

Puis modifier main.rs:
```rust
let fcpo_symbol_id = ctrader_client
    .get_symbol_id(&config.symbol_name) // Use config instead of hardcoded "FCPO"
    .await?;
```

---

## ✅ VALIDATION

### Test 1: Compilation
```bash
cargo build --release
```
✅ Doit compiler sans erreurs

### Test 2: Dry-Run (commenté)
Ajouter test dans `src/modules/trading/ctrader.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Ignore par défaut (requires cTrader connection)
    async fn test_get_symbol_id_fcpo() {
        // Setup config from .env
        let config = Config::from_env().unwrap();
        
        // Connect
        let mut client = CTraderClient::connect(&config).await.unwrap();
        client.authenticate().await.unwrap();
        
        // Resolve FCPO
        let symbol_id = client.get_symbol_id("FCPO").await.unwrap();
        
        assert!(symbol_id > 0, "Symbol ID should be positive");
        println!("✅ FCPO symbol ID: {}", symbol_id);
    }
}
```

Run avec:
```bash
cargo test --lib ctrader::tests::test_get_symbol_id_fcpo -- --ignored --nocapture
```

### Test 3: Check Logs
Au démarrage du bot, vérifier logs affichent:
```
INFO Querying symbol ID for: FCPO
DEBUG Received 150 symbols from cTrader
INFO ✅ Resolved symbol 'FCPO' -> ID 12345
INFO 🌴 Trading FCPO with symbol ID: 12345
```

---

## 📦 LIVRABLES

1. **Code modifié**:
   - `proto/ctrader.proto` (si messages manquants)
   - `src/modules/trading/ctrader.rs` (méthode `get_symbol_id()`)
   - `src/main.rs` (remove const, add dynamic resolution)
   - `src/config.rs` (add symbol_name field)
   - `.env.example` (add SYMBOL_NAME)

2. **Rapport**:
   Créer `SYMBOL_ID_RESOLUTION_REPORT.md`:
   ```markdown
   # Symbol ID Resolution - TASK-PO-014
   
   **Date**: 2026-01-23
   **Agent**: Antigravity
   
   ## Changements
   
   ✅ Removed hardcoded `FCPO_SYMBOL_ID = 1`
   ✅ Implemented `CTraderClient::get_symbol_id()`
   ✅ Added dynamic resolution in main.rs
   ✅ Made symbol name configurable via .env
   
   ## Test Results
   
   ```
   [Coller output de cargo build + test]
   ```
   
   ## Verified Symbol IDs (DEMO server)
   
   - FCPO: 12345 (example)
   
   ## Next Steps
   
   - Test on DEMO account startup
   - Verify correct symbol trades executed
   
   ## Status
   
   ✅ TASK-PO-014 COMPLETED
   ```

---

## 🚨 NOTES IMPORTANTES

- **cTrader Protobuf**: Si proto definitions manquent, consulter doc officielle
- **Error Handling**: Si symbol not found, bot doit CRASH (fail-fast) plutôt que trader mauvais symbol
- **DEMO First**: Tester UNIQUEMENT sur DEMO account
- **Logs**: Ajouter logs détaillés pour debug

---

## 📝 TODO CHECKLIST

- [ ] Vérifier proto/ctrader.proto (ProtoOASymbolsListReq/Res)
- [ ] Ajouter méthode `get_symbol_id()` dans ctrader.rs
- [ ] Modifier main.rs (remove const, add resolution)
- [ ] Ajouter symbol_name dans config.rs
- [ ] Update .env.example
- [ ] Compiler: `cargo build --release`
- [ ] Tester résolution (optional test)
- [ ] Créer SYMBOL_ID_RESOLUTION_REPORT.md
- [ ] Commit: "fix: resolve FCPO symbol ID dynamically via cTrader API (TASK-PO-014)"

---

**READY TO EXECUTE** - Lance cette tâche en priorité (CRITIQUE).
