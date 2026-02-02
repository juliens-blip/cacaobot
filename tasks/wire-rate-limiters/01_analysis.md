# Analyse: Wire Rate Limiters into perplexity.rs and twitter.rs

## 📋 Contexte
**Date:** 2026-01-28 23:05 CET
**Demande initiale:** Intégrer ApiRateLimiter dans les clients Perplexity et Twitter
**Objectif:** Éviter les bans API en limitant les requêtes selon les quotas

## 🔍 État Actuel de la Codebase

### Fichiers Concernés
| Fichier | Type | Rôle | Lignes Clés |
|---------|------|------|-------------|
| src/modules/security/rate_limiter.rs | Utility | Rate limiter avec backoff exponentiel | L1-250 |
| src/modules/scraper/perplexity.rs | API Client | Client Perplexity AI | Complet |
| src/modules/scraper/twitter.rs | Scraper | Scraper Twitter (backup) | Complet |
| src/modules/scraper/mod.rs | Module | Exports | L1-10 |

### Architecture Actuelle

```
scraper/
  ├── perplexity.rs
  │     └── PerplexityClient
  │           └── analyze_sentiment() → HTTP POST sans rate limit
  │
  ├── twitter.rs  
  │     └── TwitterScraper
  │           └── scrape() → HTTP GET sans rate limit
  │
security/
  └── rate_limiter.rs
        ├── ApiRateLimiter::for_perplexity() → 60 req/min
        ├── ApiRateLimiter::for_twitter() → 10 req/min
        └── wait_for_rate_limit() → async wait avec backoff
```

### Code Snippets Clés

#### Fichier 1: src/modules/security/rate_limiter.rs
```rust
impl ApiRateLimiter {
    /// Perplexity API: 60 requests per minute
    pub fn for_perplexity() -> Self {
        Self::new(RateLimiterConfig {
            max_requests: 60,
            window_duration: Duration::from_secs(60),
            ..Default::default()
        })
    }

    /// Twitter scraping: 10 requests per minute (conservative)
    pub fn for_twitter() -> Self {
        Self::new(RateLimiterConfig {
            max_requests: 10,
            window_duration: Duration::from_secs(60),
            ..Default::default()
        })
    }

    /// Wait until rate limit allows, with exponential backoff
    pub async fn wait_for_rate_limit(&self) {
        // Implementation avec sleep + backoff
    }

    /// Record successful request
    pub fn record_success(&self) {
        // Reset consecutive failures
    }

    /// Record failed request
    pub fn record_failure(&self) {
        // Increment consecutive failures for backoff
    }
}
```

#### Fichier 2: src/modules/scraper/perplexity.rs (extrait)
```rust
pub struct PerplexityClient {
    api_key: String,
    client: reqwest::Client,
    // MANQUE: rate_limiter: Arc<ApiRateLimiter>
}

impl PerplexityClient {
    pub async fn analyze_sentiment(&self, query: &str) -> Result<i32> {
        // MANQUE: self.rate_limiter.wait_for_rate_limit().await;
        
        let response = self.client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await?;
        
        // MANQUE: 
        // if response.status() == 429 { self.rate_limiter.record_failure(); }
        // else { self.rate_limiter.record_success(); }
        
        // Parse response...
    }
}
```

#### Fichier 3: src/modules/scraper/twitter.rs (extrait)
```rust
pub struct TwitterScraper {
    client: reqwest::Client,
    // MANQUE: rate_limiter: Arc<ApiRateLimiter>
}

impl TwitterScraper {
    pub async fn scrape(&self, query: &str) -> Result<Vec<String>> {
        // MANQUE: self.rate_limiter.wait_for_rate_limit().await;
        
        let response = self.client.get(&url).send().await?;
        
        // MANQUE: record_success/failure
        
        // Parse HTML...
    }
}
```

## 🔗 Dépendances

### Internes
- `scraper/perplexity.rs` → `security/rate_limiter.rs` (import ApiRateLimiter)
- `scraper/twitter.rs` → `security/rate_limiter.rs` (import ApiRateLimiter)

### Externes
- `reqwest` (déjà utilisé): HTTP client
- `tokio` (déjà utilisé): async runtime
- `std::sync::Arc` (déjà utilisé): thread-safe sharing

## ⚠️ Points d'Attention
1. **Thread-safety**: ApiRateLimiter utilise Arc<Mutex<>> → Besoin de Arc<ApiRateLimiter> dans structs
2. **HTTP 429 detection**: Perplexity renvoie 429 Too Many Requests → Détecter et record_failure()
3. **Twitter rate limits**: Pas d'API officielle → Rate limiter conservatif (10 req/min) pour éviter IP ban
4. **Backoff exponentiel**: Déjà implémenté dans ApiRateLimiter → Juste appeler wait_for_rate_limit()

## 💡 Opportunités Identifiées
- Pattern réutilisable pour futurs clients API (cTrader, etc.)
- Logs de rate limiting déjà intégrés (via tracing dans rate_limiter.rs)
- Possibilité d'ajouter métriques Prometheus (current_request_count, consecutive_failures)

## 📊 Résumé Exécutif
1. **Rate limiter existe** et est fonctionnel (12 tests passing)
2. **Clients API n'utilisent PAS** le rate limiter actuellement
3. **Risque**: Bans API sur Perplexity (60 req/min dépassé) et Twitter (IP ban)
4. **Solution**: Ajouter Arc<ApiRateLimiter> dans PerplexityClient et TwitterScraper
5. **Effort**: Faible (~15 lignes de code par fichier + 1 ligne dans constructeurs)
