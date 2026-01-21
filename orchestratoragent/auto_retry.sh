#!/bin/bash
# Auto-retry script for Antigravity if it fails

LOG="/home/julien/Documents/palm-oil-bot/orchestratoragent/logs/auto_retry.log"
mkdir -p "$(dirname "$LOG")"

echo "=== Auto-Retry Script Started at $(date) ===" | tee -a "$LOG"

for attempt in {1..20}; do
    echo "" | tee -a "$LOG"
    echo "[Attempt $attempt] Checking Antigravity status..." | tee -a "$LOG"
    
    # Check if proxy is running
    if ! pgrep -f antigravity-claude-proxy > /dev/null; then
        echo "[Attempt $attempt] Proxy not running - restarting..." | tee -a "$LOG"
        
        # Kill any existing instances
        pkill -9 -f antigravity-claude-proxy 2>/dev/null
        sleep 2
        
        # Restart proxy in tmux window 3
        tmux send-keys -t palm-oil-orchestration:3 C-c C-c
        sleep 1
        tmux send-keys -t palm-oil-orchestration:3 "antigravity-claude-proxy start" Enter
        sleep 5
        
        # Restart client in window 4
        tmux send-keys -t palm-oil-orchestration:4 C-c C-c
        sleep 1
        tmux send-keys -t palm-oil-orchestration:4 'export ANTHROPIC_BASE_URL="http://localhost:8080" && export ANTHROPIC_AUTH_TOKEN="test"' Enter
        sleep 1
        tmux send-keys -t palm-oil-orchestration:4 "claude --dangerously-skip-permissions" Enter
        sleep 8
        tmux send-keys -t palm-oil-orchestration:4 "Execute TASK-APEX-001: Advanced Strategy Engine with multi-indicators in advanced_strategy.rs" Enter
    fi
    
    # Check for errors in Antigravity window
    ERROR_COUNT=$(tmux capture-pane -t palm-oil-orchestration:4 -p | grep -c -i "error\|failed\|unable")
    
    if [ "$ERROR_COUNT" -gt 0 ]; then
        echo "[Attempt $attempt] Detected $ERROR_COUNT errors - will retry in 30s" | tee -a "$LOG"
        sleep 30
    else
        echo "[Attempt $attempt] No errors detected - looks good" | tee -a "$LOG"
    fi
    
    # Wait 60s between checks
    sleep 60
done

echo "=== Auto-Retry Script Finished at $(date) ===" | tee -a "$LOG"
