#!/bin/bash

# Launch Antigravity for TASK-PO-011: Strategy Analysis

echo "🚀 Launching Antigravity - Strategy Analysis Agent"
echo "Task: TASK-PO-011"
echo "Workspace: /home/julien/Documents/palm-oil-bot"
echo ""

# Check if amp CLI is available
if ! command -v amp &> /dev/null; then
    echo "❌ AMP CLI not found. Please submit this prompt manually to Antigravity agent:"
    echo ""
    echo "==============================================="
    cat << 'EOF'
# ANTIGRAVITY - Strategy Optimization Mission

**Context**: Palm Oil Trading Bot - RSI + Sentiment Strategy
**Workspace**: /home/julien/Documents/palm-oil-bot
**Task**: TASK-PO-011 - Strategy Analysis & Optimization

## Your Mission

Lire `/home/julien/Documents/palm-oil-bot/orchestratoragent/ANTIGRAVITY_TASK.md` et analyser la stratégie de trading.

## Steps
1. Read ANTIGRAVITY_TASK.md
2. Analyze current strategy (strategy.rs)
3. Research FCPO market characteristics
4. Identify weaknesses & propose optimizations
5. Generate ANTIGRAVITY_STRATEGY_REPORT.md
6. Update ORCHESTRATION_STATUS.md

## Output
Create strategy report with:
- Current strategy analysis
- Market research (FCPO)
- ≥3 concrete optimizations with code
- Implementation roadmap
- Impact estimations

Start now.
EOF
    echo "==============================================="
    echo ""
    echo "📋 Instructions:"
    echo "1. Copy the prompt above"
    echo "2. Open Antigravity agent (or strategy analyst)"
    echo "3. Paste and execute"
else
    # Launch with amp CLI (using backend-architect as proxy for strategy analysis)
    echo "✅ AMP CLI found - Launching Antigravity..."
    cd /home/julien/Documents/palm-oil-bot
    amp agent backend-architect --prompt "$(cat orchestratoragent/ANTIGRAVITY_TASK.md)"
fi
