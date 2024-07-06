use crate::{
    data_sources::datasource::DataSource,
    models::{
        database::db::DB, setups::setup::Setup, strategy_orientation::StrategyOrientation,
        timeseries::TimeSeries, trade::Trade, traits::trading_strategy::TradingStrategy,
    },
    resolution_strategies::resolution_strategy::ResolutionStrategy,
};
use actix::Addr;
use anyhow::{anyhow, Result};
use uuid::Uuid;

#[derive(Debug)]
pub struct TradeBuilder {
    pub id: Uuid,
    pub setup: Option<Setup>,
    pub quantity: Option<f64>,
    pub dollar_value: Option<f64>,
    pub source: Option<DataSource>,
    pub notifications_enabled: bool,
    pub trading_enabled: bool,
    pub resolution_strategy: Option<ResolutionStrategy>,
    pub orientation: Option<StrategyOrientation>,
    pub timeseries_addr: Option<Addr<TimeSeries>>,
    pub trading_strategy: Option<Box<dyn TradingStrategy>>,
    pub db_addr: Option<Addr<DB>>,
}

impl TradeBuilder {
    pub fn new() -> Self {
        TradeBuilder {
            id: Uuid::new_v4(),
            setup: None,
            quantity: None,
            dollar_value: None,
            source: None,
            notifications_enabled: false,
            trading_enabled: false,
            resolution_strategy: None,
            orientation: None,
            timeseries_addr: None,
            trading_strategy: None,
            db_addr: None,
        }
    }

    pub fn quantity(mut self, quantity: f64) -> Self {
        self.quantity = Some(quantity);
        self
    }

    pub fn dollar_value(mut self, dollar_value: f64) -> Self {
        self.dollar_value = Some(dollar_value);
        self
    }

    pub fn source(mut self, source: DataSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn notifications_enabled(mut self, notifications_enabled: bool) -> Self {
        self.notifications_enabled = notifications_enabled;
        self
    }

    pub fn trading_enabled(mut self, trading_enabled: bool) -> Self {
        self.trading_enabled = trading_enabled;
        self
    }

    pub fn resolution_strategy(mut self, resolution_strategy: ResolutionStrategy) -> Self {
        self.resolution_strategy = Some(resolution_strategy);
        self
    }

    pub fn orientation(mut self, orientation: StrategyOrientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    pub fn timeseries_addr(mut self, timeseries: Addr<TimeSeries>) -> Self {
        self.timeseries_addr = Some(timeseries);
        self
    }

    pub fn setup(mut self, setup: Setup) -> Self {
        self.setup = Some(setup);
        self
    }

    pub fn db_addr(mut self, db_addr: Addr<DB>) -> Self {
        self.db_addr = Some(db_addr);
        self
    }

    #[allow(dead_code)]
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn trading_strategy(mut self, trading_strategy: Box<dyn TradingStrategy>) -> Self {
        self.trading_strategy = Some(trading_strategy);
        self
    }

    pub fn build(&self) -> Result<Trade> {
        let notifications_enabled = self.notifications_enabled;
        let trading_enabled = self.trading_enabled;
        let setup = self
            .setup
            .clone()
            .ok_or(anyhow!("Setup is required to build Trade."))?;
        let quantity = self
            .quantity
            .clone()
            .ok_or(anyhow!("Quantity is required to build Trade."))?;
        let dollar_value = self
            .dollar_value
            .clone()
            .ok_or(anyhow!("Dollar value is required to build Trade."))?;
        let source = self
            .source
            .clone()
            .ok_or(anyhow!("DataSource is required to build Trade."))?;
        let resolution_strategy = self
            .resolution_strategy
            .clone()
            .ok_or(anyhow!("Resolution strategy is required to build Trade."))?;
        let timeseries = self
            .timeseries_addr
            .clone()
            .ok_or(anyhow!("TimeSeries is required to build Trade."))?;

        let db_addr = self
            .db_addr
            .clone()
            .ok_or(anyhow!("DB Address is required to build Trade."))?;

        let trading_strategy = self
            .trading_strategy
            .as_ref()
            .ok_or(anyhow!("Trading Strategy is required to build Trade."))?
            .clone_box();

        let trade = Trade {
            id: self.id,
            setup,
            quantity,
            dollar_value,
            source,
            notifications_enabled,
            trading_enabled,
            resolution_strategy,
            timeseries,
            trading_strategy,
            db_addr,
        };

        Ok(trade)
    }
}
