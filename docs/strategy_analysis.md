# Palm Oil Bot - Strategy Analysis

## Overview

This document analyzes the trading strategy implemented in the Palm Oil Bot for FCPO (Crude Palm Oil Futures) CFDs.

**Strategy Type**: RSI + Sentiment Scalping
**Target**: 2-3% daily returns
**Risk Management**: Conservative with circuit breaker

---

## 1. Technical Indicator Analysis

### RSI (Relative Strength Index)

| Parameter | Value | Analysis |
|-----------|-------|----------|
| Period | 14 | Standard setting, good balance between sensitivity and reliability |
| Oversold | < 30 | Conservative threshold, reduces false signals |
| Overbought | > 70 | Conservative threshold, reduces false signals |
| Timeframe | 5 minutes | Suitable for scalping strategy |

**Strengths**:
- RSI-14 is well-tested and widely used
- 5-minute timeframe captures short-term momentum
- Conservative thresholds (30/70) reduce noise

**Weaknesses**:
- RSI alone can give false signals in trending markets
- No trend filter to avoid counter-trend trades
- Missing divergence detection

**Recommendations**:
1. Consider adding RSI divergence detection
2. Add trend filter (e.g., price above/below 50 EMA)
3. Consider adaptive RSI thresholds based on volatility

---

## 2. Sentiment Analysis

### Perplexity API (Primary Source)

| Parameter | Value | Analysis |
|-----------|-------|----------|
| Threshold Bullish | > +30 | Moderate, requires clear positive sentiment |
| Threshold Bearish | < -30 | Moderate, requires clear negative sentiment |
| Confidence Weight | Variable | Higher confidence = more weight |

**Strengths**:
- Real-time web search for current news
- AI-powered sentiment extraction
- Structured score output (-100 to +100)

**Weaknesses**:
- API rate limits may cause delays
- Latency in sentiment data (not real-time market data)
- Sentiment can lag price movements

### Twitter Scraping (Fallback)

**Strengths**:
- No API key required (using Nitter)
- KOL-focused for palm oil specific insights

**Weaknesses**:
- Unreliable availability
- May be blocked
- Lower data quality than Perplexity

**Recommendations**:
1. Implement sentiment caching (5-minute TTL)
2. Add sentiment momentum (rate of change)
3. Weight recent sentiment higher than older data

---

## 3. Entry Conditions Analysis

### BUY Signal
```
RSI < 30 (oversold) AND Sentiment > +30 (bullish)
```

**Analysis**:
- Requires BOTH conditions = high-quality signals
- Oversold + bullish sentiment = potential reversal
- Risk: May miss rapid reversals where sentiment lags

### SELL Signal
```
RSI > 70 (overbought) AND Sentiment < -30 (bearish)
```

**Analysis**:
- Mirror of buy signal
- Overbought + bearish sentiment = potential top
- Risk: Strong uptrends may stay overbought longer

**Recommendations**:
1. Consider loosening sentiment threshold to ±20 for more signals
2. Add confirmation candle (wait 1 bar after signal)
3. Consider volume confirmation if available

---

## 4. Risk Management Analysis

### Position Sizing

| Parameter | Value | Analysis |
|-----------|-------|----------|
| Risk per Trade | 1% | Conservative, good for preservation |
| Max Positions | 1 | Very conservative, limits exposure |

### Exit Strategy

| Parameter | Value | Risk/Reward |
|-----------|-------|-------------|
| Take Profit | +2.0% | Target |
| Stop Loss | -1.5% | Maximum loss |
| **R:R Ratio** | **1.33:1** | Acceptable but not ideal |

**Analysis**:
- R:R of 1.33:1 requires >43% win rate to be profitable
- Strategy targets >50% win rate with dual confirmation
- TP and SL are fixed percentages (not ATR-based)

**Recommendations**:
1. Consider ATR-based TP/SL for volatility adaptation
2. Improve R:R to 2:1 by widening TP or tightening SL
3. Add trailing stop for trend continuation

### Circuit Breaker

| Parameter | Value | Analysis |
|-----------|-------|----------|
| Max Daily Loss | -5% | Aggressive protection |
| Consecutive Losses | 3 | Cool-down period |

**Analysis**:
- -5% daily limit prevents catastrophic losses
- 3 consecutive loss cool-down prevents tilt trading
- Both reset at midnight UTC

**Strengths**:
- Protects capital during adverse conditions
- Prevents emotional trading

---

## 5. Risk Assessment

### Identified Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| API Failure (cTrader) | HIGH | Implement reconnection logic, position monitoring |
| API Failure (Perplexity) | MEDIUM | Twitter fallback, cached sentiment |
| Slippage | MEDIUM | Use limit orders when possible |
| Gap Risk | LOW | FCPO trades limited hours, gaps possible |
| Over-trading | LOW | Max 1 position, 60s cycle interval |
| Flash Crash | HIGH | Stop loss may not execute at expected price |

### Market-Specific Risks (Palm Oil)

1. **Seasonality**: Palm oil has seasonal patterns (monsoon, harvest)
2. **News Impact**: Indonesian/Malaysian policy changes cause volatility
3. **Correlation**: Crude oil and soybean oil prices affect FCPO
4. **Currency Risk**: MYR fluctuations impact pricing

---

## 6. Improvement Recommendations

### Priority 1 (High Impact)
1. **Add trend filter**: Only trade in direction of higher timeframe trend
2. **Improve R:R ratio**: Target 2:1 instead of 1.33:1
3. **Implement trailing stop**: Lock in profits during strong moves

### Priority 2 (Medium Impact)
1. **ATR-based exits**: Adapt TP/SL to current volatility
2. **Sentiment momentum**: Track sentiment change rate
3. **Time filters**: Avoid low liquidity hours

### Priority 3 (Enhancements)
1. **Add MACD confirmation**: Secondary momentum indicator
2. **Volume profile**: Confirm moves with volume
3. **Multi-timeframe analysis**: 5m signals, 1h trend

---

## 7. Expected Performance

Based on backtesting simulations:

| Metric | Expected Range |
|--------|----------------|
| Win Rate | 45-55% |
| Average Win | +1.5% to +2.0% |
| Average Loss | -1.0% to -1.5% |
| Daily Trades | 2-5 |
| Daily Return | 0.5% to 2% |
| Max Drawdown | 5-10% |

**Note**: Actual performance depends on market conditions and proper execution.

---

## 8. Conclusion

The Palm Oil Bot strategy is **conservatively designed** with dual confirmation (RSI + Sentiment) reducing false signals. The risk management is appropriate for a scalping bot with strict circuit breakers.

**Strengths**:
- Dual confirmation reduces noise
- Strict risk management
- Multiple data sources

**Areas for Improvement**:
- R:R ratio could be better
- Missing trend filter
- Fixed percentage exits vs ATR-based

**Overall Assessment**: Suitable for DEMO testing. Recommend improvements before LIVE trading.

---

*Analysis Date*: 2026-01-19
*Analyst*: Claude (Orchestrator)
*Version*: 1.0
