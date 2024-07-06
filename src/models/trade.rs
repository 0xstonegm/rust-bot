use crate::{
    data_sources::datasource::DataSource,
    models::{
        database::{db::DB, db_trade::DBTrade},
        interval::Interval,
        message_payloads::finish_db_trade_payload::FinishDBTradePayload,
        message_payloads::{
            candle_added_payload::CandleAddedPayload,
            create_db_trade_payload::CreateDBTradePayload, ping_payload::PingPayload,
            request_latest_candles_payload::RequestLatestCandlesPayload, stop_payload::StopPayload,
        },
        setups::setup::Setup,
        timeseries::TimeSeries,
    },
    resolution_strategies::{
        is_resolution_strategy::IsResolutionStrategy, resolution_strategy::ResolutionStrategy,
    },
    TradingStrategy,
};
use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, WrapFuture};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

#[derive(Debug)]
pub struct Trade {
    pub id: Uuid,
    pub setup: Setup,
    pub quantity: f64,
    pub dollar_value: f64,
    pub source: DataSource,
    pub notifications_enabled: bool,
    pub trading_enabled: bool,
    pub resolution_strategy: ResolutionStrategy,
    pub trading_strategy: Box<dyn TradingStrategy>,
    pub timeseries: Addr<TimeSeries>,
    pub db_addr: Addr<DB>,
}

impl Actor for Trade {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.resolution_strategy
            .set_initial_values(&self.setup)
            .expect("Unable to set initial values resolution strategy when starting Trade.");

        let id = self.id.clone().clone();
        let source = self.source.clone();
        let symbol = self.setup.symbol.clone();
        let dollar_value = self.dollar_value.clone();
        let interval = self.setup.interval.to_string().clone();
        let orientation = self.setup.orientation.to_string().clone();
        let trading_strategy = format!("{}", self.trading_strategy);
        let resolution_strategy = format!("{}", self.resolution_strategy);
        let data_source = self.source.to_string().clone();
        let entered_at = self.setup.candle.timestamp.clone();
        let entry_price = self.setup.candle.close.clone();
        let quantity = self.quantity.clone();
        let db_addr = self.db_addr.clone();

        let fut = async move {
            let res = source.enter_trade(&symbol, dollar_value).await;

            match res {
                Ok(_) => println!("Successfully entered trade"),
                Err(e) => println!("Unable to enter trade, error: {:#?}", e),
            }

            // Create DBTrade
            let db_trade = DBTrade {
                id,
                symbol,
                interval,
                orientation,
                trading_strategy,
                resolution_strategy,
                data_source,
                entered_at,
                exited_at: None,
                bars_in_trade: None,
                entry_price,
                exit_price: None,
                quantity,
                dollar_value,
                entry_fee: 0.0,
                exit_fee: None,
                comments: None,
            };
            let payload = CreateDBTradePayload { db_trade };

            db_addr.do_send(payload)

            // TODO: Setup system that manages scenario where Trade is unable
            // to enter. Send notifications and or stop service.
        };

        ctx.spawn(fut.into_actor(self));
    }
}

impl Handler<StopPayload> for Trade {
    type Result = ();

    fn handle(&mut self, _msg: StopPayload, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

impl Handler<PingPayload> for Trade {
    type Result = ();

    fn handle(&mut self, _msg: PingPayload, _ctx: &mut Self::Context) -> Self::Result {}
}

impl Handler<CandleAddedPayload> for Trade {
    type Result = ();

    fn handle(&mut self, msg: CandleAddedPayload, ctx: &mut Self::Context) -> Self::Result {
        let resolution_strategy = self.resolution_strategy.clone();
        let tp_candles_needed = resolution_strategy.n_candles_take_profit();
        let sl_candles_needed = resolution_strategy.n_candles_stop_loss();
        let orientation = self.setup.orientation.clone();
        let ts_addr = self.timeseries.clone();
        let source = self.source.clone();
        let symbol = self.setup.symbol.clone();
        let self_addr = ctx.address();
        let id = self.id;
        let entered_at = self.setup.candle.timestamp.clone();
        let interval = self.setup.interval.clone();
        let db_addr = self.db_addr.clone();
        let candle = msg.candle.clone();

        // Multiply to avoid scenarios where quantity is slightly larger than
        // account balance (caused by sudden price changes in time between
        // account balance is checked and initial buy is performed).
        let quantity = self.quantity * 0.99;

        let payload = RequestLatestCandlesPayload {
            n: tp_candles_needed.max(sl_candles_needed),
        };

        let fut = async move {
            let candle_response = ts_addr
                .send(payload)
                .await
                .expect("Unable to fetch timeseries data in Trade.")
                .expect("Unable to parse LatestCandleResponse in Trade.");

            let end = candle_response.candles.len();

            let tp_candles = &candle_response.candles[end - tp_candles_needed..end];
            let take_profit_reached = resolution_strategy
                .take_profit_reached(&orientation, tp_candles)
                .expect("Unable to perform take-profit check in Active Trade");

            let sl_candles = &candle_response.candles[end - sl_candles_needed..end];
            let stop_loss_reached = resolution_strategy
                .stop_loss_reached(&orientation, sl_candles)
                .expect("Unable to perform stop-loss check in Active Trade");

            if take_profit_reached || stop_loss_reached {
                let res = source.exit_trade(&symbol, quantity).await;

                match res {
                    Ok(_) => println!("Trade successfully exited!"),
                    Err(e) => println!("Trade exit failed with error: {:#?}", e),
                }

                let exited_at = candle.timestamp.clone();

                println!("Exited at: {:#?}", exited_at);
                let exit_price = candle.close.clone();
                let bars_in_trade = get_bars_in_trade(&entered_at, &exited_at, &interval);

                let finish_payload = FinishDBTradePayload {
                    id,
                    exited_at,
                    bars_in_trade,
                    exit_price,
                };

                db_addr.do_send(finish_payload);
                self_addr.do_send(StopPayload);

                // TODO: Handle/notify user in case selling was unsuccessful.
            }
        };

        ctx.spawn(fut.into_actor(self));
    }
}

fn get_bars_in_trade(
    entered_at: &DateTime<Utc>,
    exited_at: &DateTime<Utc>,
    interval: &Interval,
) -> i32 {
    // Add seconds to ensure proper number of bars in scenarios where difference
    // between entered_at and exited_at does not exactly match the interval.
    let duration = (*exited_at - *entered_at) + Duration::seconds(10);
    let total_seconds = duration.num_seconds();

    let bars = match interval {
        Interval::Minute1 => total_seconds / 60,
        Interval::Minute5 => total_seconds / (5 * 60),
        Interval::Minute15 => total_seconds / (15 * 60),
        Interval::Minute30 => total_seconds / (30 * 60),
        Interval::Hour1 => total_seconds / (60 * 60),
        Interval::Hour4 => total_seconds / (4 * 60 * 60),
        Interval::Hour12 => total_seconds / (12 * 60 * 60),
        Interval::Day1 => total_seconds / (24 * 60 * 60),
        Interval::Day5 => total_seconds / (5 * 24 * 60 * 60),
        Interval::Week1 => total_seconds / (7 * 24 * 60 * 60),
    };

    bars as i32
}
