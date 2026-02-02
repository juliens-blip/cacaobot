# Next Steps - Production Deployment Recommendations

**Last updated**: 2026-01-26
**Scope**: Palm Oil Bot production readiness

---

## 1) Security Hardening

- **Secrets management**: Move API keys and cTrader credentials to a secret manager (Railway/VM secrets), remove `.env` from runtime, and ensure logs never print secrets.
- **TLS enforcement**: Ensure cTrader connections use TLS (rustls) with verified certificates for both LIVE and DEMO.
- **Credential rotation**: Define rotation cadence for Perplexity and cTrader credentials; add automated reminders.
- **Network boundaries**: Restrict outbound calls to required endpoints (cTrader, Perplexity, Nitter). Use allow-lists.
- **Rate limiting**: Add backoff + jitter for Perplexity and Twitter scraping to avoid bans and reduce costs.

## 2) Monitoring & Alerting

- **Structured logs**: Ensure logs include timestamps, trade IDs, symbol IDs, and request IDs.
- **Metrics export**: Expose Prometheus-style metrics (P&L, win rate, drawdown, circuit breaker triggers, cache hit rate).
- **Health checks**: Add liveness/readiness endpoints or periodic heartbeat logs for process supervision.
- **Alerting**: Configure alerts for:
  - cTrader disconnects / auth failures
  - Order rejections
  - Sudden drawdown or consecutive losses
  - Perplexity rate limits and Twitter fallback usage

## 3) Required Tests Before Production

- **Unit**:
  - Strategy signals (RSI + sentiment boundaries)
  - Risk/circuit breaker thresholds
  - Sentiment cache TTL and eviction
- **Integration**:
  - cTrader demo connection + auth + subscribe flow
  - Perplexity API happy path + 429 handling
  - Twitter fallback path
- **Regression**:
  - Backtest optimizer CSV generation
  - Strategy config parameter bounds
- **Manual**:
  - End-to-end demo run (dry_run=false but no real orders)

## 4) Deployment Checklist

- **Build**: `cargo build --release`
- **Runtime config**: Validate all required env vars are present and parsed correctly.
- **Dry-run gate**: Keep `DRY_RUN=true` for first live smoke test.
- **Rollback plan**: Prepare a one-command rollback (previous image/tag).
- **Post-deploy validation**:
  - Confirm market data feed updates
  - Confirm sentiment pipeline runs
  - Confirm orders are blocked in dry-run

## 5) Production Readiness Gaps

- **Socket concurrency**: Ensure single-reader pattern for cTrader socket to avoid framing corruption.
- **Reconciliation**: Implement position reconciliation at startup and after reconnect.
- **Persistence**: Persist positions and trades to durable storage (SQLite/Postgres) for audit and recovery.
- **Observability**: Add dashboards for latency, order fill rate, and error distribution.

---

## Suggested Priority Order

1. Security hardening (TLS + secrets)
2. cTrader socket correctness + reconciliation
3. Monitoring + alerts
4. Integration tests (demo environment)
5. Production dry-run
