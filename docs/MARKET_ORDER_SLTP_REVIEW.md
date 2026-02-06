# MARKET Order SL/TP Handling - Code Review

**Date**: 2026-02-03  
**Reviewer**: Orchestrator LLM  
**Task**: TODO-ORCH-LLM-002

---

## Summary

The MARKET order SL/TP implementation uses **relative distance** (pips/points) instead of absolute prices, as required by cTrader API. The implementation is **correct** with proper validation, normalization, and safeguards.

---

## Code Analysis

### 1. Relative Distance Calculation ([src/bot.rs:608-617](file:///mnt/c/Users/beatr/cacaobot/src/bot.rs#L608-L617))

```rust
fn relative_distance(&self, entry: f64, target: f64) -> Option<i64> {
    if !entry.is_finite() || !target.is_finite() {
        return None;
    }
    let diff = (entry - target).abs();
    if diff <= 0.0 {
        return None;
    }
    Some((diff * 100000.0).round() as i64)
}
```

**✅ Correct Implementation:**
- Validates both prices are finite (prevents NaN/Infinity)
- Returns `None` if distance is zero (prevents invalid orders)
- Converts to pips: `100000` = 5 decimal places (standard for commodities)
- Uses `.abs()` for bidirectional calculation (works for BUY/SELL)

### 2. OrderTicket Construction ([src/bot.rs:508-517](file:///mnt/c/Users/beatr/cacaobot/src/bot.rs#L508-L517))

```rust
let ticket = OrderTicket {
    symbol_id: self.symbol_id,
    side: trade_side,
    volume: volume_units,
    stop_loss: Some(stop_loss),
    take_profit: Some(take_profit),
    relative_stop_loss: self.relative_distance(entry_price, stop_loss),
    relative_take_profit: self.relative_distance(entry_price, take_profit),
    label: Some("PalmOilBot".to_string()),
};
```

**✅ Correct:**
- Stores BOTH absolute prices (for display) AND relative distances (for API)
- Passes `entry_price` as reference point for distance calculation

### 3. cTrader API Message ([src/modules/trading/ctrader.rs:475-500](file:///mnt/c/Users/beatr/cacaobot/src/modules/trading/ctrader.rs#L475-L500))

```rust
let order_req = ProtoOaNewOrderReq {
    order_type: ProtoOaOrderType::Market as i32,
    // ...
    // For MARKET orders, cTrader requires relative SL/TP (absolute values rejected).
    stop_loss: None,              // ✅ Absolute values NOT sent
    take_profit: None,            // ✅ Absolute values NOT sent
    relative_stop_loss: ticket.relative_stop_loss,     // ✅ Relative used
    relative_take_profit: ticket.relative_take_profit, // ✅ Relative used
    // ...
};
```

**✅ Correct:**
- Explicitly sets `stop_loss: None` and `take_profit: None`
- Uses ONLY `relative_stop_loss` and `relative_take_profit`
- Documentation comment explains why
- Matches cTrader API specification

### 4. TP/SL Normalization ([src/bot.rs:619-658](file:///mnt/c/Users/beatr/cacaobot/src/bot.rs#L619-L658))

```rust
fn normalize_tp_sl(&self, side: OrderSide, entry: f64, take_profit: f64, stop_loss: f64) -> (f64, f64) {
    let mut tp = take_profit;
    let mut sl = stop_loss;

    if let Some(meta) = &self.symbol_meta {
        if let Some(min_tp) = meta.min_distance_price(entry, meta.tp_distance) {
            match side {
                OrderSide::Buy => {
                    let target = entry + min_tp;
                    if tp < target { tp = target; }  // ✅ Enforce min distance
                }
                OrderSide::Sell => {
                    let target = entry - min_tp;
                    if tp > target { tp = target; }  // ✅ Enforce min distance
                }
            }
        }

        if let Some(min_sl) = meta.min_distance_price(entry, meta.sl_distance) {
            match side {
                OrderSide::Buy => {
                    let target = entry - min_sl;
                    if sl > target { sl = target; }  // ✅ Enforce min distance
                }
                OrderSide::Sell => {
                    let target = entry + min_sl;
                    if sl < target { sl = target; }  // ✅ Enforce min distance
                }
            }
        }
    }

    (tp, sl)
}
```

**✅ Correct:**
- Enforces broker minimum distance requirements
- Handles BUY vs SELL direction correctly
- Adjusts prices to meet minimums before calculating relative distance

---

## Safeguards Already in Place

1. **Finite Number Validation**: `is_finite()` prevents NaN/Infinity
2. **Zero Distance Rejection**: Returns `None` if distance is 0
3. **Broker Minimum Distance**: `normalize_tp_sl()` enforces broker limits
4. **Direction-Aware Calculation**: `.abs()` handles BUY/SELL correctly
5. **Optional Return Type**: `Option<i64>` allows graceful handling of invalid inputs
6. **Comment Documentation**: Code explains why relative distances are used

---

## Suggested Additional Safeguards

### 1. **Maximum Distance Validation**
Prevent unrealistic SL/TP that could tie up margin or fail broker validation.

```rust
fn relative_distance(&self, entry: f64, target: f64) -> Option<i64> {
    const MAX_DISTANCE_PIPS: i64 = 1_000_000; // 10.00 price points (100,000 = 1 point)
    
    if !entry.is_finite() || !target.is_finite() {
        return None;
    }
    let diff = (entry - target).abs();
    if diff <= 0.0 {
        return None;
    }
    let distance = (diff * 100000.0).round() as i64;
    
    // ⚠️ NEW: Reject distances > 10 points (likely config error)
    if distance > MAX_DISTANCE_PIPS {
        warn!("SL/TP distance too large: {} pips (entry={}, target={})", distance, entry, target);
        return None;
    }
    
    Some(distance)
}
```

### 2. **Direction Consistency Check**
Ensure TP is in profit direction and SL is in loss direction.

```rust
fn validate_tp_sl_direction(
    &self,
    side: OrderSide,
    entry: f64,
    tp: f64,
    sl: f64,
) -> Result<()> {
    match side {
        OrderSide::Buy => {
            if tp <= entry {
                return Err(anyhow!("Invalid BUY order: TP ({}) must be > entry ({})", tp, entry));
            }
            if sl >= entry {
                return Err(anyhow!("Invalid BUY order: SL ({}) must be < entry ({})", sl, entry));
            }
        }
        OrderSide::Sell => {
            if tp >= entry {
                return Err(anyhow!("Invalid SELL order: TP ({}) must be < entry ({})", tp, entry));
            }
            if sl <= entry {
                return Err(anyhow!("Invalid SELL order: SL ({}) must be > entry ({})", sl, entry));
            }
        }
    }
    Ok(())
}
```

### 3. **Relative Distance Unit Test**
Add explicit tests for edge cases.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_distance_buy_order() {
        let bot = TradingBot::new(/* ... */);
        let entry = 4200.0;
        let tp = 4300.0;  // +100 points
        let sl = 4100.0;  // -100 points
        
        assert_eq!(bot.relative_distance(entry, tp), Some(10_000_000)); // 100 * 100000
        assert_eq!(bot.relative_distance(entry, sl), Some(10_000_000));
    }

    #[test]
    fn test_relative_distance_rejects_invalid() {
        let bot = TradingBot::new(/* ... */);
        assert_eq!(bot.relative_distance(4200.0, f64::NAN), None);
        assert_eq!(bot.relative_distance(4200.0, f64::INFINITY), None);
        assert_eq!(bot.relative_distance(4200.0, 4200.0), None); // Zero distance
    }
}
```

### 4. **Logging Enhancement**
Add trace-level logging to debug SL/TP calculation in production.

```rust
fn relative_distance(&self, entry: f64, target: f64) -> Option<i64> {
    if !entry.is_finite() || !target.is_finite() {
        warn!("Invalid SL/TP prices: entry={}, target={}", entry, target);
        return None;
    }
    let diff = (entry - target).abs();
    if diff <= 0.0 {
        warn!("Zero SL/TP distance: entry={}, target={}", entry, target);
        return None;
    }
    let distance = (diff * 100000.0).round() as i64;
    debug!("Relative distance calculated: entry={}, target={}, distance={} pips", entry, target, distance);
    Some(distance)
}
```

---

## Conclusion

**Status**: ✅ **PRODUCTION READY**

The current implementation correctly handles MARKET order SL/TP using relative distances as required by cTrader API. All critical safeguards are in place.

**Optional Enhancements** (non-blocking):
1. Add maximum distance validation (prevent config errors)
2. Add direction consistency check (catch logic errors)
3. Add unit tests for `relative_distance()`
4. Enhance logging for production debugging

**Risk Level**: **LOW** - Current implementation is safe and compliant.
