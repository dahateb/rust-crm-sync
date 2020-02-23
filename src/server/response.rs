use crate::util::Message;
use crossbeam_channel::Sender;
use std::collections::HashMap;
use warp::reply::{json, with_status};
use futures_03::{StreamExt, SinkExt};
use warp::ws::{WebSocket};
use std::time::{Duration};
use tokio_02::time::{self};
use crossbeam_channel::Receiver;

pub static INDEX: &[u8] = b"<h4>===> SYNC API <===</h4>";

pub fn response_for_sender(
    params: HashMap<String, String>,
    route: String,
    sender: Sender<(String, usize)>,
    status: warp::http::StatusCode,
) -> impl warp::Reply {
    let number = if let Some(n) = params.get("number") {
        if let Ok(v) = n.parse::<usize>() {
            v
        } else {
            return with_status(
                json(&json!({"status":"Error"})),
                warp::http::StatusCode::BAD_REQUEST,
            );
        }
    } else {
        return with_status(
            json(&json!({"status":"Error"})),
            warp::http::StatusCode::BAD_REQUEST,
        );
    };
    let _ = sender.send((route, number));
    with_status(json(&json!({"status":"OK"})), status)
}

pub async fn handle_ws(ws: WebSocket, receiver: Receiver<Box<dyn Message>>) {
    let (mut user_ws_tx, mut _user_ws_rx) = ws.split();

    tokio_02::task::spawn(async move{
        let mut interval = time::interval(Duration::from_millis(1000));
        'outer: loop {
            let i = interval.tick().await;
            while let Ok(to_message) = receiver.try_recv() {
                match user_ws_tx.send(warp::ws::Message::text(to_message.to_string())).await {
                    Ok(()) => println!("Send Message: {:?}", i),
                    Err(e) => {println!("Ws ended with: {}", e); break 'outer;}
                }
                let _ = user_ws_tx.flush().await; 
            } 
            match user_ws_tx.send(warp::ws::Message::text("{}")).await {
                Ok(()) => (),
                Err(e) => {println!("Ws ended with: {}", e); break 'outer;}
            }
        }
    });
}