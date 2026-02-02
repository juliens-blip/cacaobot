# Analyse: prometheus-metrics

## 📋 Contexte
**Date:** 2026-01-26
**Demande initiale:** Mettre en place /metrics Prometheus + alerting de base
**Objectif:** Exposer des métriques runtime (P&L, win rate, drawdown, circuit breakers) et préparer l’alerting

## 🔍 État Actuel de la Codebase

### Fichiers Concernés
| Fichier | Type | Rôle | Lignes |
| --- | --- | --- | --- |
| src/modules/monitoring/metrics.rs | Module | Agrégation métriques bot (P&L, win rate, positions) | L1-440 | 
| src/modules/monitoring/risk_metrics.rs | Module | Risk metrics (Sharpe, VaR, Drawdown) | L1-260 |
| src/modules/monitoring/dashboard.rs | UI | Dashboard terminal (ratatui) | L1-460 |
| src/modules/monitoring/mod.rs | Module | Exports monitoring | L1-30 |
| src/bot.rs | Runtime | Boucle trading (pas d’export metrics) | L1-520 |

### Architecture Actuelle
```
TradingBot -> Strategy -> Metrics (in-memory) -> Dashboard (TUI)
(no HTTP endpoint /metrics)
```

### Code Snippets Clés
#### src/modules/monitoring/metrics.rs
```rust
pub struct BotMetrics { /* balances, trades, positions */ }
impl BotMetrics { pub fn win_rate(&self) -> f64 { ... } }
```

## 📚 Documentation Externe (Context7)
- ⚠️ Context7 indisponible dans cet environnement (outils MCP non configurés).

## 🔗 Dépendances

### Internes
- monitoring::metrics -> dashboard
- bot.rs n’expose pas de serveur HTTP

### Externes
- Aucune dépendance Prometheus dans Cargo.toml actuellement.

## ⚠️ Points d'Attention
- Aucune infra HTTP server dans le bot (pas d’API interne).
- Les métriques sont accessibles uniquement via TUI (ratatui).

## 💡 Opportunités Identifiées
- Ajouter un serveur HTTP léger (hyper/axum) pour /metrics.
- Réutiliser BotMetrics/RiskMetrics comme source de vérité.

## 📊 Résumé Exécutif
- Les métriques existent en mémoire mais ne sont pas exportées.
- Aucun endpoint /metrics ni stack d’alerting configuré.
- Ajout d’un export Prometheus est un changement transversal (runtime + deps).
