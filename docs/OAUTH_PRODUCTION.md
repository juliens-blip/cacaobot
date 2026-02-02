# 🔐 Guide OAuth Production - cTrader Live

**Version**: 1.0  
**Date**: 2026-01-24  
**Bot**: Palm Oil Trading Bot  

---

## 📋 Vue d'ensemble

Ce guide explique comment migrer le bot du compte **DEMO** cTrader vers un compte **LIVE** (production) en toute sécurité.

⚠️ **ATTENTION**: Le trading LIVE implique de l'argent réel. Ne déployez jamais sans tests exhaustifs sur DEMO.

---

## 🔄 Différences DEMO vs LIVE

| Aspect | DEMO | LIVE |
|--------|------|------|
| **Serveur** | demo.ctraderapi.com:5035 | live.ctraderapi.com:5035 |
| **Argent** | Virtuel (illimité) | Réel (votre capital) |
| **OAuth App** | App DEMO | App LIVE (approval requise) |
| **Certificat TLS** | Certificat test | Certificat production |
| **Rate Limits** | Plus permissifs | Plus stricts |
| **Latence** | Variable | Critique |

---

## 📝 Prérequis

Avant de commencer:

1. ✅ **Compte cTrader LIVE vérifié**
   - KYC complété (Know Your Customer)
   - Capital déposé
   - 2FA activé

2. ✅ **OAuth Application approuvée**
   - Créée sur https://ctrader.com/developer
   - Type: Trading Bot
   - Scopes: `trading`, `accounts`, `ohlc`
   - Status: **APPROVED** (peut prendre 1-3 jours)

3. ✅ **Tests DEMO réussis**
   - Bot tourne stable 7+ jours
   - Profit factor > 1.5
   - Aucun bug critique
   - Circuit breakers fonctionnent

---

## 🔑 Obtenir les Credentials LIVE

### Étape 1: Créer OAuth App sur cTrader

1. Connexion: https://ctrader.com/developer
2. **Applications** → **New Application**
3. Remplir:
   - **Name**: Palm Oil Bot Production
   - **Type**: Trading Bot
   - **Description**: Automated palm oil CFD trading bot using RSI + sentiment analysis
   - **Redirect URI**: `http://localhost:8080/callback` (si callback needed)
   - **Scopes**: 
     - ✅ `trading` (place/modify/cancel orders)
     - ✅ `accounts` (read account info)
     - ✅ `ohlc` (market data)
4. **Submit for Review**
5. **Attendre approval** (email de confirmation)

### Étape 2: Récupérer Client ID / Secret

Une fois approuvé:
1. Ouvrir l'app dans Developer Portal
2. Copier **Client ID** (format: `XXXXX_XXXXXXXXX`)
3. Copier **Client Secret** (format: `YYYYYYYYYYY...`)
4. Copier **Account ID** LIVE depuis cTrader platform (Settings → API)

⚠️ **Ne JAMAIS partager ces credentials** - Donnent accès complet à votre compte trading.

---

## ⚙️ Configuration .env

### Étape 1: Backup .env actuel

```bash
cp .env .env.demo.backup
```

### Étape 2: Modifier .env

```bash
# ============================================
# ENVIRONMENT SELECTION
# ============================================
CTRADER_ENVIRONMENT=live  # ⚠️ CHANGER ICI: demo | live

# ============================================
# DEMO CREDENTIALS (pour tests)
# ============================================
CTRADER_DEMO_CLIENT_ID=12345_ABC123DEF
CTRADER_DEMO_CLIENT_SECRET=demo_secret_key_here
CTRADER_DEMO_ACCOUNT_ID=1234567

# ============================================
# LIVE CREDENTIALS (⚠️ PRODUCTION ONLY)
# ============================================
CTRADER_LIVE_CLIENT_ID=67890_XYZ789GHI
CTRADER_LIVE_CLIENT_SECRET=live_secret_key_here
CTRADER_LIVE_ACCOUNT_ID=9876543

# ============================================
# PERPLEXITY API
# ============================================
PERPLEXITY_API_KEY=pplx-xxxxxxxxxxxxx

# ============================================
# RISK MANAGEMENT (⚠️ PRODUCTION)
# ============================================
MAX_DAILY_LOSS_PCT=3.0          # Plus strict en LIVE (3% vs 5%)
MAX_POSITION_SIZE=0.05          # Volume max par trade (0.05 lots)
ENABLE_CIRCUIT_BREAKERS=true    # OBLIGATOIRE en LIVE
```

### Étape 3: Vérifier .gitignore

```bash
# Vérifier que .env est ignoré
cat .gitignore | grep "^\.env$"

# Si absent, ajouter
echo ".env" >> .gitignore
```

---

## ✅ Tests de Connexion

### Test 1: Vérifier TLS Certificate

```bash
# Tester connexion TLS au serveur LIVE
cargo run --bin test_tls_connection

# Output attendu:
# ✅ LIVE Server (live.ctraderapi.com:5035): OK
# ✅ Certificate: VALID
# ✅ Handshake: SUCCESS
```

**Si échec**:
- Vérifier firewall
- Vérifier DNS resolution
- Contacter support cTrader

### Test 2: Test OAuth Flow

```bash
# Tester l'authentification
cargo run --bin test_connection

# Output attendu:
# ✅ Environment: LIVE
# ✅ OAuth Token: Obtained
# ✅ Account ID: 9876543
# ✅ Balance: $10,000.00
# ✅ Connection: STABLE
```

**Si échec**:
- Vérifier credentials dans .env
- Vérifier que l'OAuth app est **APPROVED**
- Vérifier scopes requis

### Test 3: Test Minimal (Paper Trading)

Avant de risquer de l'argent:

```bash
# Lancer bot avec flag dry-run (si implémenté)
CTRADER_ENVIRONMENT=live cargo run -- --dry-run

# Vérifier dans logs:
# - Connexion OK
# - Prix reçus
# - Signaux générés
# - AUCUN ordre réel envoyé
```

---

## 🔐 Sécurité

### 1. Ne JAMAIS commit .env

```bash
# Vérifier status git
git status

# Si .env apparaît:
git rm --cached .env
git commit -m "Remove .env from tracking"
```

### 2. Utiliser Railway Secrets

Pour déploiement production:

```bash
# Ajouter secrets via Railway CLI
railway variables set CTRADER_ENVIRONMENT=live
railway variables set CTRADER_LIVE_CLIENT_ID=xxx
railway variables set CTRADER_LIVE_CLIENT_SECRET=xxx
railway variables set CTRADER_LIVE_ACCOUNT_ID=xxx

# Vérifier (sans afficher valeurs)
railway variables list
```

### 3. Activer 2FA sur cTrader

1. cTrader Platform → Settings → Security
2. Enable **Two-Factor Authentication**
3. Scanner QR code avec Google Authenticator
4. Backup codes: **stocker dans password manager sécurisé**

### 4. IP Whitelisting (si disponible)

1. cTrader Developer Portal → App Settings
2. **Allowed IPs**: Ajouter IP de Railway container
3. Obtenir IP Railway:
   ```bash
   railway run curl ifconfig.me
   ```

---

## 🚀 Migration DEMO → LIVE

### Checklist Complète

#### Phase 1: Validation DEMO (1-2 semaines)

- [ ] Bot stable 7+ jours sur DEMO
- [ ] Profit factor > 1.5 (backtest + live)
- [ ] Win rate > 60%
- [ ] Circuit breakers déclenchés et fonctionnels
- [ ] Aucun crash / erreur critique
- [ ] Logs propres (pas d'exceptions)
- [ ] Dashboard monitoring opérationnel

#### Phase 2: Setup LIVE (1 jour)

- [ ] Compte cTrader LIVE vérifié (KYC)
- [ ] OAuth App créée et **APPROVED**
- [ ] Credentials LIVE obtenus
- [ ] .env configuré avec LIVE credentials
- [ ] Tests TLS passés
- [ ] Tests OAuth passés
- [ ] 2FA activé sur compte
- [ ] Railway secrets configurés

#### Phase 3: Tests LIVE (2-3 jours)

- [ ] Test connexion LIVE (test_connection.rs)
- [ ] Test dry-run (aucun ordre réel)
- [ ] Test 1 ordre manuel (volume minimum)
- [ ] Test 1 trade complet (entry + TP/SL)
- [ ] Test circuit breakers en conditions réelles
- [ ] Vérifier latence réseau acceptable (<100ms)
- [ ] Monitoring alertes fonctionnelles

#### Phase 4: Déploiement Production

- [ ] Capital initial déposé (recommandé: $10,000+)
- [ ] CTRADER_ENVIRONMENT=live dans Railway
- [ ] MAX_DAILY_LOSS_PCT réduit (3% recommandé)
- [ ] Surveillance 24/7 active (première semaine)
- [ ] Backup plan documenté
- [ ] Contact support cTrader en cas d'urgence

---

## 🔄 Rollback Plan

### Si problème en LIVE:

#### Option 1: Pause Immédiate

```bash
# SSH dans Railway container
railway run bash

# Arrêter le bot proprement
pkill -SIGTERM palm-oil-bot

# Vérifier arrêt
ps aux | grep palm-oil-bot
```

#### Option 2: Switch DEMO

```bash
# Modifier env var Railway
railway variables set CTRADER_ENVIRONMENT=demo

# Redéployer
railway up --detach

# Vérifier logs
railway logs --tail 100
```

#### Option 3: Fermer Positions Manuellement

1. Ouvrir cTrader Platform
2. **Positions** tab
3. **Close All Positions**
4. Vérifier balance

---

## 🐛 Troubleshooting

### Erreur: "Invalid client credentials"

**Cause**: Client ID/Secret incorrects  
**Solution**:
```bash
# Vérifier .env
cat .env | grep CTRADER_LIVE

# Re-copier depuis cTrader Developer Portal
# Vérifier absence d'espaces/retours à la ligne
```

### Erreur: "Insufficient scope"

**Cause**: OAuth app manque de permissions  
**Solution**:
1. Developer Portal → App Settings
2. **Scopes**: Ajouter `trading`, `accounts`, `ohlc`
3. **Save** → **Submit for Review** (re-approval nécessaire)

### Erreur: "Connection timeout"

**Cause**: Firewall / DNS  
**Solution**:
```bash
# Tester connectivité
ping live.ctraderapi.com
telnet live.ctraderapi.com 5035

# Si échec, vérifier Railway network settings
```

### Erreur: "Rate limit exceeded"

**Cause**: Trop de requêtes API  
**Solution**:
- Augmenter délai entre requêtes (60s → 120s)
- Implémenter cache Perplexity (TODO-CODEX-002)
- Contacter cTrader pour augmenter limits

---

## 📊 Monitoring Production

### KPIs à surveiller

| Métrique | Seuil Normal | Alerte si |
|----------|--------------|-----------|
| **Uptime** | >99% | <95% |
| **Latence API** | <100ms | >500ms |
| **Win Rate** | >55% | <50% |
| **Daily P&L** | Positif | <-3% |
| **Circuit Breakers** | <1/jour | >3/jour |
| **Memory Usage** | <500MB | >1GB |

### Dashboard Production

Ajouter à dashboard:
```rust
// src/modules/monitoring/dashboard.rs
fn render_production_status(frame, area) {
    // Environment badge
    let env = if is_live() { 
        Span::styled("LIVE 🔴", Style::red()) 
    } else { 
        Span::styled("DEMO 🟢", Style::green()) 
    };
    
    // Risk metrics
    let daily_loss = format!("{:.2}% / 3.0%", current_loss_pct);
    let positions = format!("{} / 1", open_positions_count);
    
    // ...
}
```

---

## 📞 Support

### Contacts d'urgence

| Problème | Contact |
|----------|---------|
| **Bug bot** | Votre équipe dev |
| **cTrader API** | api-support@ctrader.com |
| **Railway** | https://railway.app/help |
| **Perplexity** | support@perplexity.ai |

### Logs à fournir

En cas de support ticket:
```bash
# Logs Railway (dernières 24h)
railway logs --tail 1000 > bot_logs.txt

# Métriques système
railway run bash -c "free -h; df -h; ps aux" > system_metrics.txt

# Envoyer à support (⚠️ redact credentials)
```

---

## 📚 FAQ

### Q: Combien de capital minimum pour LIVE ?

**R**: Recommandé **$10,000+**. Le bot utilise 0.1 lots max, soit ~$1000 de marge par trade. Avec $10k, vous avez un bon buffer pour drawdowns.

### Q: Le bot peut-il perdre tout mon argent ?

**R**: Oui, si les circuit breakers échouent. C'est pourquoi:
- MAX_DAILY_LOSS_PCT=3% (limite à -$300/jour si $10k)
- Stop loss sur chaque trade (-1.5%)
- Surveillance 24/7 obligatoire première semaine

### Q: Dois-je surveiller le bot 24/7 ?

**R**: 
- **Première semaine LIVE**: OUI, surveillance stricte
- **Après stabilisation**: Checks 3-4x/jour suffisent
- **Alertes critiques**: Configurer notifications (email/SMS)

### Q: Combien de temps avant d'être profitable ?

**R**: Variable. En moyenne:
- **Semaine 1-2**: Rodage, breakeven
- **Semaine 3-4**: +0.5-1%/jour
- **Mois 2+**: Objectif 2-3%/jour

### Q: Puis-je switcher LIVE ↔ DEMO rapidement ?

**R**: Oui, via Railway env var:
```bash
railway variables set CTRADER_ENVIRONMENT=demo
```
Effet immédiat au prochain redémarrage.

### Q: Les credentials DEMO et LIVE peuvent coexister ?

**R**: Oui ! Le bot charge automatiquement les bonnes credentials selon `CTRADER_ENVIRONMENT`. C'est sécurisé.

---

**Auteur**: AMP Orchestrator  
**Version**: 1.0  
**Dernière mise à jour**: 2026-01-24  
**Fichier**: `docs/OAUTH_PRODUCTION.md`
