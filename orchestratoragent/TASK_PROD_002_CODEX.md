# TASK-PROD-002: TLS Verification Tests

**Assigné à**: Codex  
**Priorité**: CRITIQUE  
**ETA**: 20min

## Objectif

Créer tests de validation TLS pour connexion cTrader LIVE.

## Fichier

`tests/tls_verification_test.rs`

## Tests Requis

### 1. test_live_server_connection()
- Connect à `live.ctraderapi.com:5035`
- Vérifier handshake TLS réussi
- Vérifier certificat valide

### 2. test_tls_certificate_chain()
- Vérifier chaîne de certificats
- Vérifier date d'expiration

### 3. test_tls_cipher_suites()
- Vérifier ciphers supportés
- Vérifier TLS 1.2+ minimum

### 4. test_demo_vs_live_connection()
- Comparer comportement DEMO/LIVE
- Documenter différences

## Dépendances

- `rustls` ou `native-tls`
- `tokio-rustls` pour tests async

## Note

Tests peuvent fail si pas d'accès LIVE - documenter comment tester manuellement.

## Livrable

- `tls_verification_test.rs` complet
- Documentation test manual
- Cargo.toml mis à jour si nouvelles deps

## Rapport

Écris ton avancement dans `orchestratoragent/CODEX_RESPONSE.md`
