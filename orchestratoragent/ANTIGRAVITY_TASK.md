# ANTIGRAVITY TASK - Palm Oil Bot

**Date**: 2026-01-21 18:36
**Priority**: HAUTE
**Status**: ASSIGNED

## Task: TASK-ANTIGRAVITY-001 - Sentiment Integration

Intégrer le système de sentiment analysis dans la génération de signaux de trading.

### Objectif

Le bot doit utiliser le sentiment en temps réel (Perplexity API) pour confirmer les signaux RSI.

### Fichiers à modifier

1. **src/modules/scraper/sentiment.rs** (déjà existe)
2. **src/bot.rs** (créé par Codex - attendre)
3. **src/modules/trading/strategy.rs** (update)

### Implémentation

#### 1. Créer fonction fetch_sentiment() dans bot.rs

```rust
async fn fetch_current_sentiment(&self) -> Result<i32> {
    let perplexity = PerplexityClient::new(self.config.perplexity_api_key.clone());
    
    let query = format!(
        "Latest palm oil (FCPO) market sentiment. News, prices, supply/demand. Current date: {}",
        Utc::now().format("%Y-%m-%d")
    );
    
    let response = perplexity.query(&query).await?;
    let analyzer = SentimentAnalyzer::new();
    
    Ok(analyzer.analyze(&response))
}
```

#### 2. Intégrer dans process_signal()

```rust
async fn process_signal(&mut self, candle: &Candle) -> Result<()> {
    // Calculer RSI
    let rsi = self.rsi_calculator.current_rsi().unwrap_or(50.0);
    
    // Fetch sentiment temps réel
    let sentiment = self.fetch_current_sentiment().await?;
    
    // Générer signal combiné
    let signal = self.strategy.generate_signal(rsi, sentiment);
    
    match signal {
        Signal::Buy => self.execute_buy().await?,
        Signal::Sell => self.execute_sell().await?,
        Signal::Hold => {}
    }
    
    Ok(())
}
```

#### 3. Ajouter cache pour éviter trop d'API calls

```rust
struct SentimentCache {
    value: i32,
    timestamp: DateTime<Utc>,
    ttl: Duration,
}

impl SentimentCache {
    fn is_valid(&self) -> bool {
        Utc::now() - self.timestamp < self.ttl
    }
}
```

**Cache refresh**: 5 minutes (éviter rate limits Perplexity)

### Tests

Créer `tests/sentiment_integration_test.rs`:
- Test cache TTL
- Test fallback si API échoue
- Test signal generation avec sentiment

### Acceptance Criteria

- [x] fetch_current_sentiment() implémenté
- [x] Cache 5min pour éviter rate limits
- [x] Sentiment intégré dans generate_signal()
- [x] Fallback gracieux si API fail (utiliser sentiment=0 neutre)
- [x] Logs montrant sentiment score

### Notes

- Perplexity API key dans `.env`: `PERPLEXITY_API_KEY`
- Rate limit: 50 req/min gratuit
- Avec cache 5min: ~288 req/jour (OK)

---
**Assigned by**: AMP Orchestrator
**ETA**: 15 minutes
**Next**: Attendre bot.rs de Codex, puis intégrer
