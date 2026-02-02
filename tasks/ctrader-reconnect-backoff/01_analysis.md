# Analyse: ctrader-reconnect-backoff

## 📋 Contexte
**Date:** 2026-01-26
**Demande initiale:** Ajouter reconnexion + backoff robuste cTrader
**Objectif:** Reconnexion automatique et retry avec backoff pour connect/auth/send

## 🔍 État Actuel de la Codebase

### Fichiers Concernés
| Fichier | Type | Rôle | Lignes |
| --- | --- | --- | --- |
| src/modules/trading/ctrader.rs | Client | Connexion/auth/heartbeat cTrader | L1-760 |
| src/bot.rs | Runtime | Orchestration connect/auth | L1-520 |
| src/error.rs | Errors | Types d’erreurs cTrader | L1-120 |

### Architecture Actuelle
```
TradingBot.run -> CTraderClient.connect() -> authenticate() -> start background tasks
(no reconnect/backoff, timeout fixed)
```

### Code Snippets Clés
#### src/modules/trading/ctrader.rs
```rust
pub async fn connect(&self) -> Result<()> { /* TCP connect + start_reader */ }
pub async fn authenticate(&self) -> Result<()> { /* app + account auth */ }
```

## 📚 Documentation Externe (Context7)
- ⚠️ Context7 indisponible dans cet environnement (outils MCP non configurés).

## 🔗 Dépendances

### Internes
- bot.rs appelle connect/auth une seule fois

### Externes
- tokio::net::TcpStream, tokio::time::timeout

## ⚠️ Points d'Attention
- Pas de boucle de reconnexion
- Pas de backoff/jitter
- Start_reader + send_message doivent être résilients aux erreurs réseau

## 💡 Opportunités Identifiées
- Introduire un wrapper retry avec backoff exponentiel
- Ajouter état de connexion + re-auth automatique

## 📊 Résumé Exécutif
- Client cTrader se connecte une fois puis échoue définitivement si erreur.
- Aucun mécanisme de retry/backoff ou reconnexion.
