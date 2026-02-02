# Plan d'Implémentation: Wire Rate Limiters

## 📋 Informations
**Date:** 2026-01-28 23:06 CET
**Basé sur:** 01_analysis.md
**Approche:** Intégration minimale avec Arc<ApiRateLimiter> dans struct fields

## 🎯 Objectif Final
Intégrer ApiRateLimiter dans PerplexityClient et TwitterScraper pour respecter les quotas API et éviter les bans.

## 📊 Gap Analysis
| État Actuel | État Cible | Action Requise |
|-------------|------------|----------------|
| PerplexityClient sans rate limiter | PerplexityClient avec rate limiter | Ajouter field + wait_for_rate_limit() |
| TwitterScraper sans rate limiter | TwitterScraper avec rate limiter | Ajouter field + wait_for_rate_limit() |
| Pas de détection HTTP 429 | Détection 429 + backoff | Vérifier status code + record_failure() |
| Constructeurs sans rate limiter | Constructeurs avec rate limiter | Passer ApiRateLimiter en paramètre |

## 🏗️ Architecture Proposée

```
PerplexityClient
  ├── api_key: String
  ├── client: reqwest::Client
  └── rate_limiter: Arc<ApiRateLimiter>  ← NOUVEAU
        │
        ├─ wait_for_rate_limit() avant HTTP POST
        ├─ record_success() si status 200-299
        └─ record_failure() si status 429

TwitterScraper
  ├── client: reqwest::Client
  └── rate_limiter: Arc<ApiRateLimiter>  ← NOUVEAU
        │
        ├─ wait_for_rate_limit() avant HTTP GET
        ├─ record_success() si status 200-299
        └─ record_failure() si status 429 ou timeout
```

## 📝 Checklist Technique (Step-by-Step)

### Phase 1: Modification PerplexityClient

- [ ] **1.1** - Ajouter import dans `src/modules/scraper/perplexity.rs`
  - Action: Ajouter `use crate::modules::security::ApiRateLimiter;` et `use std::sync::Arc;`
  - Ligne: En haut du fichier (après existing imports)
  - Validation: cargo check passe

- [ ] **1.2** - Ajouter field `rate_limiter` dans struct `PerplexityClient`
  - Code pattern:
    ```rust
    pub struct PerplexityClient {
        api_key: String,
        client: reqwest::Client,
        rate_limiter: Arc<ApiRateLimiter>,  // NOUVEAU
    }
    ```
  - Validation: Struct a 3 fields

- [ ] **1.3** - Modifier constructeur `new()` de `PerplexityClient`
  - Signature: `pub fn new(api_key: String, rate_limiter: Arc<ApiRateLimiter>) -> Self`
  - Logique:
    ```rust
    Self {
        api_key,
        client: reqwest::Client::new(),
        rate_limiter,
    }
    ```
  - Validation: Constructeur accepte 2 paramètres

- [ ] **1.4** - Intégrer rate limiter dans `analyze_sentiment()`
  - Position: **AVANT** `self.client.post(...).send().await`
  - Code:
    ```rust
    // Wait for rate limit before making request
    self.rate_limiter.wait_for_rate_limit().await;
    
    let response = self.client.post(&self.endpoint)...;
    
    // Record success/failure
    match response.status().as_u16() {
        429 => {
            tracing::warn!("Perplexity rate limit hit (429)");
            self.rate_limiter.record_failure();
            return Err(anyhow::anyhow!("Rate limit exceeded"));
        }
        200..=299 => {
            self.rate_limiter.record_success();
        }
        _ => {
            tracing::warn!("Perplexity API error: {}", response.status());
        }
    }
    ```
  - Validation: Function appelle wait_for_rate_limit() et record_*()

### Phase 2: Modification TwitterScraper

- [ ] **2.1** - Ajouter import dans `src/modules/scraper/twitter.rs`
  - Action: Ajouter `use crate::modules::security::ApiRateLimiter;` et `use std::sync::Arc;`
  - Validation: cargo check passe

- [ ] **2.2** - Ajouter field `rate_limiter` dans struct `TwitterScraper`
  - Code pattern:
    ```rust
    pub struct TwitterScraper {
        client: reqwest::Client,
        rate_limiter: Arc<ApiRateLimiter>,  // NOUVEAU
    }
    ```
  - Validation: Struct a 2 fields

- [ ] **2.3** - Modifier constructeur `new()` de `TwitterScraper`
  - Signature: `pub fn new(rate_limiter: Arc<ApiRateLimiter>) -> Self`
  - Logique:
    ```rust
    Self {
        client: reqwest::Client::new(),
        rate_limiter,
    }
    ```
  - Validation: Constructeur accepte 1 paramètre

- [ ] **2.4** - Intégrer rate limiter dans `scrape()`
  - Position: **AVANT** `self.client.get(...).send().await`
  - Code:
    ```rust
    // Wait for rate limit before making request
    self.rate_limiter.wait_for_rate_limit().await;
    
    let response = self.client.get(&url).send().await?;
    
    // Record success (Twitter scraping rarely returns 429, more likely 403 or timeout)
    if response.status().is_success() {
        self.rate_limiter.record_success();
    } else {
        tracing::warn!("Twitter scrape failed: {}", response.status());
        self.rate_limiter.record_failure();
    }
    ```
  - Validation: Function appelle wait_for_rate_limit() et record_*()

### Phase 3: Mettre à Jour les Appels (bot.rs ou scraper/mod.rs)

- [ ] **3.1** - Trouver où PerplexityClient est créé
  - Recherche: `PerplexityClient::new(`
  - Fichier probable: `src/bot.rs` ou `src/modules/scraper/mod.rs`
  - Action: Lire le fichier pour localiser instanciation

- [ ] **3.2** - Créer rate limiter pour Perplexity et passer au constructeur
  - Code:
    ```rust
    use crate::modules::security::ApiRateLimiter;
    use std::sync::Arc;
    
    let perplexity_rate_limiter = Arc::new(ApiRateLimiter::for_perplexity());
    let perplexity_client = PerplexityClient::new(api_key, perplexity_rate_limiter);
    ```
  - Validation: cargo check passe

- [ ] **3.3** - Créer rate limiter pour Twitter et passer au constructeur
  - Code:
    ```rust
    let twitter_rate_limiter = Arc::new(ApiRateLimiter::for_twitter());
    let twitter_scraper = TwitterScraper::new(twitter_rate_limiter);
    ```
  - Validation: cargo check passe

### Phase 4: Tests et Validation

- [ ] **4.1** - Lancer tests unitaires existants
  - Commande: `cargo test --lib`
  - Résultat attendu: 221+ tests PASSING (aucun test cassé)
  - Si FAILED: Déboguer avec méthode Ralph

- [ ] **4.2** - Test manuel: Offline dry-run
  - Commande: `cargo run`
  - Vérifier logs: Chercher "Perplexity rate limit" ou "Twitter rate limit" dans output
  - Résultat attendu: Bot démarre sans crash

- [ ] **4.3** - Ajouter tests unitaires pour rate limiting (optionnel si temps)
  - Fichier: `tests/rate_limiting_integration_test.rs`
  - Tests:
    - Perplexity rate limiter permet 60 requêtes en 60s
    - Twitter rate limiter bloque après 10 requêtes en 60s
  - Validation: cargo test passe

## ⚠️ Risques et Mitigations

| Risque | Impact | Mitigation |
|--------|--------|------------|
| Breaking change dans constructeurs | Build fail | Méthode Ralph détectera, on fixera |
| Tests existants cassés | CI fail | Ralph cycle: test → debug → fix |
| Rate limiter trop strict | Bot lent en offline | Config déjà optimale (60/min Perplexity, 10/min Twitter) |
| HTTP 429 non détecté | Ban API quand même | Code handle status 429 explicitement |

## 🎯 Critères de Validation

1. ✅ cargo check: PASS
2. ✅ cargo test --lib: 221+ tests PASSING
3. ✅ cargo run: Bot démarre en offline mode
4. ✅ Logs montrent "wait_for_rate_limit()" appelé avant API requests
5. ✅ Aucun warning de compilation lié aux structs modifiés

## 📊 Effort Estimé

- **Temps total**: 15-20 minutes
- **Complexité**: MOYENNE (modifications structurelles simples mais impact sur bot.rs)
- **Agent recommandé**: Claude (orchestrateur) ou Codex (backend-architect skill)

---

**Plan validé. Prêt pour /implement.**
