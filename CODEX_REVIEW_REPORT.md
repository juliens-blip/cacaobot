# Code Review Report - Palm Oil Bot
**Reviewer**: Codex
**Date**: 2026-01-20
**Status**: FAIL

## Summary
Overall structure is coherent and the trading, scraping, and monitoring modules are cleanly separated, but there are several correctness gaps that prevent the bot from functioning reliably in production. The most serious issues are in the cTrader client message handling and order/position ID handling, which can block price updates and break position closures. Error handling is generally good but multiple `unwrap()`/`expect()` calls remain in non-test code. Test count is solid (73 tests), yet documentation and runtime behavior diverge in key places (trend filter, circuit breaker behavior, README test counts).

## Issues Found

### 🔴 Critical (Must Fix)
- [ ] [src/modules/trading/ctrader.rs:307] Spot price updates are only processed inside `wait_for_message`; there is no continuous reader task after subscription, so prices never update and `get_price()` fails in steady-state.
- [ ] [src/modules/trading/ctrader.rs:201] `place_order()` returns `order_id` from the execution event, but `main` treats it as a position ID when closing; this will fail or close the wrong position.

### 🟠 Major (Should Fix)
- [ ] [src/main.rs:121] The EMA/trend filter never updates because `TradingStrategy::update_price()` is never called; trend filter is effectively disabled despite being enabled by default.
- [ ] [src/main.rs:158] Metrics never close trades; only `record_realized_pnl()` is called, so open positions stay open and win-rate/P&L stats are wrong.
- [ ] [src/modules/trading/ctrader.rs:116] Demo shortcut uses `client_id` as `access_token`; live auth flow and refresh token handling are missing.
- [ ] [src/modules/monitoring/metrics.rs:321] Multiple `unwrap()`/`expect()` calls in production code (mutex locks, regex/client builders, protobuf encodes) can panic; violates the “no unwrap in prod” requirement.
- [ ] [Cargo.toml] Unused dependencies increase attack surface and build time (e.g., `tokio-tungstenite`, `tokio-native-tls`, `native-tls`, `select`, `ta`, `config`, `async-channel`, `url`, `prost-types`).

### 🟡 Minor (Nice to Have)
- [ ] [README.md:4] README reports 62 tests, but the repo currently has 73 tests; also the README mentions a 15-minute cooldown and forced exits not present in code.
- [ ] [src/modules/monitoring/dashboard.rs:121] Dashboard shows a hardcoded account ID (“10092792 (DEMO)”) instead of config-driven data.
- [ ] [src/config.rs:120] `Config::validate()` does not check `account_id`, `endpoint`, or `model` for emptiness; minor input validation gap.
- [ ] [src/modules/monitoring/metrics.rs:305] `std::sync::Mutex` is used in async paths; consider `tokio::sync::Mutex/RwLock` to avoid blocking the runtime.
- [ ] [src/main.rs:22] FCPO symbol ID hardcoded as `1` with TODO; should be resolved via `ProtoOASymbolsListReq` at startup.

## Code Quality Metrics

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Error Handling | ~82% | 95% | ❌ |
| Test Coverage | 73 tests | >50 | ✅ |
| Documentation | ~70% | 80% | ❌ |
| Security Issues | 0 | 0 | ✅ |

## Recommendations

1. Implement a dedicated reader task for the cTrader stream that continuously processes incoming messages (spot events, execution events) and updates shared state.
2. Return and store `position_id` from execution events (and track order/position mapping) to ensure closes target the correct position.
3. Remove remaining `unwrap()`/`expect()` from non-test code and propagate errors with context.
4. Update the main loop to call `strategy.update_price()` and close metrics trades using actual exit prices.
5. Prune unused dependencies and align README/CLAUDE documentation with runtime behavior.

## Files Reviewed
- [x] src/main.rs
- [x] src/lib.rs
- [x] src/config.rs
- [x] src/error.rs
- [x] src/modules/scraper/perplexity.rs
- [x] src/modules/scraper/twitter.rs
- [x] src/modules/scraper/sentiment.rs
- [x] src/modules/scraper/mod.rs
- [x] src/modules/trading/ctrader.rs
- [x] src/modules/trading/protobuf.rs
- [x] src/modules/trading/indicators.rs
- [x] src/modules/trading/orders.rs
- [x] src/modules/trading/strategy.rs
- [x] src/modules/trading/mod.rs
- [x] src/modules/monitoring/metrics.rs
- [x] src/modules/monitoring/dashboard.rs
- [x] src/modules/monitoring/mod.rs
- [x] src/modules/utils/helpers.rs
- [x] src/modules/utils/mod.rs
- [x] src/bin/test_connection.rs
- [x] src/bin/backtest.rs
- [x] tests/integration_test.rs

## Next Steps
Fix the two critical issues (stream reader + position ID handling), then address major error-handling and metrics/EMA integration gaps before considering a live deployment.
