# Analyse: immediate-test-trade-auth-fail

## 📋 Contexte
**Date:** 2026-02-04
**Demande initiale:** Forcer un trade immédiat (BUY+SELL) en mode test et corriger les erreurs d'auth/reconnexion.
**Objectif:** Avoir un mode de test immédiat stable + éviter les crashs `ALREADY_LOGGED_IN` / `Timeout waiting for response`.

## 🔍 État Actuel de la Codebase

### Fichiers Concernés
| Fichier | Type | Rôle |
|---|---|---|
| src/bot.rs | Core | Boucle principale, exécution trades, helpers de retry |
| src/modules/trading/ctrader.rs | Core | Client cTrader (auth, reconnect, send/receive) |
| src/modules/utils/helpers.rs | Utils | retry_with_backoff + logs |

### Architecture Actuelle
- `TradingBot::run()` orchestre la connexion, authentification, subscribe, boucle ticks.
- `CTraderClient::authenticate()` fait AppAuth + AccountAuth.
- `retry_with_backoff()` relance connect/auth sur erreurs réseau/timeout.

### Erreurs observées
- Compilation: méthodes `run_immediate_test_trades` / `close_all_positions` hors `impl TradingBot`.
- Runtime: `ALREADY_LOGGED_IN` renvoyé pendant auth après reconnect.
- Runtime: timeouts de réponse (probablement lors d’auth/réauth concurrente).

## 🔗 Dépendances
- Internes: `TradingBot` → `CTraderClient` → `wait_for_message()`
- Externes: cTrader Open API (Protobuf)

## ⚠️ Points d'Attention
- Exe verrouillé par process actif → `Access denied` au build.
- `ALREADY_LOGGED_IN` doit être traité comme non-fatal pendant AppAuth.

## 📊 Résumé Exécutif
- Le mode test immédiat doit être dans `impl TradingBot`.
- Auth doit ignorer `ALREADY_LOGGED_IN` pour éviter crash.
- Build échoue si process encore actif.
