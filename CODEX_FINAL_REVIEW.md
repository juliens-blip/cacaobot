# CODEX Final Review - TASK-PO-013

Date: 2026-01-22
Reviewer: Codex
Scope: /home/julien/Documents/palm-oil-bot/src

## 1) Architecture review

- **Séparation des responsabilités globalement cohérente** : `src/modules` isole clairement *scraper* (sentiment), *trading* (cTrader + stratégie), *monitoring* (metrics + dashboard) et *utils*. Cela facilite les tests unitaires et la lecture (ex: `src/modules/trading/strategy.rs`, `src/modules/monitoring/metrics.rs`, `src/modules/scraper/perplexity.rs`).
- **Deux “moteurs” concurrents** : le binaire `src/main.rs` implémente un `BotState` autonome tandis que `src/bot.rs` expose un `TradingBot` plus structuré (event pipeline, cache sentiment, candles). Cette duplication crée une dette technique et un risque d’incohérence fonctionnelle (logique d’exécution, metrics, gestion d’événements).
- **Pipeline event-driven partiellement intégré** : `src/modules/trading/event_system.rs` fournit un pub/sub complet, mais il n’est utilisé que par `TradingBot` (pas par `main.rs`). L’architecture gagnerait à converger vers un seul moteur + un bus d’événements unique.
- **Gestion des données de marché** : `CandleBuilder` + `Tick` (`src/modules/trading/candles.rs`) sont propres, mais `main.rs` travaille en prix “mid” instantané plutôt qu’en candles, alors que `bot.rs` agrège les ticks. Cela complique la cohérence stratégie/backtest.
- **cTrader client isolé mais “monolithique”** : `src/modules/trading/ctrader.rs` gère connexion, auth, lecture, heartbeat, et parsing. L’absence d’un “dispatcher” de messages robuste limite l’extensibilité (ex: réconciliation de positions, gestion des erreurs d’ordres, historique).

## 2) Points d’attention sécurité

- **Risque critique de concurrence sur le socket** : `start_reader()` lit en continu la socket *et* `wait_for_message()` lit simultanément la même socket (`src/modules/trading/ctrader.rs`). Cela peut corrompre le framing, perdre des messages ou bloquer l’auth. En production, ce pattern est **non sûr**.
- **Framing incohérent du protocole** : `start_reader()` attend un header 8 bytes (type+len), alors que `send_message()` encode un message avec un préfixe 4 bytes de longueur (`ProtoMessage::encode_with_length()`), et `wait_for_message()` lit aussi un préfixe 4 bytes (`src/modules/trading/ctrader.rs`, `src/modules/trading/protobuf.rs`). Cette divergence peut provoquer des erreurs de parsing, des désynchronisations et des timeouts.
- **Authentification cTrader “demo only”** : usage de `client_id` comme access token (`authenticate()`) est explicitement une simplification. En prod, il faut OAuth + refresh token. Risque d’échec d’auth en réel.
- **Absence de TLS vérifiée** : la connexion est un `TcpStream` brut (`ctrader.rs`). Si cTrader exige TLS (souvent le cas), c’est un risque de confidentialité et de MITM. À confirmer et corriger.
- **Secrets dans l’environnement sans hardening** : configuration via `.env` est OK en dev (`src/config.rs`), mais il n’y a ni rotation de secrets, ni support KMS/Vault, ni masquage systématique des logs côté erreurs. Pour production, c’est insuffisant.
- **Fallback Twitter via scraping** : `src/modules/scraper/twitter.rs` utilise Nitter en guest mode. Cela peut provoquer des blocages, des timeouts ou du contenu non fiable. Risque d’indisponibilité + dépendance non contractuelle.

## 3) Optimisations possibles

- **Unifier le moteur d’exécution** : choisir `TradingBot` *ou* `BotState` comme entrée principale. Aujourd’hui deux loops de trading différentes créent des divergences et doublons de logique.
- **Refactor cTrader client en “single reader + router”** : un seul task lit la socket, push dans un channel, puis un dispatcher route les messages (auth responses, spot events, execution events). Supprime la concurrence dangereuse et permet un backoff/reconnect propre.
- **Améliorer la gestion de l’état compte/positions** : implémenter `ProtoOAReconcileReq` (cf. TODO dans `ctrader.rs`) pour synchroniser l’état réel au démarrage et après reconnexion.
- **Backoff/retry centralisé** : utiliser `utils::retry_with_backoff` pour les appels critiques (Perplexity, cTrader send/receive) avec métriques de retry et circuit breakers.
- **Persistance minimale** : stocker positions ouvertes + P&L localement (ex: SQLite ou JSON file) pour reprise après crash et audit.
- **Validation stricte des configs** : ajout de checks (ex: `max_positions >= 1`, `risk_per_trade` borné, `account_id` numérique) dans `Config::validate()`.
- **Optimisations de polling** : le loop trading peut intégrer un “price stale check” pour éviter les ordres basés sur des prix anciens (si feed cTrader se coupe).

## 4) Validation production-ready

**Verdict: NON production-ready** (bloqueur).

### Bloqueurs critiques
- **Concurrence sur la socket cTrader** (lecture simultanée) + framing incohérent → risque de corruption, pertes d’events, exécution aléatoire. (`src/modules/trading/ctrader.rs`, `src/modules/trading/protobuf.rs`)
- **Authentification cTrader non conforme** (access token “demo only”).
- **Absence de réconciliation positions** et de reprise après crash.

### Manques majeurs
- Pas de stratégie de reconnect + backoff robuste côté cTrader.
- Pas de journal d’audit des ordres/trades persisté.
- Pas de tests d’intégration end-to-end (cTrader demo, Perplexity live, Twitter fallback).
- Monitoring et alerting limités au dashboard local (pas de logs structurés, pas d’export prometheus).

### OK pour pré-prod / sandbox
- La structure des modules est propre.
- La couverture de tests unitaires est solide sur RSI/EMA/candles/metrics/strategy.
- Le mode `dry_run` permet de valider la boucle sans exécution réelle.

---

**Recommandation**: corriger le client cTrader (framing + concurrency + auth), unifier le moteur d’exécution, et ajouter une couche de persistance/observabilité avant tout déploiement réel.

---

# CODEX Final Review - TODO-CODEX-001/002/003

Date: 2026-01-26
Reviewer: Codex
Scope: TODO-CODEX-001, TODO-CODEX-002, TODO-CODEX-003

## TODO-CODEX-001: Backtest Parameter Sweep

### Résumé
- Ajout d’un binaire d’optimisation par grid search couvrant RSI buy/sell, TP et SL, avec export CSV.
- Exécution locale pour trouver une combinaison avec profit_factor > 1.5.

### Fichiers créés
- `src/bin/backtest_optimizer.rs`
- `backtest_results.csv`

### Tests validés
- `cargo run --bin backtest-optimizer`

### Problèmes rencontrés
- Aucun blocant. Profit factor retourné `inf` pour l’optimum trouvé (aucune perte sur la simulation).

## TODO-CODEX-002: Sentiment Cache System

### Résumé
- Cache sentiment en mémoire avec TTL 5 min, `HashMap<String, (i32, Instant)>`.
- Logs info sur cache hit/miss.
- Fallback Twitter uniquement en cas de rate limit Perplexity (HTTP 429).

### Fichiers créés
- `src/modules/scraper/sentiment_cache.rs` (réécrit pour la version TTL demandée)

### Tests validés
- Tests unitaires ajoutés dans `src/modules/scraper/sentiment_cache.rs`.

### Problèmes rencontrés
- `cargo test sentiment_cache` a échoué à cause d’erreurs préexistantes (champs `environment` manquants dans des initialisations `CTraderConfig`, et tests TLS obsolètes dans `tests/tls_verification_test.rs`). Les tests du cache n’ont pas pu être exécutés tant que ces erreurs persistent.

## TODO-CODEX-003: TLS Certificate Validation

### Résumé
- Binaire de validation TLS avec rustls pour `live.ctraderapi.com:5035` et `demo.ctraderapi.com:5035`.
- Affichage des métadonnées du certificat (subject/issuer/validité/SANs).

### Fichiers créés
- `src/bin/test_tls_connection.rs`

### Tests validés
- `cargo run --bin test-tls-connection`

### Problèmes rencontrés
- Aucun blocant.
