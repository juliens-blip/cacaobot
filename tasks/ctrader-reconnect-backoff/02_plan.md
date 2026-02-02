# Plan d'Implémentation: ctrader-reconnect-backoff

## 📋 Informations
**Date:** 2026-01-26
**Basé sur:** 01_analysis.md
**Approche:** Utiliser retry_with_backoff + boucle de reconnexion

## 🎯 Objectif Final
Reconnexion automatique au broker avec backoff exponentiel + re-auth robuste.

## 📊 Gap Analysis
| État Actuel | État Cible | Action Requise |
| --- | --- | --- |
| Connexion unique | Reconnexion auto | Ajouter wrapper retry + loop |
| Pas de backoff | Backoff + jitter | Utiliser utils::retry_with_backoff |

## 🏗️ Architecture Proposée
```
TradingBot.run -> connect_with_retry -> authenticate_with_retry
```

## 📝 Checklist Technique (Step-by-Step)

### Phase 1: Préparation
- [ ] Identifier points d’échec (connect/auth/send)
- [ ] Réutiliser `utils::retry_with_backoff`

### Phase 2: Implémentation Core
- [ ] Ajouter méthodes `connect_with_retry`, `authenticate_with_retry`
- [ ] Boucle de reconnexion si heartbeat/send échoue

### Phase 3: Intégration
- [ ] Appeler wrappers dans `TradingBot::run()`
- [ ] Ajouter logs et compteur de tentatives

### Phase 4: Tests & Validation
- [ ] Tests unitaires sur backoff helper
- [ ] Test manuel: simulate disconnect (network cut)

## 🔧 Commandes à Exécuter
```bash
cargo test
```

## ⚠️ Risques Identifiés
| Risque | Impact | Mitigation |
| --- | --- | --- |
| Loop infinie | Moyen | Max retries + circuit breaker |
| Thundering herd | Faible | Jitter |

## 🔍 Points de Validation
- [ ] Reconnecte après coupure réseau
- [ ] Pas de retry agressif

## 📊 Estimation
- **Complexité:** Moyenne
- **Fichiers modifiés:** 2-3

## 🚦 Prêt pour Implémentation
- [ ] Analyse complète (01_analysis.md ✓)
- [ ] Plan validé par l'utilisateur
