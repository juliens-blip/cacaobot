# 🚀 DEPLOYMENT CHECKLIST - Palm Oil Bot

**Status**: ✅ READY FOR DEPLOYMENT  
**Date**: 2026-01-22 12:20 CET  
**Version**: 0.1.0

---

## ✅ PRE-DEPLOYMENT CHECKS

### Code Quality
- [✅] Compilation: 0 errors
- [✅] Tests: 190/190 passed (100%)
- [✅] Clippy: 25 warnings (non-bloquants)
- [✅] Backtest: +1.28% P&L, Profit Factor 1.31

### Architecture
- [✅] Risk Management: Circuit breakers implemented
- [✅] Position Management: Persistence + reconciliation
- [✅] Risk Metrics: Sharpe, Sortino, VaR, Max DD
- [✅] Integration Tests: Full stack coverage

### Documentation
- [✅] README.md: Complete
- [✅] .env.example: All variables documented
- [✅] CLAUDE.md: Project instructions
- [✅] RALPH_REPORT_FINAL.md: Test results

---

## 📝 ENVIRONMENT VARIABLES

### Required (Production)

```bash
# cTrader API (Production)
CTRADER_CLIENT_ID=your_production_client_id
CTRADER_CLIENT_SECRET=your_production_client_secret
CTRADER_ACCOUNT_ID=your_production_account_id
CTRADER_SERVER=live.ctraderapi.com:5035  # LIVE SERVER

# Perplexity API
PERPLEXITY_API_KEY=your_perplexity_api_key

# Bot Configuration
SYMBOL=FCPO
MAX_POSITIONS=1
RISK_PER_TRADE=0.02
TAKE_PROFIT_PERCENT=2.0
STOP_LOSS_PERCENT=1.5
DRY_RUN=false  # ⚠️ SET TO false FOR LIVE

# Risk Limits
MAX_DAILY_LOSS=-500.0
MAX_CONSECUTIVE_LOSSES=3
VOLATILITY_THRESHOLD=5.0

# Logging
RUST_LOG=info
```

### Optional (Advanced)

```bash
# Trading Strategy
RSI_PERIOD=14
RSI_OVERSOLD=30
RSI_OVERBOUGHT=70
SENTIMENT_THRESHOLD=30

# Monitoring
DASHBOARD_ENABLED=true
METRICS_PORT=9090

# Twitter Fallback (Backup sentiment)
TWITTER_ENABLED=false
```

---

## 🐳 DOCKER DEPLOYMENT

### 1. Build Image

```bash
cd /home/julien/Documents/palm-oil-bot
docker build -t palm-oil-bot:0.1.0 .
```

**Expected output**:
```
[+] Building 180.3s (15/15) FINISHED
=> => naming to docker.io/library/palm-oil-bot:0.1.0
```

### 2. Test Locally

```bash
# Create .env file
cp .env.example .env
# Edit with production credentials
nano .env

# Run container
docker run --env-file .env palm-oil-bot:0.1.0
```

### 3. Verify Container Health

```bash
# Check logs
docker logs <container_id>

# Expected logs:
# ✅ "Connected to cTrader"
# ✅ "Subscribed to FCPO price feed"
# ✅ "Bot initialized successfully"
```

---

## 🚂 RAILWAY DEPLOYMENT

### 1. Install Railway CLI

```bash
npm install -g @railway/cli
railway login
```

### 2. Initialize Project

```bash
cd /home/julien/Documents/palm-oil-bot
railway init
```

### 3. Set Environment Variables

```bash
# Production cTrader
railway variables set CTRADER_CLIENT_ID=xxx
railway variables set CTRADER_CLIENT_SECRET=xxx
railway variables set CTRADER_ACCOUNT_ID=xxx
railway variables set CTRADER_SERVER=live.ctraderapi.com:5035

# Perplexity
railway variables set PERPLEXITY_API_KEY=xxx

# Risk Management
railway variables set MAX_DAILY_LOSS=-500.0
railway variables set MAX_CONSECUTIVE_LOSSES=3
railway variables set DRY_RUN=false

# Logging
railway variables set RUST_LOG=info
```

### 4. Deploy

```bash
railway up
```

**Expected output**:
```
✓ Build successful
✓ Deployment live at https://palm-oil-bot-production.up.railway.app
```

### 5. Monitor Deployment

```bash
# View logs
railway logs

# Check status
railway status
```

---

## ⚠️ PRODUCTION WARNINGS

### CRITICAL - Before Going Live

1. **Test on DEMO First**
   ```bash
   DRY_RUN=true  # Test without real orders
   CTRADER_SERVER=demo.ctraderapi.com:5035
   ```
   Run for 24-48h to validate behavior.

2. **Start with Small Capital**
   - Max $1000 initial balance
   - Monitor closely first week
   - Scale gradually if profitable

3. **Circuit Breakers Mandatory**
   ```bash
   MAX_DAILY_LOSS=-500.0  # -50% of $1000 balance
   MAX_CONSECUTIVE_LOSSES=3
   VOLATILITY_THRESHOLD=5.0
   ```

4. **OAuth Authentication**
   - Current implementation uses demo auth
   - Production requires proper OAuth flow
   - **Action**: Implement OAuth before live trading

5. **TLS Verification**
   - Verify cTrader requires TLS
   - **Action**: Test with production server

---

## 📊 MONITORING CHECKLIST

### Daily Checks

- [ ] Review P&L (target: +2-3% daily)
- [ ] Check circuit breaker triggers
- [ ] Verify no missed signals
- [ ] Monitor max drawdown (limit: -5%)

### Weekly Checks

- [ ] Win rate trend (target: >50%)
- [ ] Profit factor (target: >1.5)
- [ ] Review strategy parameters (RSI thresholds)
- [ ] Check API quota (Perplexity)

### Monthly Checks

- [ ] Backtest with latest data
- [ ] Review risk metrics (Sharpe, Sortino)
- [ ] Update dependencies (`cargo update`)
- [ ] Rotate API keys

---

## 🔧 TROUBLESHOOTING

### Connection Issues

```bash
# Test cTrader connection
cargo run --bin test-connection

# Expected: "✅ Authentication successful"
```

### High Memory Usage

```bash
# Check container stats
docker stats

# If > 500MB, reduce candle history size
```

### Missing Signals

```bash
# Verify Perplexity API
curl -H "Authorization: Bearer $PERPLEXITY_API_KEY" \
  https://api.perplexity.ai/chat/completions

# Expected: 200 OK
```

---

## 🎯 SUCCESS CRITERIA

### Day 1
- ✅ Bot connects to cTrader
- ✅ Receives FCPO price feed
- ✅ Generates RSI signals
- ✅ Fetches sentiment (Perplexity)
- ✅ No crashes

### Week 1
- ✅ P&L > 0
- ✅ Win rate > 40%
- ✅ Max drawdown < 10%
- ✅ Circuit breakers tested (no trigger)

### Month 1
- ✅ Cumulative P&L > +5%
- ✅ Win rate > 50%
- ✅ Sharpe Ratio > 1.0
- ✅ Max drawdown < 8%

---

## 📞 SUPPORT

**Issues**: https://github.com/your-repo/palm-oil-bot/issues  
**Documentation**: /README.md, /CLAUDE.md  
**Test Reports**: /RALPH_REPORT_FINAL.md

---

## ✅ FINAL GO/NO-GO

| Critère | Status | Notes |
|---------|--------|-------|
| Compilation | ✅ GO | 0 errors |
| Tests | ✅ GO | 190/190 passed |
| Backtest | ⚠️ CAUTION | +1.28%, needs tuning |
| Risk Management | ✅ GO | Circuit breakers OK |
| OAuth Production | ❌ NO-GO | Demo auth only |
| TLS Verification | ⚠️ VERIFY | Needs testing |

**Recommendation**: **DEPLOY TO DEMO FIRST**

1. Deploy to Railway with DRY_RUN=true
2. Monitor for 48h
3. Implement OAuth for production
4. Verify TLS requirement
5. Test with $100 live (if OAuth OK)
6. Scale to $1000 after 1 week success

---

**Prepared by**: AMP Orchestrator  
**Approved by**: [PENDING]  
**Deployment Date**: [TBD]
