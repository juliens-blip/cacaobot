# Plan d'Implémentation: sqlite-trade-audit

## 📋 Informations
**Date:** 2026-01-26
**Basé sur:** 01_analysis.md
**Approche:** Compléter l’audit trail (export + reconciliation DB/broker)

## 🎯 Objectif Final
Audit trades durable avec export CSV/JSON et vérifications post-crash.

## 📊 Gap Analysis
| État Actuel | État Cible | Action Requise |
| --- | --- | --- |
| closed_trades stockés | Export audit | Ajouter exporteur + tests |
| Pas de vérif au startup | Validation | Ajouter check DB -> log |

## 📝 Checklist Technique (Step-by-Step)

### Phase 1: Préparation
- [ ] Ajouter méthodes export dans `persistence.rs` (CSV/JSON)

### Phase 2: Implémentation Core
- [ ] Export closed_trades + daily_stats
- [ ] Ajouter commande CLI `export-trades` (bin)

### Phase 3: Intégration
- [ ] Documenter usage dans README/DEPLOY_CHECKLIST

### Phase 4: Tests & Validation
- [ ] Tests d’export (temp DB, contenu non vide)

## 🔧 Commandes à Exécuter
```bash
cargo test
```

## ⚠️ Risques Identifiés
| Risque | Impact | Mitigation |
| --- | --- | --- |
| Gros volume | Moyen | Pagination/export streaming |

## 🔍 Points de Validation
- [ ] Export fonctionne sur DB locale
- [ ] Format stable (CSV/JSON)

## 📊 Estimation
- **Complexité:** Faible à moyenne
- **Fichiers créés:** 1 bin + tests

## 🚦 Prêt pour Implémentation
- [ ] Analyse complète (01_analysis.md ✓)
- [ ] Plan validé par l'utilisateur
