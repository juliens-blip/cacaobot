//! Bot startup validation tests.

use palm_oil_bot::bot::TradingBot;
use palm_oil_bot::config::{
    BotConfig, CTraderConfig, Config, PerplexityConfig, StrategyConfig, TradingConfig,
    TradingEnvironment,
};

fn test_config_without_token() -> Config {
    Config {
        ctrader: CTraderConfig {
            environment: TradingEnvironment::Demo,
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            account_id: "test_account_id".to_string(),
            access_token: None,
            server: "demo.ctraderapi.com".to_string(),
            port: 5035,
            client_id_live: None,
            client_secret_live: None,
            account_id_live: None,
        },
        perplexity: PerplexityConfig {
            api_key: "test_key".to_string(),
            endpoint: "https://api.perplexity.ai/chat/completions".to_string(),
            model: "sonar".to_string(),
        },
        trading: TradingConfig {
            symbol: "FCPO".to_string(),
            risk_per_trade: 1.0,
            take_profit_percent: 2.0,
            stop_loss_percent: 1.5,
            max_positions: 1,
            max_daily_loss_percent: 5.0,
                initial_balance: 10000.0,
        },
        strategy: StrategyConfig {
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            rsi_timeframe: "5m".to_string(),
            sentiment_threshold: 30,
        },
        kols: vec!["PalmOilTrader".to_string()],
        bot: BotConfig {
            cycle_interval_secs: 1,
            dry_run: true,
            log_level: "info".to_string(),
        },
    }
}

#[tokio::test]
async fn test_bot_startup_fails_without_access_token() {
    let config = test_config_without_token();
    let mut bot = TradingBot::new(config).expect("bot creation");

    let err = bot.run().await.expect_err("bot should fail without access token");
    let message = err.to_string();
    assert!(
        message.contains("CTRADER_ACCESS_TOKEN") || message.contains("get-token"),
        "unexpected error message: {message}"
    );
}
