# TASK-PROD-001: OAuth Production cTrader

**Assigné à**: Apex  
**Priorité**: CRITIQUE  
**ETA**: 30min

## Objectif

Implémenter OAuth Production pour cTrader dans `src/modules/trading/oauth.rs`.

## Contexte

- Fichier existe avec auth DEMO fonctionnelle
- Besoin: OAuth 2.0 flow complet pour serveur LIVE
- Endpoint LIVE: `live.ctraderapi.com:5035`
- Variables: `CTRADER_CLIENT_ID_LIVE`, `CTRADER_CLIENT_SECRET_LIVE`

## Implémentation Requise

1. **Enum Environment**
   ```rust
   pub enum Environment {
       Demo,
       Live,
   }
   ```

2. **OAuth Flow Complet**
   - Refresh token mechanism
   - Token persistence (JSON sécurisé)
   - Auto-refresh avant expiration

3. **Fichiers à Modifier**
   - `src/modules/trading/oauth.rs`
   - `src/config.rs` (ajouter config LIVE)
   - `.env.example` (documenter variables LIVE)

## Tests Requis

- `test_oauth_demo_flow()`
- `test_oauth_live_flow()`
- `test_token_refresh()`
- `test_token_persistence()`

## Livrable

- oauth.rs production-ready
- Tests passant
- Documentation dans .env.example

## Rapport

Écris ton avancement dans `orchestratoragent/APEX_RESPONSE.md`
