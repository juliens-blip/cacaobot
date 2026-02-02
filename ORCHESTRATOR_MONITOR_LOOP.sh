#!/bin/bash
# Orchestrator Auto-Monitor Loop
# Surveille Codex et dispatche nouveau TODO quand COMPLETED détecté

SESSION="orchestration-palm-oil-bot"
CODEX_WINDOW=5
CLAUDE_MD="/home/julien/Documents/palm-oil-bot/CLAUDE.md"
TODO_FILE="/home/julien/Documents/palm-oil-bot/TODO_CODEX_NOUVELLES_TACHES.md"

echo "🤖 Orchestrator Auto-Monitor ACTIVE"
echo "Session: $SESSION"
echo "Codex: window $CODEX_WINDOW"
echo "Monitoring: $CLAUDE_MD pour COMPLETED"
echo ""

while true; do
  # Vérifier TODO-CODEX-004
  CODEX004_DONE=$(grep -c "TODO-CODEX-004: COMPLETED" "$CLAUDE_MD" 2>/dev/null || echo 0)
  
  if [ "$CODEX004_DONE" -eq 0 ]; then
    echo "🔄 [$(date +%H:%M:%S)] TODO-CODEX-004 EN COURS..."
    
    # Vérifier output Codex
    OUTPUT=$(tmux capture-pane -t $SESSION:$CODEX_WINDOW -p | tail -5)
    
    if echo "$OUTPUT" | grep -q "files changed"; then
      echo "⚠️  Codex semble avoir terminé (files changed détecté)"
      echo "   Attente documentation CLAUDE.md..."
    fi
    
  else
    echo "✅ TODO-CODEX-004 COMPLETED détecté !"
    echo "📤 Dispatch TODO-CODEX-005 à Codex..."
    
    # Envoyer TODO-CODEX-005
    tmux send-keys -t $SESSION:$CODEX_WINDOW '<system>
Tu es backend-architect expert sécurité Rust. CHARGE @/home/julien/Documents/palm-oil-bot/agents_library/backend-architect.md.
</system>

<task>
TODO-CODEX-005: Implémenter security hardening (secrets + rate limiting)
</task>

<context>
Projet: Palm Oil Trading Bot
Déploiement: Railway (production)
Besoin sécurité:
- Secrets chargés depuis env uniquement (no .env runtime)
- Panic si secrets manquants (fail-fast)
- Rate limiting Perplexity API (éviter ban + coûts)
- Rate limiting Twitter scraping (éviter IP ban)

Fichiers existants:
- src/config.rs (charge config depuis .env)
- src/modules/scraper/perplexity.rs (appelle Perplexity)
- src/modules/scraper/twitter.rs (scrappe Twitter)
</context>

<constraints>
- OBLIGATOIRE: Charger @backend-architect.md
- Secrets: std::env::var().expect() avec messages clairs
- Rate limiter: crate '\''governor'\'' OU '\''backoff'\'' (exponential + jitter)
- Logs ne doivent JAMAIS exposer secrets (redacted)
- Tests: mock env vars + assert panic si manquant
- Documenter CLAUDE.md "TODO-CODEX-005 COMPLETED" avec ID, date, fichiers
</constraints>

<deliverables>
1. src/modules/security/mod.rs (module declaration)
2. src/modules/security/secrets_manager.rs (Config::from_env_strict)
3. src/modules/security/rate_limiter.rs (RateLimiter with backoff)
4. Modification config.rs (integration secrets_manager)
5. tests/security_test.rs (8+ tests secrets + rate limit)
6. Documentation CLAUDE.md section "TODO-CODEX-005 COMPLETED"
</deliverables>

<acceptance_criteria>
- cargo build --release: PASSED
- Panic si CTRADER_CLIENT_ID manquant
- Rate limiter bloque après N calls (configurable)
- Tests security PASSED
</acceptance_criteria>' Enter
    
    echo "✅ TODO-CODEX-005 envoyé !"
    
    # Passer au monitoring TODO-CODEX-005
    break
  fi
  
  sleep 60  # Check toutes les 60s
done

echo ""
echo "🔄 Monitoring TODO-CODEX-005 maintenant..."

while true; do
  CODEX005_DONE=$(grep -c "TODO-CODEX-005: COMPLETED" "$CLAUDE_MD" 2>/dev/null || echo 0)
  
  if [ "$CODEX005_DONE" -eq 0 ]; then
    echo "🔄 [$(date +%H:%M:%S)] TODO-CODEX-005 EN COURS..."
  else
    echo "✅ TODO-CODEX-005 COMPLETED !"
    echo "📤 Dispatcher TODO-CODEX-006 (monitoring - complexe)..."
    break
  fi
  
  sleep 60
done

echo "🎯 Boucle orchestrateur terminée. Relancer pour TODO-CODEX-006."
