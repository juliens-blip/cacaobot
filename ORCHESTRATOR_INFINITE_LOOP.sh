#!/bin/bash
# Boucle infinie autonome - Orchestrator surveille et re-dispatche

SESSION="orchestration-palm-oil-bot"
CLAUDE_MD="/home/julien/Documents/palm-oil-bot/CLAUDE.md"
LOG="/home/julien/Documents/palm-oil-bot/ORCHESTRATOR_LOOP.log"

echo "🤖 ORCHESTRATOR INFINITE LOOP STARTED - $(date)" | tee -a $LOG
echo "Session: $SESSION" | tee -a $LOG
echo "Monitoring every 60s..." | tee -a $LOG
echo "" | tee -a $LOG

while true; do
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a $LOG
    echo "⏰ [$(date +%H:%M:%S)] CHECK AGENTS" | tee -a $LOG
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" | tee -a $LOG
    
    # ═══════════════════════════════════════════════
    # ANTIGRAVITY (window 4)
    # ═══════════════════════════════════════════════
    
    # TODO-ANTI-001
    if grep -q "### TODO-ANTI-001: COMPLETED" "$CLAUDE_MD"; then
        echo "✅ Antigravity: TODO-ANTI-001 DONE" | tee -a $LOG
        
        # Check si TODO-ANTI-002 déjà dispatché
        ANTI_OUTPUT=$(tmux capture-pane -t $SESSION:4 -p | tail -20)
        if echo "$ANTI_OUTPUT" | grep -q "TODO-ANTI-002"; then
            echo "   → TODO-ANTI-002 déjà en cours" | tee -a $LOG
        else
            echo "📤 Dispatching TODO-ANTI-002 à Antigravity..." | tee -a $LOG
            tmux send-keys -t $SESSION:4 "TODO-ANTI-002: Position Reconciliation System. Créer src/modules/trading/position_reconciliation.rs avec cache local HashMap<String, Position>, mécanisme re-sync après reconnexion, logs détaillés audit trail avec timestamps. Tests connexions intermittentes: tests/position_reconciliation_test.rs. Utiliser agents_library/backend-architect.md et agents_library/apex-workflow.md. Documenter CLAUDE.md: ### TODO-ANTI-002: COMPLETED **Tests passing** **Cache implemented**. GO!" Enter
        fi
    else
        echo "🔄 Antigravity: TODO-ANTI-001 EN COURS" | tee -a $LOG
        tmux capture-pane -t $SESSION:4 -p | tail -3 | tee -a $LOG
    fi
    
    # TODO-ANTI-002
    if grep -q "### TODO-ANTI-002: COMPLETED" "$CLAUDE_MD"; then
        echo "✅ Antigravity: TODO-ANTI-002 DONE" | tee -a $LOG
        
        ANTI_OUTPUT=$(tmux capture-pane -t $SESSION:4 -p | tail -20)
        if echo "$ANTI_OUTPUT" | grep -q "TODO-ANTI-003"; then
            echo "   → TODO-ANTI-003 déjà en cours" | tee -a $LOG
        else
            echo "📤 Dispatching TODO-ANTI-003 à Antigravity..." | tee -a $LOG
            tmux send-keys -t $SESSION:4 "TODO-ANTI-003: OAuth Production Setup. Modifier src/modules/trading/ctrader.rs pour switch DEMO/LIVE selon CTRADER_ENVIRONMENT=demo|live. Ajouter variables dans .env.example. Documenter flux OAuth production dans README.md section Production Deployment. Créer guide migration DEMO → LIVE. Utiliser agents_library/backend-architect.md. Documenter CLAUDE.md: ### TODO-ANTI-003: COMPLETED **README updated** **Migration guide created**. GO!" Enter
        fi
    fi
    
    echo "" | tee -a $LOG
    
    # ═══════════════════════════════════════════════
    # CODEX (window 5)
    # ═══════════════════════════════════════════════
    
    # TODO-CODEX-002
    if grep -q "### TODO-CODEX-002: COMPLETED" "$CLAUDE_MD"; then
        echo "✅ Codex: TODO-CODEX-002 DONE" | tee -a $LOG
        
        CODEX_OUTPUT=$(tmux capture-pane -t $SESSION:5 -p | tail -20)
        if echo "$CODEX_OUTPUT" | grep -q "TODO-CODEX-001"; then
            echo "   → TODO-CODEX-001 déjà en cours" | tee -a $LOG
        else
            echo "📤 Dispatching TODO-CODEX-001 à Codex..." | tee -a $LOG
            tmux send-keys -t $SESSION:5 "TODO-CODEX-001: Backtest Parameter Sweep. Créer src/bin/backtest_optimizer.rs avec grid search: RSI buy 20-35 step 5, RSI sell 65-80 step 5, TP 1.5-3% step 0.5%, SL 1-2% step 0.5%. Output CSV backtest_results.csv avec colonnes [rsi_buy, rsi_sell, tp, sl, profit_factor, win_rate]. Trouver combinaison profit_factor > 1.5. Utiliser agents_library/test-engineer.md. Documenter CLAUDE.md: ### TODO-CODEX-001: COMPLETED **Profit Factor** **Optimal Params**. GO!" Enter
        fi
    else
        echo "🔄 Codex: TODO-CODEX-002 EN COURS" | tee -a $LOG
        tmux capture-pane -t $SESSION:5 -p | tail -3 | tee -a $LOG
    fi
    
    echo "" | tee -a $LOG
    echo "⏳ Sleep 60s avant prochain check..." | tee -a $LOG
    sleep 60
done
