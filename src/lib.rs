mod data_sources;
mod indicators;
mod models;
mod notifications;
mod resolution_strategies;
mod run_configs;
mod strategy_testing;
mod trading_strategies;
mod utils;

use crate::{
    data_sources::bybit::rest::bybit_rest_api::BybitRestApi,
    indicators::{atr::ATR, populates_candles::PopulatesCandles, rsi::RSI, stochastic::Stochastic},
    models::{net_version::NetVersion, websockets::wsclient::WebsocketClient},
    trading_strategies::private::jb_2::JB2,
};
use actix::Actor;
use anyhow::{Ok, Result};
use data_sources::{datasource::DataSource, local};
use dotenv::dotenv;
use indicators::{indicator_type::IndicatorType, populates_candles::PopulatesCandlesWithSelf};
use models::{
    candle::Candle,
    database::db::DB,
    interval::Interval,
    message_payloads::{
        ts_subscribe_payload::TSSubscribePayload, websocket_payload::WebsocketPayload,
    },
    setups::setup_finder_builder::SetupFinderBuilder,
    strategy_orientation::StrategyOrientation,
    timeseries::TimeSeries,
    timeseries_builder::TimeSeriesBuilder,
    traits::trading_strategy::TradingStrategy,
};
use run_configs::{always_true, manual_candles, multiple_strategies, ws_only};
use strategy_testing::strategy_tester::StrategyTester;
use tokio::time::{sleep, Duration};
use trading_strategies::{
    private::kq_14::KQ14,
    public::{rsi_basic::RsiBasic, true_once_strategy::TrueOnceStrategy},
};
use utils::constants::DEFAULT_SYMBOL;

pub async fn run_dummy() -> Result<()> {
    todo!()
}

pub async fn run_multiple_strategies() -> Result<()> {
    multiple_strategies::run().await
}

pub async fn run_actual_strategy() -> Result<()> {
    let strategy: Box<dyn TradingStrategy> = Box::new(KQ14::new());
    let interval = strategy.interval();
    let source = DataSource::Bybit;
    let net = NetVersion::Mainnet;

    println!("Starting strategy {} on interval {}", strategy, interval);

    // Initialize timeseries and indicators
    let mut ts = source
        .get_historical_data(DEFAULT_SYMBOL, &interval, strategy.min_length() + 300, &net)
        .await?;

    for indicator_type in strategy.required_indicators() {
        ts.add_indicator(indicator_type)?;
    }

    let ts_addr = ts.start();

    // Set up DB
    let db = DB::new().await?;
    let db_addr = db.start();

    // Create setup finder
    let setup_finder = SetupFinderBuilder::new()
        .strategy(strategy)
        .ts_addr(ts_addr.clone())
        .db_addr(db_addr.clone())
        .notifications_enabled(true)
        .live_trading_enabled(true)
        .source(source.clone())
        .build()?;

    setup_finder.start();

    // Start websocket client
    let mut wsclient = WebsocketClient::new(source, interval, net);
    wsclient.add_observer(ts_addr);
    wsclient.start();

    // Run program indefinately
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

pub async fn run_true_once_buys() -> Result<()> {
    always_true::true_once().await
}

pub async fn run_true_always() -> Result<()> {
    always_true::true_always().await
}

pub async fn run_manual_candles() -> Result<()> {
    manual_candles::run().await
}

pub async fn run_ws_only() -> Result<()> {
    ws_only::run().await
}

pub async fn run_market_buy() -> Result<()> {
    let time = BybitRestApi::get_server_time().await?;

    println!("Time: {:#?}", time);

    let wallet = BybitRestApi::get_wallet_balance().await?;

    println!("Wallet: {:#?}", wallet);

    let buy = false;

    if buy {
        let balance: f64 = wallet.total_available_balance;
        let symbol = DEFAULT_SYMBOL;
        BybitRestApi::market_buy(symbol, balance * 0.5).await?;
    } else {
        BybitRestApi::market_sell_all(&wallet).await?;
    }

    Ok(())
}

pub async fn run_market_sell_all() -> Result<()> {
    let time = BybitRestApi::get_server_time().await?;

    println!("Time: {:#?}", time);

    let wallet = BybitRestApi::get_wallet_balance().await?;

    println!("Wallet: {:#?}", wallet);

    BybitRestApi::market_sell_all(&wallet).await?;

    Ok(())
}

pub async fn run_single_indicator() -> Result<()> {
    let (k_len, k_smoothing, d_smoothing) = Stochastic::krown_args().stochastic_res()?;
    let indicator_type = IndicatorType::Stochastic(k_len, k_smoothing, d_smoothing);

    let interval = Interval::Minute1;
    let source = DataSource::Bybit;
    let net = NetVersion::Mainnet;
    let needed_candles = k_len + k_smoothing + d_smoothing - 2;
    let mut ts = source
        .get_historical_data(DEFAULT_SYMBOL, &interval, needed_candles + 500, &net)
        .await?;

    indicator_type.populate_candles(&mut ts)?;
    println!("Ts:{:#?}", ts);

    let mut client = WebsocketClient::new(source, interval, net);
    let addr = ts.start();

    client.add_observer(addr);
    client.start();

    // TODO: Enable check for whether new setups have arisen from updated indicators
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

pub async fn run_strategy() -> Result<()> {
    let strategy: Box<dyn TradingStrategy> = Box::new(TrueOnceStrategy::new());
    let interval = Interval::Minute1;
    let source = DataSource::Bybit;
    let net = NetVersion::Mainnet;

    // Initialize timeseries and indicators
    let mut ts = source
        .get_historical_data(DEFAULT_SYMBOL, &interval, strategy.min_length() + 300, &net)
        .await?;

    // ts.save_to_local(&source).await?;
    // let ts = source.load_local_data(symbol, &interval).await?;

    for indicator_type in strategy.required_indicators() {
        ts.add_indicator(indicator_type)?;
    }

    let ts_addr = ts.start();

    // Create setup finder and subscribe to timeseries
    let setup_finder = SetupFinderBuilder::new()
        .strategy(strategy)
        .ts_addr(ts_addr.clone())
        .notifications_enabled(true)
        .live_trading_enabled(false)
        .source(source.clone())
        .build()?;
    setup_finder.start();

    // Start websocket client
    let mut wsclient = WebsocketClient::new(source, interval, net);
    wsclient.add_observer(ts_addr);
    wsclient.start();

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

pub async fn run_double_strategies() -> Result<()> {
    let short_strategy: Box<dyn TradingStrategy> = Box::new(RsiBasic::new_args(
        14,
        45.0,
        55.0,
        StrategyOrientation::Short,
    ));
    let long_strategy: Box<dyn TradingStrategy> = Box::new(RsiBasic::new_args(
        14,
        45.0,
        55.0,
        StrategyOrientation::Long,
    ));
    let interval = Interval::Minute1;
    let source = DataSource::Bybit;
    let net = NetVersion::Mainnet;

    // Initialize timeseries and indicators
    let mut ts = source
        .get_historical_data(
            DEFAULT_SYMBOL,
            &interval,
            long_strategy.min_length() + 300,
            &net,
        )
        .await?;
    // ts.save_to_local(&source).await?;
    // let ts = source.load_local_data(symbol, &interval).await?;

    for indicator_type in long_strategy.required_indicators() {
        ts.add_indicator(indicator_type)?;
    }

    let ts_addr = ts.start();

    // Create setup finder and subscribe to timeseries
    let long_setup_finder = SetupFinderBuilder::new()
        .strategy(long_strategy)
        .ts_addr(ts_addr.clone())
        .notifications_enabled(true)
        .build()?;
    let short_setup_finder = SetupFinderBuilder::new()
        .strategy(short_strategy)
        .ts_addr(ts_addr.clone())
        .notifications_enabled(true)
        .build()?;

    let long_sf_addr = long_setup_finder.start();
    let short_sf_addr = short_setup_finder.start();

    let long_payload = TSSubscribePayload {
        observer: long_sf_addr.recipient(),
    };

    let short_payload = TSSubscribePayload {
        observer: short_sf_addr.recipient(),
    };

    ts_addr.do_send(long_payload);
    ts_addr.do_send(short_payload);

    // Start websocket client
    let mut wsclient = WebsocketClient::new(source, interval, net);
    wsclient.add_observer(ts_addr);
    wsclient.start();

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

pub async fn run_historical() -> Result<()> {
    dotenv().ok();

    // Get TimeSeries data
    let source = DataSource::Bybit;
    let interval = Interval::Day1;
    let net = NetVersion::Mainnet;
    let mut ts = source
        .get_historical_data(DEFAULT_SYMBOL, &interval, 19, &net)
        .await?;

    RSI::populate_candles(&mut ts)?;

    println!("Candles:{:#?}", ts.candles);

    Ok(())
}

pub async fn run_local() -> Result<()> {
    let mut ts = local::read_dummy_data("src/utils/data/atr_dummy_data.csv").await?;
    ATR::populate_candles(&mut ts)?;

    println!("{:#?}", ts);

    Ok(())
}

pub async fn run_setup_finder() -> Result<()> {
    let strategy: Box<dyn TradingStrategy> = Box::new(RsiBasic::new());
    let ts = TimeSeries::dummy();
    let ts = ts.start();

    SetupFinderBuilder::new()
        .strategy(strategy)
        .ts_addr(ts.clone())
        .notifications_enabled(true)
        .build()?
        .start();

    let mut i = 0.0;
    loop {
        sleep(Duration::from_secs(2)).await;
        let candle = Candle::dummy_from_val(i * 10.0);

        let payload = WebsocketPayload {
            ok: true,
            message: None,
            candle: Some(candle),
        };

        ts.do_send(payload);
        i += 1.0;
    }
}

pub async fn run_manual_setups() -> Result<()> {
    let mut ts = TimeSeriesBuilder::new()
        .symbol(DEFAULT_SYMBOL.to_string())
        .interval(Interval::Day1)
        .build();
    RSI::populate_candles(&mut ts)?;

    let strategy: Box<dyn TradingStrategy> = Box::new(RsiBasic::new());
    let ts = ts.start();

    SetupFinderBuilder::new()
        .strategy(strategy)
        .ts_addr(ts.clone())
        .notifications_enabled(true)
        .build()?
        .start();

    let mut candles = Candle::dummy_data(20, "positive", 100.0);
    candles.extend(Candle::dummy_data(15, "negative", 300.0));

    for candle in candles {
        sleep(Duration::from_millis(500)).await;

        let payload = WebsocketPayload {
            ok: true,
            message: None,
            candle: Some(candle),
        };

        ts.do_send(payload);
    }

    Ok(())
}

pub async fn run_strategy_tester() -> Result<()> {
    // Get TimeSeries data
    let source = DataSource::Bybit;
    let mut strategy: Box<dyn TradingStrategy> = Box::new(JB2::new());
    let interval = strategy.interval();
    let net = NetVersion::Mainnet;

    println!("Fetching Timeseries data.");
    let mut ts = source
        .get_historical_data(DEFAULT_SYMBOL, &interval, 20000, &net)
        .await?;

    // Calculate indicators for TimeSeries
    // Implement Strategy to analyze TimeSeries

    println!("Starting indicator calculations.");
    for indicator in strategy.required_indicators() {
        println!("Populating indicator: {:#?}", indicator);
        indicator.populate_candles(&mut ts)?;
    }

    let result = StrategyTester::test_strategy(&mut strategy, &ts.candles[300..])?;

    println!("{:#?}", result);

    Ok(())
}
