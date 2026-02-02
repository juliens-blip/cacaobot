use palm_oil_bot::{Config, TradingBot};

#[test]
fn test_bot_new_dry_run() {
    let mut config = Config::default();
    config.bot.dry_run = true;

    let bot = TradingBot::new(config);
    assert!(bot.is_ok());
}
