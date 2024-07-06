use actix::Message;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct FinishDBTradePayload {
    pub id: Uuid,
    pub exited_at: DateTime<Utc>,
    pub bars_in_trade: i32,
    pub exit_price: f64,
}

impl Message for FinishDBTradePayload {
    type Result = ();
}
