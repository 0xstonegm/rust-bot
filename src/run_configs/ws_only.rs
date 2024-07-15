use crate::{
    data_sources::datasource::DataSource,
    models::{net_version::NetVersion, websockets::wsclient::WebsocketClient},
};
use actix::Actor;
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

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
