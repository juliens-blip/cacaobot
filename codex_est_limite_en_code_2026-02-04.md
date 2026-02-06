# Codex est limité en code — 2026-02-04

## Ce qui ne marche pas
1. **Build bloqué par l’exe en cours**
   - `palm-oil-bot.exe` restait lancé.
   - Windows refuse de le remplacer → `Access denied`.

2. **Commande “tout-en-un” cassée par les logs**
   - Les backticks ` ont été recollés pendant que le bot logguait.
   - Les logs ont été envoyés dans `Stop-Process` → erreur `InputObjectNotBound`.

3. **Reconnexion instable**
   - Déconnexions `early eof` fréquentes côté demo.
   - Ré-auth trop répétée → `ALREADY_LOGGED_IN`.
   - Parfois `Timeout waiting for response`.

## Ce qu’on a déjà essayé
1. **Fix `ALREADY_LOGGED_IN`**
   - AppAuth ne crash plus si ce code revient.

2. **Mode test immédiat BUY+SELL**
   - Ajout `TEST_IMMEDIATE_TRADES=1`.
   - Envoie BUY puis SELL, puis ferme.

3. **Multiples recompilations**
   - Souvent bloquées par exe encore actif.

## Ce qui a fonctionné au moins une fois
- Log confirmé : `TEST_IMMEDIATE_TRADES enabled: placing BUY then SELL`
- `Order placed` + `Order executed`

## Résumé court
Le bot peut passer des trades en demo, mais la stabilité est cassée par :
- exe verrouillé pendant le build
- reconnect instable cTrader demo
- commandes PowerShell cassées par logs
