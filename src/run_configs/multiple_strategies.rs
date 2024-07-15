use crate::{
    data_sources::datasource::DataSource,
    indicators::indicator_type::IndicatorType,
    models::{
        database::db::DB, interval::Interval, net_version::NetVersion,
        setups::setup_finder_builder::SetupFinderBuilder, timeseries::TimeSeries,
        traits::trading_strategy::TradingStrategy, websockets::wsclient::WebsocketClient,
    },
    trading_strategies::private::{kq_12::KQ12, kq_14::KQ14},
    utils::constants::DEFAULT_SYMBOL,
};
use actix::{Actor, Addr};
use anyhow::Result;
use futures_util::future::try_join_all;
use indexmap::IndexSet;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;

pub async fn run() -> Result<()> {
    // Define active strategies, this should be the only input
    let strats: Vec<Box<dyn TradingStrategy>> = vec![Box::new(KQ14::new()), Box::new(KQ12::new())];

    start(strats).await?;

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn start(mut strats: Vec<Box<dyn TradingStrategy>>) -> Result<Vec<Addr<TimeSeries>>> {
    let source = DataSource::Bybit;
    let net = NetVersion::Mainnet;

    // Get indexset of all used intervals and min length needed for timeseries
    let (intervals, min_len) = strats.iter().fold(
        (IndexSet::new(), 0),
        |(mut intervals, mut min_len), strat| {
            if !intervals.contains(&strat.interval()) {
                intervals.insert(strat.interval());
            }

            if strat.min_length() > min_len {
                min_len = strat.min_length();
            }

            (intervals, min_len)
        },
    );

    // Get historical data and create base timeseries
    let handles: Vec<_> = intervals
        .iter()
        .map(|interval| source.get_historical_data(DEFAULT_SYMBOL, interval, min_len + 300, &net))
        .collect();

    let mut timeseries = try_join_all(handles).await?;

    // Create indicator map to track which intevals require which indicators
    let mut indicator_map: HashMap<Interval, IndexSet<IndicatorType>> = HashMap::new();

    for strat in strats.iter() {
        let interval = strat.interval();

        for indicator in strat.required_indicators().iter() {
            if let Some(set) = indicator_map.get_mut(&interval) {
                if !set.contains(indicator) {
                    set.insert(indicator.clone());
                }
            } else {
                let mut set = IndexSet::new();
                set.insert(indicator.clone());
                indicator_map.insert(interval.clone(), set);
            }
        }
    }

    // TODO: Parallelize here
    // Populate indicators for TimeSeries using the indicator map
    for ts in timeseries.iter_mut() {
        let indicators = indicator_map
            .get(&ts.interval)
            .expect("Expected interval to have matching indicators.");

        for indicator_type in indicators {
            ts.add_indicator(*indicator_type)
                .expect("Unable to add indicator to TimeSeries.");
        }
    }

    // Find indexes matching strategies to now populated timeseries
    let strat_indices: Vec<(&mut Box<dyn TradingStrategy>, usize)> = strats
        .iter_mut()
        .map(|strat| {
            let interval = strat.interval();
            let ts_idx = timeseries
                .iter()
                .position(|ts| ts.interval == interval)
                .expect("Expected there to exist a timeseries with correct interval.");

            (strat, ts_idx)
        })
        .collect();

    // Start timeseries listening via websockets for new data
    let ts_addrs: Vec<Addr<TimeSeries>> = timeseries
        .into_iter()
        .map(|ts| {
            let mut wsclient =
                WebsocketClient::new(source.clone(), ts.interval.clone(), net.clone());
            let ts_addr = ts.start();

            wsclient.add_observer(ts_addr.clone());
            wsclient.start();
            ts_addr
        })
        .collect();

    // Start DB connection
    let db = DB::new().await?;
    let db_addr = db.start();

    // Start setupfinders
    strat_indices.iter().for_each(|(strat, i)| {
        println!(
            "Starting strategy {} on interval {}",
            strat,
            strat.interval()
        );
        let sf = SetupFinderBuilder::new()
            .strategy(strat.clone_box())
            .ts_addr(ts_addrs[*i].clone())
            .db_addr(db_addr.clone())
            .notifications_enabled(true)
            .live_trading_enabled(true)
            .source(source.clone())
            .build()
            .expect("Expected to successfully build SetupFinder.");

        sf.start();
    });

    Ok(ts_addrs)
}
