use crate::models::message_payloads::{
    create_db_trade_payload::CreateDBTradePayload, finish_db_trade_payload::FinishDBTradePayload,
};
use actix::{Actor, AsyncContext, Context, Handler, WrapFuture};
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, query, Pool, Postgres};
use std::env;

pub struct DB {
    pool: Pool<Postgres>,
}

impl DB {
    pub async fn new() -> Result<Self> {
        let db_url = env::var("DATABASE_URL")?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&db_url)
            .await?;

        Ok(Self { pool })
    }
}

impl Actor for DB {
    type Context = Context<Self>;
}

impl Handler<CreateDBTradePayload> for DB {
    type Result = ();

    fn handle(&mut self, msg: CreateDBTradePayload, ctx: &mut Context<Self>) {
        let pool = self.pool.clone();
        let fut = async move {
            let db_trade = msg.db_trade;

            let q = r#"
            insert into trades (
                id, symbol, interval, orientation, trading_strategy, 
                resolution_strategy, data_source, entered_at, entry_price, 
                quantity, dollar_value, entry_fee, comments
            ) values (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            )
            "#;

            let row = query(q)
                .bind(&db_trade.id)
                .bind(&db_trade.symbol)
                .bind(&db_trade.interval)
                .bind(&db_trade.orientation)
                .bind(&db_trade.trading_strategy)
                .bind(&db_trade.resolution_strategy)
                .bind(&db_trade.data_source)
                .bind(&db_trade.entered_at)
                .bind(&db_trade.entry_price)
                .bind(&db_trade.quantity)
                .bind(&db_trade.dollar_value)
                .bind(&db_trade.entry_fee)
                .bind(&db_trade.comments)
                .execute(&pool)
                .await;

            match row {
                Ok(_) => println!("DBTrade sucessfully inserted!"),
                Err(e) => println!("DBTrade failed to insert with error: {:#?}", e),
            };
        };

        ctx.spawn(fut.into_actor(self));
    }
}

impl Handler<FinishDBTradePayload> for DB {
    type Result = ();

    fn handle(&mut self, msg: FinishDBTradePayload, ctx: &mut Context<Self>) -> Self::Result {
        let id = msg.id;
        let exited_at = msg.exited_at;
        let bars_in_trade = msg.bars_in_trade;
        let exit_price = msg.exit_price;
        let pool = self.pool.clone();

        let q = r#"
            update trades 
            set 
                exited_at = $1,
                bars_in_trade = $2,
                exit_price = $3
            where 
                id = $4
            "#;

        let fut = async move {
            let res = query(q)
                .bind(&exited_at)
                .bind(&bars_in_trade)
                .bind(&exit_price)
                .bind(&id)
                .execute(&pool)
                .await;

            match res {
                Ok(_) => println!("DBTrade sucessfully updated!"),
                Err(e) => println!("DBTrade failed to update with error: {:#?}", e),
            };
        };

        ctx.spawn(fut.into_actor(self));
    }
}
