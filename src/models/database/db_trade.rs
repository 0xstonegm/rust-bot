use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DBTrade {
    pub id: Uuid,
    pub symbol: String,
    pub interval: String,
    pub orientation: String,
    pub trading_strategy: String,
    pub resolution_strategy: String,
    pub data_source: String,
    pub entered_at: DateTime<Utc>,
    pub exited_at: Option<DateTime<Utc>>,
    pub bars_in_trade: Option<i32>,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub quantity: f64,
    pub dollar_value: f64,
    pub entry_fee: f64,
    pub exit_fee: Option<f64>,
    pub comments: Option<String>,
}
