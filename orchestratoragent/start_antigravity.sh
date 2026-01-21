#!/bin/bash
cd /home/julien/Documents/palm-oil-bot

echo "🧠 ANTIGRAVITY - Strategy Analysis Agent"
echo "=========================================="
echo ""

# Lancer Claude avec l'agent backend-architect (pour analyse stratégique)
claude --agent agents_library/backend-architect.md << 'EOF'
# MISSION ANTIGRAVITY - Strategy Optimization

**Context**: /home/julien/Documents/palm-oil-bot
**Task**: TASK-PO-011

## Instructions

1. Lis orchestratoragent/ANTIGRAVITY_TASK.md pour ta mission complète
2. Analyse src/modules/trading/strategy.rs (RSI + sentiment)
3. Évalue le risk management actuel
4. Recherche les caractéristiques du marché FCPO palm oil
5. Propose ≥3 optimisations concrètes avec code Rust:
   - Multi-indicator confirmation
   - Dynamic position sizing
   - Time-based filters
   - Sentiment confidence score
6. Crée ANTIGRAVITY_STRATEGY_REPORT.md avec roadmap d'implémentation
7. Mets à jour ORCHESTRATION_STATUS.md

Commence maintenant.
EOF
