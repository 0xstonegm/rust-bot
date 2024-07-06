use crate::models::database::db_trade::DBTrade;
use actix::Message;

#[derive(Debug, Clone)]
pub struct CreateDBTradePayload {
    pub db_trade: DBTrade,
}

impl Message for CreateDBTradePayload {
    type Result = ();
}
