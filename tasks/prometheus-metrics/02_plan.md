# Plan d'Implémentation: prometheus-metrics

## 📋 Informations
**Date:** 2026-01-26
**Basé sur:** 01_analysis.md
**Approche:** Ajouter un serveur HTTP minimal pour /metrics, exporter BotMetrics/RiskMetrics

## 🎯 Objectif Final
Exposer `/metrics` en format Prometheus avec métriques clés (P&L, win rate, drawdown, circuit breakers) et documenter l’alerting de base.

## 📊 Gap Analysis
| État Actuel | État Cible | Action Requise |
| --- | --- | --- |
| Metrics en mémoire uniquement | Export Prometheus | Ajouter endpoint HTTP + registry |
| Pas de dépendance metrics | Dépendance stable | Ajouter crate Prometheus + HTTP server |

## 🏗️ Architecture Proposée
```
TradingBot -> MetricsHandle -> /metrics (HTTP)
```

## 📝 Checklist Technique (Step-by-Step)

### Phase 1: Préparation
- [ ] Ajouter dépendances `prometheus` + `axum` (ou `hyper`) dans Cargo.toml
- [ ] Créer module `src/modules/monitoring/prometheus.rs`

### Phase 2: Implémentation Core
- [ ] Définir gauges/counters (balance, pnl, win_rate, drawdown, open_positions)
- [ ] Mapper BotMetrics snapshot -> registry
- [ ] Ajouter server HTTP `/metrics` (port configurable: `METRICS_PORT`)

### Phase 3: Intégration
- [ ] Démarrer le serveur metrics dans `TradingBot::run()` (task Tokio)
- [ ] Ajouter config env `METRICS_ENABLED`, `METRICS_PORT`

### Phase 4: Tests & Validation
- [ ] Tests unitaires: render /metrics output non-vide
- [ ] Test manuel: `curl http://localhost:PORT/metrics`

## 🔧 Commandes à Exécuter
```bash
cargo test
```

## ⚠️ Risques Identifiés
| Risque | Impact | Mitigation |
| --- | --- | --- |
| Exposition réseau | Moyen | Bind localhost par défaut |
| Overhead metrics | Faible | Snapshot périodique |

## 🔍 Points de Validation
- [ ] `/metrics` répond 200
- [ ] Valeurs cohérentes avec BotMetrics

## 📊 Estimation
- **Complexité:** Moyenne
- **Fichiers modifiés:** 2-4
- **Fichiers créés:** 1

## 🚦 Prêt pour Implémentation
- [ ] Analyse complète (01_analysis.md ✓)
- [ ] Plan validé par l'utilisateur
