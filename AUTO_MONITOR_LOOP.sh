#!/bin/bash
# Auto-monitoring orchestration avec re-dispatch automatique

SESSION="orchestration-palm-oil-bot"
PROJECT="/home/julien/Documents/palm-oil-bot"
CLAUDE_MD="$PROJECT/CLAUDE.md"

echo "🎯 ORCHESTRATION AUTO-MONITOR - Démarré $(date)"
echo "Session: $SESSION"
echo "Monitoring: CLAUDE.md pour COMPLETED"
echo ""

while true; do
    echo "─────────────────────────────────────────"
    echo "⏰ Check: $(date +%H:%M:%S)"
    
    # Vérifier Antigravity (window 4)
    ANTI_DONE=$(grep -c "TODO-ANTI-001: COMPLETED" "$CLAUDE_MD" 2>/dev/null || echo 0)
    if [ "$ANTI_DONE" -eq 0 ]; then
        echo "🔄 Antigravity: TODO-ANTI-001 EN COURS"
        tmux capture-pane -t $SESSION:4 -p | tail -3
    else
        echo "✅ Antigravity: TODO-ANTI-001 DONE"
        
        # Dispatcher TODO-ANTI-002
        ANTI2_DONE=$(grep -c "TODO-ANTI-002: COMPLETED" "$CLAUDE_MD" 2>/dev/null || echo 0)
        if [ "$ANTI2_DONE" -eq 0 ]; then
            echo "📤 Dispatch TODO-ANTI-002 à Antigravity..."
            tmux send-keys -t $SESSION:4 "TODO-ANTI-002: Position Reconciliation. Créer src/modules/trading/position_reconciliation.rs avec cache local HashMap, re-sync après reconnexion, logs audit. Tests: tests/position_reconciliation_test.rs. Documenter CLAUDE.md: ### TODO-ANTI-002: COMPLETED. GO!" Enter
        fi
    fi
    
    echo ""
    
    # Vérifier Codex (window 5)
    CODEX_DONE=$(grep -c "TODO-CODEX-003: COMPLETED" "$CLAUDE_MD" 2>/dev/null || echo 0)
    if [ "$CODEX_DONE" -eq 0 ]; then
        echo "🔄 Codex: TODO-CODEX-003 EN COURS"
        tmux capture-pane -t $SESSION:5 -p | tail -3
    else
        echo "✅ Codex: TODO-CODEX-003 DONE"
        
        # Dispatcher TODO-CODEX-002
        CODEX2_DONE=$(grep -c "TODO-CODEX-002: COMPLETED" "$CLAUDE_MD" 2>/dev/null || echo 0)
        if [ "$CODEX2_DONE" -eq 0 ]; then
            echo "📤 Dispatch TODO-CODEX-002 à Codex..."
            tmux send-keys -t $SESSION:5 "TODO-CODEX-002: Sentiment Cache. Créer src/modules/scraper/sentiment_cache.rs avec HashMap<String, (i32, Instant)>, TTL 5min, fallback Twitter si rate limited. Tests expiration. Documenter CLAUDE.md: ### TODO-CODEX-002: COMPLETED. GO!" Enter
        fi
    fi
    
    echo ""
    echo "⏳ Prochain check dans 60s..."
    sleep 60
done
