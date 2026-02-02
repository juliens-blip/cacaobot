# Plan d'Implémentation: security-hardening

## 📋 Informations
**Date:** 2026-01-26
**Basé sur:** 01_analysis.md
**Approche:** Redaction logs + validation config + guardrails DRY_RUN

## 🎯 Objectif Final
Renforcer sécurité via validation stricte, redaction des secrets, et garde-fous live.

## 📊 Gap Analysis
| État Actuel | État Cible | Action Requise |
| --- | --- | --- |
| Validation config minimale | Validation stricte | Étendre Config::validate | 
| Logs non-redactés | Logs safe | Masquer secrets et tokens |
| DRY_RUN non forcé en live | Garde-fou | Bloquer live si DRY_RUN=true non explicite |

## 📝 Checklist Technique (Step-by-Step)

### Phase 1: Validation Config
- [ ] Ajouter checks: account_id numérique, port valide, max_positions >=1
- [ ] Bloquer CTRADER_ENV=live si credentials live absents

### Phase 2: Redaction Logs
- [ ] Helper de redaction pour secrets/token
- [ ] Appliquer aux logs critiques (Perplexity/ctrader)

### Phase 3: Guardrails
- [ ] Alerte si DRY_RUN=false en live sans confirmation (env flag)

### Phase 4: Tests
- [ ] Tests unitaires validation config

## 🔧 Commandes à Exécuter
```bash
cargo test --lib
```

## ⚠️ Risques Identifiés
| Risque | Impact | Mitigation |
| --- | --- | --- |
| Faux positifs validation | Moyen | Messages d’erreur clairs |

## 🚦 Prêt pour Implémentation
- [ ] Analyse complète (01_analysis.md ✓)
- [ ] Plan validé par l'utilisateur
