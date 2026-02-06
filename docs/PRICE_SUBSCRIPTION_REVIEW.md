# Price Subscription & Data Flow Review

**Date**: 2026-02-03  
**Reviewer**: Orchestrator LLM  
**Task**: TODO-ORCH-LLM-004

---

## Executive Summary

⚠️ **CRITICAL ISSUE IDENTIFIED**: Price subscription lacks confirmation mechanism and timeout handling. Bot may wait indefinitely for price data that never arrives.

**Status**: 🟡 FUNCTIONAL BUT FRAGILE  
**Risk Level**: MEDIUM-HIGH (production trading could hang without price data)

---

## Current Implementation Analysis

### 1. Subscription Flow ([src/modules/trading/ctrader.rs:428-455](file:///mnt/c/Users/beatr/cacaobot/src/modules/trading/ctrader.rs#L428-L455))

```rust
pub async fn subscribe_to_symbol(&self, symbol_id: i64) -> Result<()> {
    // ...authentication check...
    
    let subscribe_req = ProtoOaSubscribeSpotsReq {
        payload_type: None,
        ctid_trader_account_id: account_id,
        symbol_id: vec![symbol_id],
        subscribe_to_spot_timestamp: Some(true),
    };
    
    let msg = new_proto_message(ProtoOaPayloadType::ProtoOaSubscribeSpotsReq, subscribe_req);
    self.send_message(msg).await?;
    
    // Track subscribed symbols for reconnection
    let mut symbols = self.subscribed_symbols.write().await;
    if !symbols.contains(&symbol_id) {
        symbols.push(symbol_id);
    }
    
    info!("Subscribed to symbol: {}", symbol_id);  // ⚠️ Premature log
    Ok(())  // ⚠️ Returns immediately without waiting for confirmation
}
```

**❌ Issues:**
1. **No confirmation wait**: Returns immediately after sending subscribe request
2. **No timeout**: Could wait forever for first price update
3. **No retry logic**: If subscription fails silently, no recovery mechanism
4. **Optimistic logging**: Logs "Subscribed" before actually receiving confirmation

### 2. Price Retrieval ([src/modules/trading/ctrader.rs:457-464](file:///mnt/c/Users/beatr/cacaobot/src/modules/trading/ctrader.rs#L457-L464))

```rust
pub async fn get_price(&self, symbol_id: i64) -> Result<Price> {
    let prices = self.prices.read().await;
    prices
        .get(&symbol_id)
        .cloned()
        .ok_or_else(|| CTraderError::InvalidResponse(
            format!("No price data for symbol {}", symbol_id)
        ).into())
}
```

**✅ Simple and correct**, but relies on price cache being populated by reader task.

### 3. Reader Task - Spot Event Handling ([src/modules/trading/ctrader.rs:714-720](file:///mnt/c/Users/beatr/cacaobot/src/modules/trading/ctrader.rs#L714-L720))

```rust
ProtoOaPayloadType::ProtoOaSpotEvent => {
    if let Some(payload) = &message.payload {
        if let Ok(spot_event) = ProtoOaSpotEvent::decode(payload.as_ref()) {
            Self::handle_spot_event(spot_event, &prices_arc).await;
        }
    }
}
```

**✅ Correct**: Asynchronously handles incoming price updates and populates cache.

### 4. Bot Main Loop - Price Fetch ([src/bot.rs:248-264](file:///mnt/c/Users/beatr/cacaobot/src/bot.rs#L248-L264))

```rust
_ = ticker.tick() => {
    let price = match self.ctrader.get_price(self.symbol_id).await {
        Ok(price) => price,
        Err(err) => {
            warn!("Failed to fetch price: {}", err);  // ⚠️ Logged every cycle
            if should_retry_ctrader(&err) {
                warn!("Attempting reconnect after price error");
                // ...reconnect logic...
            }
            continue;  // ⚠️ Skips tick, tries again next cycle
        }
    };
    
    let mid_price = (price.bid + price.ask) / 2.0;
    let tick = Tick::new(price.timestamp, mid_price);
    self.process_tick(tick).await?;
}
```

**⚠️ Issues:**
1. **Retry spam**: If price never arrives, logs warning every `cycle_interval_secs` (default 60s)
2. **No timeout**: Could run indefinitely without price data
3. **Reconnect may not help**: Reconnect re-subscribes, but if original subscription was broken, same issue persists

### 5. Test Connection Binary ([src/bin/test_connection.rs:70-83](file:///mnt/c/Users/beatr/cacaobot/src/bin/test_connection.rs#L70-L83))

```rust
// Wait 10 seconds for price data
tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

match client.get_price(symbol_id).await {
    Ok(price) => {
        info!("✓ Price received:");
        // ...
    }
    Err(e) => {
        error!("✗ No price data: {}", e);  // ⚠️ But test continues anyway
    }
}
```

**⚠️ Issue**: Arbitrary 10-second wait with no guarantee price will arrive.

---

## Root Cause Analysis

### cTrader API Behavior

According to [cTrader Open API docs](https://help.ctrader.com/open-api/):
- `ProtoOaSubscribeSpotsReq` → No explicit response message
- First `ProtoOaSpotEvent` confirms subscription success
- If symbol doesn't exist or no market data available, **no error is sent**

**Implication**: Client must implement timeout-based confirmation detection.

### Current Failure Modes

| Scenario | Current Behavior | Risk |
|----------|------------------|------|
| Symbol doesn't exist | No price, no error → infinite wait | HIGH |
| Market closed (no ticks) | No price updates → appears broken | MEDIUM |
| Network delay on subscribe | 10s wait in test, indefinite wait in bot | MEDIUM |
| Reader task crash | No price updates, no reconnect trigger | HIGH |
| Symbol ID typo | No error, no price → silent failure | HIGH |

---

## Recommended Improvements

### 1. **Add Subscription Confirmation with Timeout** ⭐ CRITICAL

```rust
pub async fn subscribe_to_symbol(&self, symbol_id: i64) -> Result<()> {
    if !*self.authenticated.read().await {
        return Err(CTraderError::AuthFailed("Not authenticated".into()).into());
    }

    let account_id = self.config.active_account_id().parse::<i64>()
        .map_err(|e| CTraderError::Protocol(format!("Invalid account ID: {}", e)))?;

    let subscribe_req = ProtoOaSubscribeSpotsReq {
        payload_type: None,
        ctid_trader_account_id: account_id,
        symbol_id: vec![symbol_id],
        subscribe_to_spot_timestamp: Some(true),
    };

    let msg = new_proto_message(ProtoOaPayloadType::ProtoOaSubscribeSpotsReq, subscribe_req);
    self.send_message(msg).await?;

    // Track subscribed symbols for reconnection
    let mut symbols = self.subscribed_symbols.write().await;
    if !symbols.contains(&symbol_id) {
        symbols.push(symbol_id);
    }

    // ✅ NEW: Wait for first price update to confirm subscription
    let timeout_duration = Duration::from_secs(30);
    let start = Instant::now();
    
    while start.elapsed() < timeout_duration {
        if self.prices.read().await.contains_key(&symbol_id) {
            info!("✅ Subscribed to symbol {} - first price received", symbol_id);
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // ⚠️ Timeout - subscription may have failed
    warn!(
        "⚠️ Subscription to symbol {} sent, but no price data received after {}s. \
        Symbol may not exist, market may be closed, or network issue occurred.",
        symbol_id, timeout_duration.as_secs()
    );
    
    // Return Ok but log warning (allow bot to continue in case market is just closed)
    Ok(())
}
```

### 2. **Add Price Availability Check Before Bot Loop** ⭐ CRITICAL

```rust
// In bot.rs after subscribe_to_symbol()
pub async fn wait_for_initial_price(&self, timeout_secs: u64) -> Result<Price> {
    let timeout_duration = Duration::from_secs(timeout_secs);
    let start = Instant::now();
    
    while start.elapsed() < timeout_duration {
        if let Ok(price) = self.ctrader.get_price(self.symbol_id).await {
            info!("✅ Initial price received: bid={:.2} ask={:.2}", price.bid, price.ask);
            return Ok(price);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    Err(BotError::Other(format!(
        "No price data received for symbol {} after {}s. \
        Possible causes:\n\
        - Symbol does not exist or is delisted\n\
        - Market is closed (check trading hours)\n\
        - Network connectivity issues\n\
        - cTrader subscription failed",
        self.symbol_id, timeout_secs
    )))
}

// Then in run() method:
self.ctrader.subscribe_to_symbol(self.symbol_id).await?;
self.wait_for_initial_price(30).await?;  // ✅ Block until first price or timeout
```

### 3. **Add Reader Task Health Check** ⭐ HIGH PRIORITY

```rust
// Add to CTraderClient struct
pub struct CTraderClient {
    // ... existing fields ...
    last_message_time: Arc<RwLock<Option<Instant>>>,
}

// In reader task loop:
async fn start_reader(&self) -> Result<()> {
    // ... existing setup ...
    
    loop {
        match Self::read_message(&mut stream_guard).await {
            Ok(message) => {
                *last_message_time_clone.write().await = Some(Instant::now());
                // ... existing message handling ...
            }
            Err(e) => {
                warn!("Reader error: {}", e);
                break;
            }
        }
    }
}

// Add public method to check reader health
pub async fn is_reader_alive(&self) -> bool {
    if let Some(last_time) = *self.last_message_time.read().await {
        last_time.elapsed() < Duration::from_secs(120)  // No messages in 2 min = dead
    } else {
        false
    }
}
```

### 4. **Improve Bot Loop Price Error Handling**

```rust
_ = ticker.tick() => {
    // ✅ Check reader health first
    if !self.ctrader.is_reader_alive().await {
        error!("❌ Reader task appears dead - forcing reconnect");
        let _ = self.ctrader.disconnect().await;
        if connect_with_retry(&self.ctrader).await.is_ok()
            && authenticate_with_retry(&self.ctrader).await.is_ok()
            && self.ctrader.subscribe_to_symbol(self.symbol_id).await.is_ok()
        {
            self.wait_for_initial_price(30).await?;
            continue;
        } else {
            return Err(BotError::Other("Failed to recover from reader death".into()));
        }
    }
    
    let price = match self.ctrader.get_price(self.symbol_id).await {
        Ok(price) => price,
        Err(err) => {
            // ✅ Track consecutive failures
            static CONSECUTIVE_FAILURES: AtomicU32 = AtomicU32::new(0);
            let failures = CONSECUTIVE_FAILURES.fetch_add(1, Ordering::SeqCst);
            
            if failures == 0 {
                warn!("Failed to fetch price: {}", err);
            } else if failures < 5 {
                debug!("Price fetch failed ({} consecutive): {}", failures + 1, err);
            } else if failures == 5 {
                error!("❌ 5 consecutive price fetch failures - attempting reconnect");
                // ... reconnect logic ...
            }
            
            continue;
        }
    };
    
    // ✅ Reset failure counter on success
    static CONSECUTIVE_FAILURES: AtomicU32 = AtomicU32::new(0);
    CONSECUTIVE_FAILURES.store(0, Ordering::SeqCst);
    
    // ... rest of tick processing ...
}
```

### 5. **Add Subscription Validation Test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscribe_receives_price() {
        let config = CTraderConfig::from_env().unwrap();
        let client = CTraderClient::new(config, CTraderEnvironment::Demo).unwrap();
        
        client.connect().await.unwrap();
        client.authenticate().await.unwrap();
        
        let symbol_id = client.get_symbol_id("FCPO").await.unwrap();
        client.subscribe_to_symbol(symbol_id).await.unwrap();
        
        // ✅ Wait up to 30 seconds for first price
        let mut received_price = false;
        for _ in 0..60 {
            if client.get_price(symbol_id).await.is_ok() {
                received_price = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        
        assert!(received_price, "No price received within 30 seconds of subscription");
    }
    
    #[tokio::test]
    async fn test_invalid_symbol_subscription() {
        let config = CTraderConfig::from_env().unwrap();
        let client = CTraderClient::new(config, CTraderEnvironment::Demo).unwrap();
        
        client.connect().await.unwrap();
        client.authenticate().await.unwrap();
        
        // Subscribe to non-existent symbol
        let result = client.subscribe_to_symbol(999999).await;
        
        // Should succeed (cTrader doesn't send error)
        assert!(result.is_ok());
        
        // But price should never arrive
        tokio::time::sleep(Duration::from_secs(5)).await;
        let price_result = client.get_price(999999).await;
        assert!(price_result.is_err(), "Should not receive price for invalid symbol");
    }
}
```

### 6. **Add Heartbeat Validation for Price Staleness**

```rust
// In bot loop
let price = match self.ctrader.get_price(self.symbol_id).await {
    Ok(price) => {
        // ✅ Check if price is stale (no updates in 5 minutes)
        if price.timestamp < Utc::now() - chrono::Duration::minutes(5) {
            warn!(
                "⚠️ Stale price data: last update was {} ago",
                Utc::now().signed_duration_since(price.timestamp)
            );
            
            // Re-subscribe to refresh
            info!("Re-subscribing to symbol {} to refresh price feed", self.symbol_id);
            self.ctrader.subscribe_to_symbol(self.symbol_id).await?;
            continue;
        }
        price
    }
    // ... existing error handling ...
};
```

---

## Additional Edge Cases

### Market Closed Handling

```rust
// Add to config.rs
pub struct TradingHours {
    pub start_hour: u32,  // UTC
    pub end_hour: u32,
    pub days: Vec<Weekday>,
}

impl TradingHours {
    pub fn is_market_open(&self) -> bool {
        let now = Utc::now();
        let hour = now.hour();
        let weekday = now.weekday();
        
        self.days.contains(&weekday) 
            && hour >= self.start_hour 
            && hour < self.end_hour
    }
}

// In bot.rs
if !self.trading_hours.is_market_open() {
    info!("Market closed - skipping price check");
    continue;
}
```

### Symbol Existence Pre-check

```rust
// Before subscribing
pub async fn validate_symbol_exists(&self, symbol_id: i64) -> Result<bool> {
    let symbols_list = self.get_symbols_list().await?;
    
    let exists = symbols_list.iter().any(|s| s.symbol_id == symbol_id);
    
    if !exists {
        warn!("Symbol {} does not exist in broker's symbol list", symbol_id);
    }
    
    Ok(exists)
}
```

---

## Priority Ranking

| Priority | Improvement | Impact | Effort |
|----------|-------------|--------|--------|
| 🔴 P0 | Subscription confirmation timeout (#1) | Prevents infinite hangs | 1 hour |
| 🔴 P0 | Wait for initial price before loop (#2) | Early failure detection | 30 min |
| 🟡 P1 | Reader task health check (#3) | Detects silent failures | 2 hours |
| 🟡 P1 | Consecutive failure tracking (#4) | Reduces log spam | 1 hour |
| 🟢 P2 | Subscription validation test (#5) | Prevents regressions | 1 hour |
| 🟢 P2 | Price staleness check (#6) | Detects stuck feeds | 30 min |
| 🟢 P3 | Market hours check | User experience | 2 hours |
| 🟢 P3 | Symbol existence pre-check | Early validation | 1 hour |

**Total P0 effort**: ~1.5 hours  
**Total P0+P1 effort**: ~4.5 hours

---

## Conclusion

**Current State**: ❌ BLOCKING ISSUES for production  
**Risk**: Bot could hang indefinitely without price data, requiring manual restart

**Recommended Action**:
1. ✅ Implement P0 fixes immediately (subscription confirmation + initial price wait)
2. ✅ Add P1 health checks before production deployment
3. ⏳ P2/P3 enhancements for operational excellence

**With P0+P1 fixes**: ✅ PRODUCTION READY

---

**Review Completed**: 2026-02-03  
**Reviewer**: Orchestrator LLM  
**Status**: 🟡 CONDITIONAL APPROVAL (pending P0 fixes)
