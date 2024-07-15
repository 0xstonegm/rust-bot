use crate::{
    data_sources::datasource::DataSource,
    models::{
        database::db::DB, interval::Interval, net_version::NetVersion,
        setups::setup_finder_builder::SetupFinderBuilder,
        traits::trading_strategy::TradingStrategy, websockets::wsclient::WebsocketClient,
    },
    trading_strategies::public::true_once_strategy::TrueOnceStrategy,
    utils::constants::DEFAULT_SYMBOL,
};
use actix::Actor;
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

pub async fn true_once() -> Result<()> {
    let strategy: Box<dyn TradingStrategy> = Box::new(TrueOnceStrategy::new());
    let interval = Interval::Minute1;
    // let source = DataSource::Bybit;
    let source = DataSource::Dummy(4000);
    let net = NetVersion::Mainnet;

    // Initialize timeseries and indicators
    let mut ts = source
        .get_historical_data(DEFAULT_SYMBOL, &interval, strategy.min_length() + 300, &net)
        .await?;
    ts.validate_candles_on_add = false;

    for indicator_type in strategy.required_indicators() {
        ts.add_indicator(indicator_type)?;
    }

    let ts_addr = ts.start();

    // Set up DB
    let db = DB::new().await?;
    let db_addr = db.start();

    // Build and start SetupFinder
    SetupFinderBuilder::new()
        .strategy(strategy)
        .ts_addr(ts_addr.clone())
        .notifications_enabled(false)
        .live_trading_enabled(true)
        .source(source.clone())
        .only_trigger_once(true)
        .db_addr(db_addr)
        .build()?
        .start();

    // Start websocket client
    let mut wsclient = WebsocketClient::new(source, interval, net);
    wsclient.add_observer(ts_addr);
    wsclient.start();

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

pub async fn true_always() -> Result<()> {
    let strategy: Box<dyn TradingStrategy> = Box::new(TrueOnceStrategy::new());
    let interval = Interval::Minute1;
    // let source = DataSource::Bybit;
    let source = DataSource::Dummy(4000);
    let net = NetVersion::Mainnet;

    // Initialize timeseries and indicators
    let mut ts = source
        .get_historical_data(DEFAULT_SYMBOL, &interval, strategy.min_length() + 300, &net)
        .await?;
    ts.validate_candles_on_add = false;

    for indicator_type in strategy.required_indicators() {
        ts.add_indicator(indicator_type)?;
    }

    let ts_addr = ts.start();

    // Set up DB
    let db = DB::new().await?;
    let db_addr = db.start();

    // Create setup finder and subscribe to timeseries
    let setup_finder = SetupFinderBuilder::new()
        .strategy(strategy)
        .ts_addr(ts_addr.clone())
        .notifications_enabled(false)
        .live_trading_enabled(true)
        .source(source.clone())
        .only_trigger_once(false)
        .db_addr(db_addr)
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
