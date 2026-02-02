# TASK-OPT-002: RSI Thresholds Data Analysis

**Assigné à**: Fullstack Developer (agents_library/fullstack-developer.md)  
**Priorité**: MOYENNE  
**ETA**: 40min  
**Complexité**: MOYENNE

## Objectif

Analyser corrélation RSI/Sentiment/P&L via data analysis et visualisations.

## Fichier à Créer

`scripts/rsi_analysis.py` (Python pour analysis + viz)

## Étapes

### 1. Export Data depuis Backtest

Modifier `src/bin/backtest.rs` pour exporter CSV:
```rust
// Export trade data
let mut csv_writer = csv::Writer::from_path("backtest_data.csv")?;
csv_writer.write_record(&["timestamp", "rsi", "sentiment", "pnl", "action"])?;
```

### 2. Analyse Python

`scripts/rsi_analysis.py`:
```python
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns

# Load data
df = pd.read_csv('backtest_data.csv')

# 1. Correlation Matrix
corr = df[['rsi', 'sentiment', 'pnl']].corr()
sns.heatmap(corr, annot=True)
plt.savefig('docs/correlation_matrix.png')

# 2. RSI vs P&L Scatter
plt.scatter(df['rsi'], df['pnl'])
plt.xlabel('RSI')
plt.ylabel('P&L')
plt.savefig('docs/rsi_pnl_scatter.png')

# 3. Heatmap RSI thresholds vs Profit Factor
# Grid: RSI oversold [20-40] vs overbought [60-80]
# Color: profit factor
```

### 3. Recommandations

Générer rapport:
- Sweet spots RSI identifiés
- Corrélations significatives
- Recommandations paramètres optimaux

## Dépendances Python

```bash
pip install pandas matplotlib seaborn numpy
```

## Livrables

1. `scripts/rsi_analysis.py` complet
2. Visualisations PNG dans `docs/`:
   - `correlation_matrix.png`
   - `rsi_pnl_scatter.png`
   - `rsi_threshold_heatmap.png`
3. Rapport PDF: `docs/RSI_ANALYSIS_REPORT.pdf`
4. Recommandations dans `orchestratoragent/FULLSTACK_OPT_002_RESPONSE.md`

## Agent

Utilise **fullstack-developer.md** (opus, peut faire Python + data viz)
