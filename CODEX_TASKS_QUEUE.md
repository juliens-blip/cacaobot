# 📋 CODEX TASKS QUEUE

**Orchestrator**: AMP  
**Date**: 2026-01-24  
**Agent**: Codex  

---

## 🎯 Tâches Assignées (Par Ordre de Priorité)

### ⚡ TODO-CODEX-003: TLS Certificate Validation 🔒
**Status**: 🔄 TO_START  
**Priorité**: BLOQUANT  
**Agent**: backend-architect  
**Durée**: 1 prompt  

**Objectif**: Tester connexion TLS avec serveur LIVE cTrader

**Instructions**:
1. Créer `src/bin/test_tls_connection.rs`
2. Tester connexions:
   - live.ctraderapi.com:5035
   - demo.ctraderapi.com:5035
3. Vérifier certificats SSL/TLS
4. Documenter différences

**Reporting**: Quand terminé, ajouter dans CLAUDE.md:
```markdown
### TODO-CODEX-003: COMPLETED
**Date**: 2026-01-24 HH:MM
**LIVE Server**: [OK/FAIL]
**Certificate**: [VALID/INVALID]
**Issues**: [DESCRIPTION si problème]
```

---

### 🧠 TODO-CODEX-002: Sentiment Cache System
**Status**: ⏳ PENDING  
**Priorité**: OPTIMISATION  
**Agent**: backend-architect  
**Durée**: 2 prompts  

**Objectif**: Cache in-memory pour Perplexity API (éviter rate limits)

**Instructions**:
1. Créer `src/modules/scraper/sentiment_cache.rs`
2. Cache avec TTL 5 min
3. Fallback Twitter si rate limited
4. Tests unitaires expiration

**Reporting**: Ajouter dans CLAUDE.md:
```markdown
### TODO-CODEX-002: COMPLETED
**Date**: 2026-01-24 HH:MM
**Cache Hit Rate**: [XX%]
**Perplexity Calls Saved**: [YY%]
```

---

### 📊 TODO-CODEX-001: Backtest Parameter Sweep
**Status**: ⏳ PENDING  
**Priorité**: OPTIMISATION  
**Agent**: test-engineer  
**Durée**: 2 prompts  

**Objectif**: Optimiser RSI thresholds (profit factor > 1.5)

**Instructions**:
1. Créer `src/bin/backtest_optimizer.rs`
2. Grid search:
   - RSI buy: 20-35
   - RSI sell: 65-80
   - TP: 1.5%-3%
   - SL: 1%-2%
3. Output CSV avec résultats

**Reporting**: Ajouter dans CLAUDE.md:
```markdown
### TODO-CODEX-001: COMPLETED
**Date**: 2026-01-24 HH:MM
**Profit Factor**: [BEST_VALUE]
**Optimal Params**: RSI=[XX,YY], TP=[Z%], SL=[W%]
```

---

## 📝 Instructions Générales

1. **Commencer par TODO-CODEX-003** (le plus urgent)
2. Utiliser les agents de `agents_library/` pour implémentation
3. Après chaque TODO complétée:
   - Ajouter section dans CLAUDE.md
   - Commit fichiers créés
   - Passer à la suivante
4. **Ne pas attendre validation** entre les TODOs

---

**Status**: ✅ Queue créée, prête pour Codex
