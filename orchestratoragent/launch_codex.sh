#!/bin/bash

# Launch Codex for TASK-PO-013: Code Review

echo "🚀 Launching Codex - Code Review Agent"
echo "Task: TASK-PO-013"
echo "Workspace: /home/julien/Documents/palm-oil-bot"
echo ""

# Check if amp CLI is available
if ! command -v amp &> /dev/null; then
    echo "❌ AMP CLI not found. Please submit this prompt manually to Codex agent:"
    echo ""
    echo "==============================================="
    cat << 'EOF'
# CODEX - Code Review Mission

**Context**: Palm Oil Trading Bot en Rust
**Workspace**: /home/julien/Documents/palm-oil-bot
**Task**: TASK-PO-013 - Code Review & Compilation Check

## Your Mission

Lire `/home/julien/Documents/palm-oil-bot/orchestratoragent/CODEX_TASK.md` et exécuter la review complète.

## Steps
1. Read CODEX_TASK.md
2. Analyze all Rust files in src/
3. Check error handling, security, performance
4. Generate CODEX_REVIEW_REPORT.md
5. Update ORCHESTRATION_STATUS.md

## Output
Create comprehensive review report with:
- Critical/Major/Minor issues
- Code quality metrics
- Concrete recommendations

Start now.
EOF
    echo "==============================================="
    echo ""
    echo "📋 Instructions:"
    echo "1. Copy the prompt above"
    echo "2. Open Codex agent (code-reviewer)"
    echo "3. Paste and execute"
else
    # Launch with amp CLI
    echo "✅ AMP CLI found - Launching Codex..."
    cd /home/julien/Documents/palm-oil-bot
    amp agent code-reviewer --prompt "$(cat orchestratoragent/CODEX_TASK.md)"
fi
