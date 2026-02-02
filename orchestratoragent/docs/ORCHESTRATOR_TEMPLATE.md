# Template Orchestrateur Multi-LLM

## Pseudo-code Python

```python
import subprocess
import time

SESSION = "moana-orchestration"
WINDOWS = {
    "amp": 2,
    "antigravity": 4,
    "codex": 5
}

def run_tmux(cmd):
    """Exécute une commande tmux"""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.stdout

def send_prompt(window, prompt):
    """Envoie un prompt à un LLM via tmux"""
    window_id = WINDOWS.get(window, window)

    # 1. Envoyer le prompt avec Enter
    cmd = f"tmux send-keys -t {SESSION}:{window_id} '{prompt}' Enter"
    run_tmux(cmd)

    # 2. Attendre un peu
    time.sleep(3)

    # 3. Vérifier si exécuté
    output = get_pane_output(window)

    # 4. Si le prompt est visible mais pas exécuté, envoyer Enter
    if prompt[:50] in output or "↵ send" in output:
        run_tmux(f"tmux send-keys -t {SESSION}:{window_id} Enter")
        time.sleep(1)

def get_pane_output(window, lines=15):
    """Récupère les dernières lignes du pane"""
    window_id = WINDOWS.get(window, window)
    cmd = f"tmux capture-pane -t {SESSION}:{window_id} -p | tail -{lines}"
    return run_tmux(cmd)

def check_status(window):
    """Vérifie le status d'un LLM"""
    output = get_pane_output(window)

    # Indicateurs de travail en cours
    working_indicators = [
        "Running tools",
        "Streaming response",
        "Thinking",
        "Burrowing",
        "Embellishing",
        "Waiting for response"
    ]

    # Indicateurs de fin
    completed_indicators = [
        "files changed",
        "Brewed for",
        "Baked for",
        "test result: ok"
    ]

    for indicator in working_indicators:
        if indicator in output:
            return "WORKING"

    for indicator in completed_indicators:
        if indicator in output:
            return "COMPLETED"

    return "IDLE"

def orchestrate(tasks):
    """Orchestration principale"""

    # 1. Distribuer les tâches initiales
    print("=== Distribution des tâches ===")
    for llm, task in tasks.items():
        print(f"Envoi à {llm}: {task[:50]}...")
        send_prompt(llm, task)

    # 2. Boucle de monitoring
    pending_tasks = []  # Queue de tâches supplémentaires

    while True:
        time.sleep(20)  # Vérifier toutes les 20 secondes

        print("\n=== Vérification des LLMs ===")
        all_idle = True

        for llm in WINDOWS.keys():
            status = check_status(llm)
            print(f"{llm}: {status}")

            if status == "WORKING":
                all_idle = False
            elif status == "COMPLETED" and pending_tasks:
                # Donner une nouvelle tâche
                new_task = pending_tasks.pop(0)
                print(f"Nouvelle tâche pour {llm}: {new_task[:50]}...")
                send_prompt(llm, new_task)
                all_idle = False

        if all_idle and not pending_tasks:
            print("\n=== Toutes les tâches terminées ===")
            break

# Exemple d'utilisation
if __name__ == "__main__":
    tasks = {
        "amp": "Crée le fichier circuit_breakers.rs avec...",
        "antigravity": "Crée le fichier risk_metrics.rs avec...",
        "codex": "Vérifie la compilation avec cargo check..."
    }
    orchestrate(tasks)
```

---

## Script Bash Équivalent

```bash
#!/bin/bash

SESSION="moana-orchestration"
PROJECT="/home/julien/Documents/palm-oil-bot"

# Fonction pour envoyer un prompt
send_prompt() {
    local window=$1
    local prompt=$2

    echo "Envoi à fenêtre $window..."

    # Envoyer le prompt
    tmux send-keys -t "$SESSION:$window" "$prompt" Enter

    # Attendre
    sleep 3

    # Vérifier et envoyer Enter si nécessaire
    output=$(tmux capture-pane -t "$SESSION:$window" -p | tail -5)
    if echo "$output" | grep -q "↵ send"; then
        tmux send-keys -t "$SESSION:$window" Enter
    fi
}

# Fonction pour vérifier le status
check_status() {
    local window=$1
    local output=$(tmux capture-pane -t "$SESSION:$window" -p | tail -15)

    if echo "$output" | grep -qE "Running tools|Streaming|Thinking|Burrowing"; then
        echo "WORKING"
    elif echo "$output" | grep -qE "files changed|Brewed for|test result: ok"; then
        echo "COMPLETED"
    else
        echo "IDLE"
    fi
}

# Distribuer les tâches
echo "=== Distribution des tâches ==="

send_prompt 2 "Crée le fichier $PROJECT/src/modules/trading/circuit_breakers.rs avec CircuitBreakers struct"
send_prompt 4 "Crée le fichier $PROJECT/src/modules/monitoring/risk_metrics.rs avec Sharpe, Sortino, MaxDrawdown"
send_prompt 5 "Vérifie que $PROJECT compile avec cargo check et corrige les erreurs"

# Boucle de monitoring
echo "=== Monitoring ==="
while true; do
    sleep 20

    echo ""
    echo "Status à $(date +%H:%M:%S):"
    echo "  AMP: $(check_status 2)"
    echo "  Antigravity: $(check_status 4)"
    echo "  Codex: $(check_status 5)"

    # Vérifier si tous terminés
    amp_status=$(check_status 2)
    anti_status=$(check_status 4)
    codex_status=$(check_status 5)

    if [[ "$amp_status" == "COMPLETED" && "$anti_status" == "COMPLETED" && "$codex_status" == "COMPLETED" ]]; then
        echo "=== Toutes les tâches terminées ==="
        break
    fi
done
```

---

## Checklist pour l'Orchestrateur

### Avant de commencer
- [ ] Vérifier que la session tmux existe: `tmux ls`
- [ ] Lister les fenêtres: `tmux list-windows -t SESSION`
- [ ] Vérifier que les LLMs sont actifs dans chaque fenêtre

### Distribution des tâches
- [ ] Préparer les prompts en langage naturel (pas de commandes bash)
- [ ] Inclure les chemins complets des fichiers
- [ ] Spécifier clairement ce qui est attendu
- [ ] Envoyer avec `send-keys "prompt" Enter`
- [ ] Vérifier avec `capture-pane` que le prompt est exécuté
- [ ] Envoyer `Enter` supplémentaire si nécessaire

### Monitoring
- [ ] Vérifier toutes les 15-30 secondes
- [ ] Identifier les LLMs terminés
- [ ] Donner de nouvelles tâches immédiatement

### Validation
- [ ] Vérifier la compilation: `cargo check`
- [ ] Lancer les tests: `cargo test`
- [ ] Documenter les résultats

---

## Format des Prompts Efficaces

### Structure recommandée

```
[ACTION] le fichier [CHEMIN_COMPLET]
avec [DESCRIPTION_DÉTAILLÉE].
Inclus [EXIGENCES_SPÉCIFIQUES].
```

### Exemples

**Création de module:**
```
Crée le fichier /home/julien/Documents/palm-oil-bot/src/modules/trading/circuit_breakers.rs
avec un struct CircuitBreakers qui implémente:
- daily_loss_limit (f64): limite de perte journalière
- consecutive_losses (u32): compteur de pertes consécutives
- is_trading_allowed(): retourne bool
Inclus des tests unitaires pour chaque fonction.
```

**Vérification:**
```
Vérifie que le projet /home/julien/Documents/palm-oil-bot compile avec cargo check.
S'il y a des erreurs, corrige-les.
Ensuite lance cargo test pour vérifier les tests.
```

**Correction:**
```
Corrige l'erreur dans /home/julien/Documents/palm-oil-bot/src/modules/trading/ctrader.rs ligne 415.
Remplace self.handle_spot_event(spot_event).await
par Self::handle_spot_event(spot_event, &self.prices).await
```
