# TASK-OPT-003: Sentiment Cache Implementation

**Assigné à**: Codex  
**Priorité**: MOYENNE (après TLS tests)  
**ETA**: 20min

## Objectif

Implémenter cache pour Perplexity API avec TTL pour éviter rate limits.

## Fichier à Créer

`src/modules/scraper/sentiment_cache.rs`

## Fonctionnalités Requises

### 1. Structure CachedSentiment
```rust
pub struct CachedSentiment {
    pub score: i32,
    pub timestamp: DateTime<Utc>,
    pub query: String,
}
```

### 2. Cache In-Memory
- HashMap<String, CachedSentiment>
- TTL: 5 minutes
- Max size: 100 entries
- Thread-safe (Arc<Mutex<>> ou Arc<RwLock<>>)

### 3. API Publique
```rust
pub struct SentimentCache {
    // ...
}

impl SentimentCache {
    pub fn new() -> Self;
    pub fn get(&self, query: &str) -> Option<i32>;
    pub fn set(&self, query: &str, score: i32);
    pub fn clear_expired(&mut self);
}
```

### 4. Integration perplexity.rs
- Check cache avant API call
- Fallback API si cache miss/expired
- Save result in cache after API call

## Tests Requis

1. `test_cache_hit()` - Vérifie cache retourne valeur stockée
2. `test_cache_miss()` - Vérifie None si pas dans cache
3. `test_cache_expiry()` - Vérifie entrées expirées sont ignorées
4. `test_cache_max_size()` - Vérifie LRU eviction si > 100 entrées

## Exemple d'Usage

```rust
let cache = SentimentCache::new();

// First call: cache miss → API call
let score1 = get_sentiment_with_cache(&cache, "palm oil market").await?;

// Second call (< 5min): cache hit → pas d'API call
let score2 = get_sentiment_with_cache(&cache, "palm oil market").await?;

assert_eq!(score1, score2);
```

## Livrables

- `src/modules/scraper/sentiment_cache.rs` complet
- Tests unitaires (4+)
- Documentation inline
- Update `src/modules/scraper/mod.rs` pour exporter

## Rapport

Écris ton avancement dans `orchestratoragent/CODEX_RESPONSE.md`
