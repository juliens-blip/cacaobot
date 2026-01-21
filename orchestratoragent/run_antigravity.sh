#!/bin/bash
cd /home/julien/Documents/palm-oil-bot

echo "🚀 ANTIGRAVITY - Strategy Analysis"
echo "=================================="
echo ""
echo "Analyse en cours..."
echo ""

# Lancer amp chat avec le prompt
/home/julien/.amp/bin/amp chat --model sonnet << 'PROMPT'
Tu es un expert en stratégies de trading quantitatives.

**MISSION**: Analyser la stratégie du Palm Oil Bot et proposer des optimisations.

**TACHES**:
1. Lire /home/julien/Documents/palm-oil-bot/src/modules/trading/strategy.rs
2. Analyser les conditions d'entrée (RSI + sentiment)
3. Évaluer le risk management (TP/SL)
4. Rechercher les caractéristiques du marché FCPO
5. Proposer ≥3 optimisations concrètes avec code Rust

**OUTPUT**: Créer /home/julien/Documents/palm-oil-bot/ANTIGRAVITY_STRATEGY_REPORT.md

Commence l'analyse maintenant.
PROMPT
