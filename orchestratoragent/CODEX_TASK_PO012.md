# CODEX - TASK-PO-012: Tests Unitaires

**Agent**: CODEX (OpenAI)
**Date**: 2026-01-23
**Priorité**: 🔴 HAUTE
**Durée estimée**: 30-40 min
**Status**: READY TO EXECUTE

---

## 📋 OBJECTIF

Créer des tests unitaires complets pour valider la logique de trading avant déploiement production.

---

## 🎯 TÂCHES À EXÉCUTER

### 1. Tests Strategy Module (`src/modules/trading/strategy.rs`)

Ajouter section `#[cfg(test)]` à la fin du fichier:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_buy_oversold_bullish() {
        let rsi = 25.0;
        let sentiment = 40;
        assert!(should_buy(rsi, sentiment), "RSI oversold + bullish sentiment should trigger BUY");
    }

    #[test]
    fn test_should_buy_neutral_no_signal() {
        let rsi = 50.0;
        let sentiment = 0;
        assert!(!should_buy(rsi, sentiment), "Neutral conditions should NOT trigger BUY");
    }

    #[test]
    fn test_should_buy_oversold_bearish_no_signal() {
        let rsi = 28.0;
        let sentiment = -40;
        assert!(!should_buy(rsi, sentiment), "Oversold but bearish sentiment should NOT trigger BUY");
    }

    #[test]
    fn test_should_sell_overbought_bearish() {
        let rsi = 75.0;
        let sentiment = -40;
        assert!(should_sell(rsi, sentiment), "RSI overbought + bearish sentiment should trigger SELL");
    }

    #[test]
    fn test_should_sell_neutral_no_signal() {
        let rsi = 50.0;
        let sentiment = 0;
        assert!(!should_sell(rsi, sentiment), "Neutral conditions should NOT trigger SELL");
    }

    #[test]
    fn test_should_sell_overbought_bullish_no_signal() {
        let rsi = 72.0;
        let sentiment = 40;
        assert!(!should_sell(rsi, sentiment), "Overbought but bullish sentiment should NOT trigger SELL");
    }

    #[test]
    fn test_edge_case_rsi_exact_threshold() {
        // RSI exactly at 30 (oversold threshold)
        assert!(!should_buy(30.0, 40), "RSI=30 should NOT trigger buy (< 30 required)");
        
        // RSI exactly at 70 (overbought threshold)
        assert!(!should_sell(70.0, -40), "RSI=70 should NOT trigger sell (> 70 required)");
    }

    #[test]
    fn test_edge_case_sentiment_exact_threshold() {
        // Sentiment exactly at +30 (bullish threshold)
        assert!(!should_buy(25.0, 30), "Sentiment=30 should NOT trigger buy (> 30 required)");
        
        // Sentiment exactly at -30 (bearish threshold)
        assert!(!should_sell(75.0, -30), "Sentiment=-30 should NOT trigger sell (< -30 required)");
    }
}
```

### 2. Tests Sentiment Module (`src/modules/scraper/sentiment.rs`)

Ajouter tests pour parsing:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_positive_sentiment() {
        let text = "Based on market analysis, the sentiment score is +75 indicating strong bullish momentum";
        let score = parse_sentiment(text);
        assert_eq!(score, 75);
    }

    #[test]
    fn test_parse_negative_sentiment() {
        let text = "Current market sentiment: -50 (bearish trend)";
        let score = parse_sentiment(text);
        assert_eq!(score, -50);
    }

    #[test]
    fn test_parse_neutral_sentiment() {
        let text = "Sentiment score: 0 (neutral market conditions)";
        let score = parse_sentiment(text);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_parse_invalid_sentiment_defaults_zero() {
        let text = "No clear sentiment indicators in this text";
        let score = parse_sentiment(text);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_parse_multiple_scores_takes_first() {
        let text = "Score: +60 was previous, now Score: +80";
        let score = parse_sentiment(text);
        assert_eq!(score, 60, "Should extract first score found");
    }

    #[test]
    fn test_parse_out_of_range_clamps() {
        // If implementation clamps values, test that
        let text = "Extreme bullish score: +150";
        let score = parse_sentiment(text);
        assert!(score <= 100, "Score should be clamped to max 100");
    }
}
```

### 3. Tests RSI Calculator (`src/modules/trading/indicators.rs`)

Ajouter tests pour RsiCalculator:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_oversold_on_falling_prices() {
        let mut rsi = RsiCalculator::new(14);
        
        // Simulate 14+ periods of falling prices
        let prices = vec![
            100.0, 99.0, 98.0, 97.0, 96.0, 95.0, 94.0,
            93.0, 92.0, 91.0, 90.0, 89.0, 88.0, 87.0, 86.0
        ];
        
        for price in prices {
            rsi.update(price);
        }
        
        let current_rsi = rsi.current().expect("RSI should be calculated");
        assert!(current_rsi < 30.0, "RSI should be oversold (<30) on falling prices, got {}", current_rsi);
    }

    #[test]
    fn test_rsi_overbought_on_rising_prices() {
        let mut rsi = RsiCalculator::new(14);
        
        // Simulate 14+ periods of rising prices
        let prices = vec![
            100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0,
            107.0, 108.0, 109.0, 110.0, 111.0, 112.0, 113.0, 114.0
        ];
        
        for price in prices {
            rsi.update(price);
        }
        
        let current_rsi = rsi.current().expect("RSI should be calculated");
        assert!(current_rsi > 70.0, "RSI should be overbought (>70) on rising prices, got {}", current_rsi);
    }

    #[test]
    fn test_rsi_none_before_warmup() {
        let mut rsi = RsiCalculator::new(14);
        
        // Add only 10 periods (less than 14)
        for i in 0..10 {
            rsi.update(100.0 + i as f64);
        }
        
        assert!(rsi.current().is_none(), "RSI should be None before 14 periods");
    }

    #[test]
    fn test_rsi_available_after_warmup() {
        let mut rsi = RsiCalculator::new(14);
        
        // Add exactly 14 periods
        for i in 0..14 {
            rsi.update(100.0 + i as f64);
        }
        
        assert!(rsi.current().is_some(), "RSI should be available after 14 periods");
    }
}
```

---

## ✅ VALIDATION

Après avoir ajouté les tests, exécuter:

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test --lib trading::strategy::tests
cargo test --lib scraper::sentiment::tests
cargo test --lib trading::indicators::tests

# Show test output
cargo test -- --nocapture
```

**Critères de succès**:
- ✅ Tous les tests passent (`test result: ok`)
- ✅ Aucun warning/error de compilation
- ✅ Coverage estimée > 80% pour modules critiques

---

## 📦 LIVRABLES

1. **Code modifié**:
   - `src/modules/trading/strategy.rs` (8 tests ajoutés)
   - `src/modules/scraper/sentiment.rs` (6 tests ajoutés)
   - `src/modules/trading/indicators.rs` (4 tests ajoutés)

2. **Rapport de tests**:
   Créer `TESTS_REPORT.md`:
   ```markdown
   # Tests Report - TASK-PO-012
   
   **Date**: 2026-01-23
   **Agent**: Codex
   
   ## Tests Exécutés
   
   Total: 18 tests unitaires
   - Strategy: 8 tests
   - Sentiment: 6 tests
   - RSI Calculator: 4 tests
   
   ## Résultats
   
   ```
   [Coller output de `cargo test`]
   ```
   
   ## Coverage Estimée
   
   - strategy.rs: ~85%
   - sentiment.rs: ~75%
   - indicators.rs: ~80%
   
   ## Status
   
   ✅ TASK-PO-012 COMPLÉTÉ
   ```

---

## 🚨 NOTES IMPORTANTES

- **Ne modifie PAS la logique existante**, SEULEMENT ajouter tests
- **Si un test échoue**, documenter dans TESTS_REPORT.md et marquer comme BLOCKED
- **Pattern à suivre**: Utiliser `#[cfg(test)]` mod tests à la fin de chaque fichier
- **Naming**: Tests nommés `test_<functionality>_<expected_result>()`

---

## 📝 TODO CHECKLIST

- [ ] Ajouter tests strategy.rs (8 tests)
- [ ] Ajouter tests sentiment.rs (6 tests)
- [ ] Ajouter tests indicators.rs (4 tests)
- [ ] Exécuter `cargo test`
- [ ] Vérifier tous passent
- [ ] Créer TESTS_REPORT.md
- [ ] Commit changes avec message "feat: add unit tests for trading modules (TASK-PO-012)"

---

**READY TO EXECUTE** - Lance immédiatement cette tâche.
