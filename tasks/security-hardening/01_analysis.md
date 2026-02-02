# Analyse: security-hardening

## 📋 Contexte
**Date:** 2026-01-26
**Demande initiale:** Security hardening end-to-end
**Objectif:** Réduire les risques (secrets/logs, TLS, rate limiting, env validation)

## 🔍 État Actuel de la Codebase

### Fichiers Concernés
| Fichier | Type | Rôle | Lignes |
| --- | --- | --- | --- |
| src/config.rs | Config | Chargement .env + validation minimale | L150-260 |
| src/bot.rs | Runtime | Logs + persistence + reconnect | L1-680 |
| src/modules/trading/ctrader.rs | Client | Auth demo + TCP | L170-260 |
| src/modules/scraper/perplexity.rs | HTTP | Appels API, logs taille | L140-210 |
| src/modules/scraper/twitter.rs | HTTP | Scraping guest mode | L1-120 |
| .env.example | Doc | Variables env | L1-180 |

### Architecture Actuelle
```
Config.from_env -> TradingBot.run -> cTrader TCP
Perplexity HTTP -> Twitter fallback
No centralized secret redaction
```

### Code Snippets Clés
#### src/config.rs
```rust
get_env("CTRADER_CLIENT_ID")?;
get_env("CTRADER_CLIENT_SECRET")?;
```

#### src/modules/trading/ctrader.rs
```rust
// For demo accounts, we use the client_id as access token
```

## 📚 Documentation Externe (Context7)
- ⚠️ Context7 indisponible dans cet environnement (outils MCP non configurés).

## 🔗 Dépendances

### Internes
- Config::validate() n’applique que des checks simples
- Logs via tracing sans redaction

### Externes
- reqwest, tokio, rustls

## ⚠️ Points d'Attention
- Secrets potentiellement loggués (env + errors)
- OAuth prod non branché côté cTrader (demo access_token)
- Pas de rate limiting global (Perplexity/Twitter)

## 💡 Opportunités Identifiées
- Ajout redaction de secrets dans logs
- Validation config étendue (account_id numérique, ports, etc.)
- Garde-fou DRY_RUN + live

## 📊 Résumé Exécutif
- Security hardening requis autour des secrets/logging/validation env et OAuth prod.
