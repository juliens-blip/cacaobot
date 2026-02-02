# TASK-PROD-002 TLS Verification Tests

Status: DONE

## Changes
- Added TLS verification tests for cTrader LIVE/DEMO endpoints in `tests/tls_verification_test.rs`.
- Added dev-dependencies for `tokio-rustls`, `rustls`, and `rustls-native-certs`.

## Manual test instructions
These tests require outbound network access to cTrader servers and are skipped by default.

Run:
- CTRADER_TLS_TESTS=1 cargo test --test tls_verification_test -- --nocapture

Notes:
- Tests connect to `live.ctraderapi.com:5035` and `demo.ctraderapi.com:5035`.
- If the environment has no LIVE access, the tests will skip; enable by setting the env var above.

## Risks / follow-up
- Network-restricted environments will skip TLS tests; run manually in a network-enabled environment.

# TASK-OPT-003 Sentiment Cache Implementation

Status: DONE

## Changes
- Added `src/modules/scraper/sentiment_cache.rs` with TTL + LRU cache (max 100, 5 min default).
- Integrated cache into `src/modules/scraper/perplexity.rs` (check cache before API, store after).
- Exported cache module in `src/modules/scraper/mod.rs`.
- Added unit tests for cache hit/miss/expiry/LRU eviction.

## Manual test instructions
Run unit tests:
- cargo test --test tls_verification_test
- cargo test --lib

## Risks / follow-up
- Cache currently stores score-only (per spec); cached confidence/raw text are not preserved.
