#!/bin/bash
# Autonomous monitoring script for AMP orchestrator
# Monitors Codex and Antigravity execution

LOG_FILE="/home/julien/Documents/palm-oil-bot/orchestratoragent/logs/autonomous_$(date +%Y%m%d_%H%M%S).log"
mkdir -p "$(dirname "$LOG_FILE")"

echo "=== AMP Autonomous Monitoring Started ===" | tee -a "$LOG_FILE"
echo "Time: $(date)" | tee -a "$LOG_FILE"
echo "Duration: 2 hours" | tee -a "$LOG_FILE"
echo "" | tee -a "$LOG_FILE"

check_agent() {
    local window=$1
    local agent_name=$2
    
    echo "[$(date +%H:%M:%S)] Checking $agent_name (window $window)..." | tee -a "$LOG_FILE"
    
    # Capture last 10 lines from tmux window
    tmux capture-pane -t palm-oil-orchestration:$window -p | tail -10 >> "$LOG_FILE"
    echo "---" >> "$LOG_FILE"
}

# Monitor loop (every 5 minutes for 2 hours = 24 iterations)
for i in {1..24}; do
    echo "" | tee -a "$LOG_FILE"
    echo "=== Check #$i at $(date +%H:%M:%S) ===" | tee -a "$LOG_FILE"
    
    check_agent 2 "Codex"
    check_agent 4 "Antigravity"
    
    # Wait 5 minutes
    sleep 300
done

echo "" | tee -a "$LOG_FILE"
echo "=== Monitoring Complete at $(date) ===" | tee -a "$LOG_FILE"
