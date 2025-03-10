use dotenv::from_filename;
use std::env;

#[actix::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = env::var("RUSTBOT_ENV").unwrap_or(".env.dev".to_string());
    from_filename(filename).ok();

    // rust_bot::run_actual_strategy().await?;
    rust_bot::run_multiple_strategies().await?;
    // rust_bot::run_true_once_buys().await?;
    // rust_bot::run_true_always().await?;
    // rust_bot::run_manual_candles().await?;
    // rust_bot::run_ws_only().await?;

    // rust_bot::run_setup_finder().await?;
    // rust_bot::run_manual_setups().await?;
    // rust_bot::run_single_indicator().awaIt?;
    // rust_bot::run_dummy().await?;
    // rust_bot::run_market_buy().await?;
    // rust_bot::run_market_sell_all().await?;
    // rust_bot::run_strategy().await?;
    // rust_bot::run_historical().await?;
    // rust_bot::run_local().await?;
    // rust_bot::run_strategy_testing().await?;
    // rust_bot::run_strategy_tester().await?;

    Ok(())
}
