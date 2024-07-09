use std::time::Duration;
use crate::{
    data_sources::datasource::DataSource,
    indicators::{
        bbwp::BBWP, ema::EMA, indicator::Indicator, indicator_type::IndicatorType,
        is_indicator::IsIndicator, pmar::PMAR, pmarp::PMARP, sma::SMA,
    },
    models::{
        candle::Candle,
        database::db::DB,
        interval::Interval,
        ma_type::MAType,
        message_payloads::{
            ts_subscribe_payload::TSSubscribePayload, websocket_payload::WebsocketPayload,
        },
        setups::setup_finder_builder::SetupFinderBuilder,
        timeseries_builder::TimeSeriesBuilder,
        traits::trading_strategy::TradingStrategy,
    },
    trading_strategies::private::kq_14::KQ14,
};
use actix::Actor;
use anyhow::Result;
use tokio::time::sleep;

pub async fn run() -> Result<()> {
    let (base_candle, entry_candle) = get_initial_candles();

    let mut ts = TimeSeriesBuilder::new()
        .symbol("BTCUSDT".to_string())
        .interval(Interval::Day1)
        .validate_candles_on_add(false)
        .build();

    ts.add_candle(&base_candle)?;
    let ts_addr = ts.start();

    let db = DB::new().await?;
    let db_addr = db.start();

    let sf = SetupFinderBuilder::new()
        .strategy(Box::new(KQ14::new()))
        .ts(ts_addr.clone())
        .source(DataSource::Dummy(1000))
        .db_addr(db_addr)
        .live_trading_enabled(true)
        .build()?;

    let sf_addr = sf.start();

    let ts_sub_payload = TSSubscribePayload {
        observer: sf_addr.recipient(),
    };

    ts_addr.do_send(ts_sub_payload);

    sleep(Duration::new(1, 0)).await;

    ts_addr.do_send(candle_payload(&entry_candle));

    sleep(Duration::new(2, 0)).await;

    let (len, lookback, ma_type) = PMARP::default_args().pmarp_res()?;
    let pmarp_type = IndicatorType::PMARP(len, lookback, ma_type);

    let mut next_candle = Candle::dyn_dummy_from_prev(&entry_candle, Interval::Day1);
    let next_pmarp = Indicator::PMARP(Some(PMARP::new(0.65, len, lookback)));
    next_candle
        .indicators
        .insert(pmarp_type.clone(), next_pmarp);
    let next_candle = fill_candle(next_candle);

    ts_addr.do_send(candle_payload(&next_candle));

    let mut exit_candle = Candle::dyn_dummy_from_prev(&next_candle, Interval::Day1);
    let exit_pmarp = Indicator::PMARP(Some(PMARP::new(0.8, len, lookback)));
    exit_candle.indicators.insert(pmarp_type, exit_pmarp);

    sleep(Duration::new(2, 0)).await;

    ts_addr.do_send(candle_payload(&exit_candle));

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

fn candle_payload(candle: &Candle) -> WebsocketPayload {
    WebsocketPayload {
        candle: Some(candle.clone()),
        ok: true,
        message: None,
    }
}

fn get_initial_candles() -> (Candle, Candle) {
    let (ema_21_type, ema_55_type, pmar_type, bbwp_type) = get_indicator_types();
    let (ema_21, ema_55, pmar, bbwp_prev, bbwp_curr) = get_indicators();

    let base_candle = Candle::dummy_data(1, "alternating", 1000.0);
    let mut base_candle = base_candle[0].clone();

    base_candle.indicators.insert(
        bbwp_type.clone(),
        Indicator::BBWP(Some(bbwp_prev.as_bbwp().expect("Expected bbwp."))),
    );

    let entry_candle = Candle::dummy_data(1, "alternating", 1000.0);
    let mut entry_candle = entry_candle[0].clone();

    entry_candle.indicators.insert(
        ema_21_type,
        Indicator::EMA(Some(ema_21.as_ema().expect("Expected ema."))),
    );
    entry_candle.indicators.insert(
        ema_55_type,
        Indicator::EMA(Some(ema_55.as_ema().expect("Expected ema."))),
    );
    entry_candle.indicators.insert(
        pmar_type,
        Indicator::PMAR(Some(pmar.as_pmar().expect("Expected pmar."))),
    );
    entry_candle.indicators.insert(
        bbwp_type,
        Indicator::BBWP(Some(bbwp_curr.as_bbwp().expect("Expected bbwp."))),
    );

    (base_candle, entry_candle)
}

fn get_indicator_types() -> (IndicatorType, IndicatorType, IndicatorType, IndicatorType) {
    let ema_21_type = IndicatorType::EMA(21);

    let ema_55_type = IndicatorType::EMA(55);

    let pmar_type = IndicatorType::PMAR(55, MAType::EMA);

    let bbwp_type = IndicatorType::BBWP(13, 252);

    (ema_21_type, ema_55_type, pmar_type, bbwp_type)
}

fn get_indicators() -> (Indicator, Indicator, Indicator, Indicator, Indicator) {
    let ema_21 = EMA {
        value: 950.0,
        len: 21,
    };
    let ema_55 = EMA {
        value: 940.0,
        len: 55,
    };
    let pmar = PMAR {
        value: 0.90,
        len: 55,
        ma: Some(0.90),
    };
    let bbwp_prev = BBWP {
        value: 0.1,
        len: 13,
        lookback: 252,
        sma: Some(SMA { value: 0.1, len: 5 }),
    };
    let bbwp_curr = BBWP {
        value: 0.07,
        len: 13,
        lookback: 252,
        sma: Some(SMA {
            value: 0.085,
            len: 5,
        }),
    };

    (
        Indicator::EMA(Some(ema_21)),
        Indicator::EMA(Some(ema_55)),
        Indicator::PMAR(Some(pmar)),
        Indicator::BBWP(Some(bbwp_prev)),
        Indicator::BBWP(Some(bbwp_curr)),
    )
}

fn fill_candle(mut candle: Candle) -> Candle {
    let (ema_21_type, ema_55_type, pmar_type, bbwp_type) = get_indicator_types();
    let (ema_21, ema_55, pmar, _bbwp_prev, bbwp_curr) = get_indicators();

    candle.indicators.insert(
        ema_21_type,
        Indicator::EMA(Some(ema_21.as_ema().expect("Expected ema."))),
    );
    candle.indicators.insert(
        ema_55_type,
        Indicator::EMA(Some(ema_55.as_ema().expect("Expected ema."))),
    );
    candle.indicators.insert(
        pmar_type,
        Indicator::PMAR(Some(pmar.as_pmar().expect("Expected pmar."))),
    );
    candle.indicators.insert(
        bbwp_type,
        Indicator::BBWP(Some(bbwp_curr.as_bbwp().expect("Expected bbwp."))),
    );

    candle
}
