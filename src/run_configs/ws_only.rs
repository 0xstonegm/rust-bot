use std::time::Duration;
use actix::Actor;
use anyhow::Result;
use tokio::time::sleep;
use crate::{data_sources::datasource::DataSource, models::{net_version::NetVersion, websockets::wsclient::WebsocketClient}};

pub async fn run() -> Result<()> {
    let source = DataSource::Bybit;
    let interval = crate::models::interval::Interval::Minute1;
    let net = NetVersion::Mainnet;

    let ws = WebsocketClient::new(source, interval, net);

    ws.start();

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
