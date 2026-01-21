#!/bin/bash
cd /home/julien/Documents/palm-oil-bot

echo "🔍 CODEX - Code Review Agent"
echo "=============================="
echo ""

# Lancer Claude avec l'agent code-reviewer
claude --agent agents_library/code-reviewer.md << 'EOF'
# MISSION CODEX - Code Review Palm Oil Bot

**Context**: /home/julien/Documents/palm-oil-bot
**Task**: TASK-PO-013

## Instructions

1. Lis orchestratoragent/CODEX_TASK.md pour comprendre ta mission
2. Analyse tous les fichiers Rust dans src/
3. Vérifie:
   - Gestion d'erreurs (pas de unwrap en production)
   - Sécurité (pas de secrets hardcodés)
   - Architecture et dépendances
   - Tests et documentation
4. Crée CODEX_REVIEW_REPORT.md avec:
   - Issues critiques/majeures/mineures
   - Métriques de qualité
   - Recommandations concrètes
5. Mets à jour ORCHESTRATION_STATUS.md

Commence maintenant.
EOF
