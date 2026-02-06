# Plan d'Implémentation: immediate-test-trade-auth-fail

## 📋 Informations
**Date:** 2026-02-04
**Basé sur:** 01_analysis.md
**Approche:** corrections minimales et sûres + validation par recompilation et test de trade.

## 🎯 Objectif Final
- Build propre
- Mode test immédiat BUY+SELL fonctionnel
- Auth stable après reconnect (pas de crash `ALREADY_LOGGED_IN`)

## ✅ Étapes
1. Corriger la position des méthodes de test (dans `impl TradingBot`).
2. Traiter `ALREADY_LOGGED_IN` comme non-fatal pendant AppAuth.
3. Nettoyer les doublons si présents.
4. Rebuild + run avec `TEST_IMMEDIATE_TRADES=1`.

## 🧪 Validation
- `cargo build --release` passe.
- Log: `TEST_IMMEDIATE_TRADES enabled: placing BUY then SELL`.
- Ordres BUY puis SELL exécutés + fermés.
