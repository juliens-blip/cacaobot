# HANDOFF: Claude → AMP
**Date**: 2026-01-21 17:50
**Raison**: Claude à 95% tokens utilisés

---

## Contexte

Projet **palm-oil-bot** - Bot de trading Rust pour Palm Oil CFDs.
Le projet est à **95% complet** avec 30 tests qui passent.

---

## Protocole Ralph en cours

Test E2E complet du bot. Tâches restantes:

| # | Tâche | Status |
|---|-------|--------|
| 1 | Explorer le code (structure, modules) | EN COURS |
| 2 | Tester compilation (cargo build) | PENDING |
| 3 | Exécuter tests unitaires (cargo test) | PENDING |
| 4 | Tester backtest (cargo run --bin backtest) | PENDING |
| 5 | Vérifier intégration event_system dans main.rs | PENDING |
| 6 | Identifier et corriger problèmes | PENDING |
| 7 | Rapport final Protocole Ralph | PENDING |

---

## Commandes à exécuter

```bash
cd /home/julien/Documents/palm-oil-bot

# 1. Compilation
cargo build 2>&1

# 2. Tests
cargo test 2>&1

# 3. Clippy
cargo clippy 2>&1

# 4. Backtest
cargo run --bin backtest 2>&1
```

---

## Prochaines étapes (post-Ralph)

1. [ ] Ajouter tests pour event_system et candles
2. [ ] Intégrer event_system dans main.rs
3. [ ] Ajouter orderbook.rs pour profondeur de marché
4. [ ] Tests de charge et performance
5. [ ] Déploiement Railway

---

## Fichiers clés

- `CLAUDE.md` - Documentation complète du projet
- `ORCHESTRATION_STATUS.md` - État de l'orchestration
- `src/main.rs` - Point d'entrée
- `src/lib.rs` - Library exports
- `tests/` - Tests unitaires

---

## Instructions pour AMP

1. Lire ce fichier et CLAUDE.md
2. Exécuter le Protocole Ralph (tests E2E)
3. Utiliser agents spécialisés si besoin:
   - `test-engineer` pour les tests
   - `debugger` pour les bugs
   - `Explore` pour explorer le code
4. Mettre à jour ORCHESTRATION_STATUS.md avec les résultats
5. Créer rapport final

**AMP est maintenant l'orchestrateur.**
