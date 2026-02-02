# RUNBOOK - Palm Oil Bot Production

**Last updated**: 2026-01-26

## 1) Preconditions

- ✅ All tests pass (`cargo test`)
- ✅ Demo run completed (DRY_RUN=true)
- ✅ OAuth production flow validated
- ✅ TLS validation confirmed against LIVE/DEMO

## 2) Deployment (Railway)

1. Set env vars (see `DEPLOY_CHECKLIST.md`)
2. Deploy: `railway up`
3. Verify logs: `railway logs`

## 3) Deployment (Docker)

1. Build image: `docker build -t palm-oil-bot:0.1.0 .`
2. Run: `docker run --env-file .env palm-oil-bot:0.1.0`
3. Verify: logs contain cTrader connect + auth

## 4) Rollback

- Railway: redeploy previous image tag
- Docker: `docker run --env-file .env palm-oil-bot:<previous_tag>`

## 5) Incident Response

- **Connection loss**: verify cTrader connectivity, check reconnect logs
- **Order rejections**: inspect broker messages + auth tokens
- **High drawdown**: confirm circuit breakers are active
- **Perplexity 429**: verify Twitter fallback

## 6) Monitoring

- Metrics endpoint (if enabled): `/metrics` on `METRICS_HOST:METRICS_PORT`
- Watch: P&L, win rate, drawdown, circuit breaker triggers

## 7) References

- `DEPLOY_CHECKLIST.md` (full checklist)
- `NEXT_STEPS.md` (production recommendations)
