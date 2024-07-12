use crate::{
    data_sources::bybit::ws::{
        incoming_message::{IncomingMessage, KlineResponse, Kline},
        outgoing_message::{OutgoingMessage, OutgoingMessageArg},
    },
    models::{
        interval::Interval, message_payloads::websocket_payload::WebsocketPayload,
        net_version::NetVersion, websockets::wsclient::WebsocketClient,
    },
};
use actix::{spawn, Addr};
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    select,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
    time::{sleep, Duration},
    try_join,
};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::{Error, Message};

pub struct BybitWebsocketApi {
    client: Addr<WebsocketClient>,
    interval: Interval,
}

impl BybitWebsocketApi {
    pub fn new(client: &Addr<WebsocketClient>, interval: Interval) -> Self {
        Self {
            client: client.clone(),
            interval,
        }
    }

    pub async fn connect(&mut self, net: &NetVersion) -> Result<()> {
        let url = match net {
            NetVersion::Testnet => "wss://stream-testnet.bybit.com/v5/public/spot",
            NetVersion::Mainnet => "wss://stream.bybit.com/v5/public/spot",
        };

        let (mut ws_stream, _) = connect_async(url).await?;
        self.subscribe_to_kline(&mut ws_stream).await?;
        Self::send_ping(None, &mut ws_stream).await?;

        let (tx, rx) = channel(32);

        let ping_handle = Self::spawn_ping_task(tx).await;
        let message_handle = Self::spawn_message_task(self.client.clone(), ws_stream, rx).await;

        try_join!(ping_handle, message_handle)?;

        Ok(())
    }

    async fn subscribe_to_kline(
        &self,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<()> {
        let args = vec![OutgoingMessageArg {
            stream: "kline".to_string(),
            interval: self.interval_as_string()?,
            symbol: "BTCUSDT".to_string(),
        }];
        let sub = OutgoingMessage::new("subscribe", args);
        let json = sub.to_json();

        ws_stream.send(Message::Text(json)).await?;

        Ok(())
    }

    fn interval_as_string(&self) -> Result<String> {
        let interval = match self.interval {
            Interval::Minute1 => "1",
            Interval::Minute5 => "5",
            Interval::Minute15 => "15",
            Interval::Minute30 => "30",
            Interval::Hour1 => "60",
            Interval::Day1 => "D",
            Interval::Week1 => "W",
            _ => return Err(anyhow!("Bybit does not support this interval.")),
        };

        Ok(interval.to_string())
    }

    async fn send_ping(
        req_id: Option<String>,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<()> {
        let ping = OutgoingMessage::ping(req_id);
        let message = Message::Text(ping.to_json());

        ws_stream.send(message).await?;

        Ok(())
    }

    async fn spawn_ping_task(tx: Sender<&'static str>) -> JoinHandle<()> {
        spawn(async move {
            loop {
                sleep(Duration::from_secs(20)).await;
                tx.send("ping").await.expect("Failed to send ping");
            }
        })
    }

    async fn spawn_message_task(
        client: Addr<WebsocketClient>,
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        mut rx: Receiver<&'static str>,
    ) -> JoinHandle<()> {
        // Keep track of previous kline. Used to determine when a new candle
        // has been formed.
        let mut prev_kline: Option<Kline> = None;

        spawn(async move {
            loop {
                let ws_msg: Option<Result<Message, Error>> = ws_stream.next().await;
                select! {
                    _ = Self::handle_ping_message(&mut rx, &mut ws_stream) => {}
                    _ = Self::handle_websocket_message(&client, ws_msg, &mut prev_kline) => {}
                }
            }
        })
    }

    async fn handle_ping_message(
        rx: &mut Receiver<&'static str>,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<(), ()> {
        if let Some(_) = rx.recv().await {
            Self::send_ping(None, ws_stream).await.map_err(|e| {
                eprintln!("Error in Websockets: {:#?}", e);
                ()
            })
        } else {
            Ok(())
        }
    }

    async fn handle_websocket_message(
        client: &Addr<WebsocketClient>,
        ws_msg: Option<Result<Message, Error>>,
        prev_kline: &mut Option<Kline>
    ) -> Result<(), ()> {
        if let Some(msg) = ws_msg {
            Self::handle_message(client, msg, prev_kline).await.map_err(|e| {
                eprintln!("Error in Websockets: {:#?}", e);
                ()
            })
        } else {
            Ok(())
        }
    }

    async fn handle_message(
        client: &Addr<WebsocketClient>,
        msg: Result<Message, tungstenite::Error>,
        prev_kline: &mut Option<Kline>
    ) -> Result<()> {
        let msg = msg?;

        if let Message::Text(txt) = msg {
            // let v: serde_json::Value = serde_json::from_str(txt.as_str())?;
            let parsed: IncomingMessage = serde_json::from_str(txt.as_str())?;

            match parsed {
                IncomingMessage::Pong(_) => {}
                IncomingMessage::Subscribe(sub) => println!("Subscribe: {:#?}", sub),
                IncomingMessage::Kline(kline_response) => {
                    Self::handle_kline(kline_response, client, prev_kline).await?
                }
            }
        }

        Ok(())
    }

    async fn handle_kline(
        kline_response: KlineResponse,
        client: &Addr<WebsocketClient>,
        prev_kline: &mut Option<Kline>
    ) -> Result<()> {
        let kline = kline_response.get_kline()?;

        if prev_kline.is_none() {
            // Only triggers on initial candle on startup
            *prev_kline = Some(kline.clone());
        }

        let prev_kline = prev_kline
            .as_mut()
            .expect("Expected kline to exist.");

        if kline.start != prev_kline.start {
            // New candle has started forming, send previous candle which is 
            // now complete to client. 
            let candle = prev_kline.to_candle()?;
            // println!("Candle payload: {:#?}", candle);

            let payload = WebsocketPayload {
                ok: true,
                message: None,
                candle: Some(candle),
            };

            client.do_send(payload);
        }         

        *prev_kline = kline.clone();

        Ok(())
    }
}
