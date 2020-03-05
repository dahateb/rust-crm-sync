use crate::sync::setup::Setup;
use crate::util::Message;
use crossbeam_channel::Receiver;
use std::time::Duration;
use tokio_02::time::{self};
use warp::reply::json;
use warp::ws::WebSocket;
use futures_03::{SinkExt, StreamExt};

pub async fn handle_setup_list(setup: Setup) -> Result<impl warp::Reply, warp::Rejection> {
    let res = tokio_02::task::spawn_blocking(move || {
        let print_func = |obj: (u32, &String, u32, usize)| {
            json!({
                "num":  obj.0,
                "name":  obj.1,
                "count":  obj.2,
                "num_fields": obj.3
            })
        };
        setup.list_db_objects(print_func).unwrap()
    })
    .await;
    Ok(json(&res.unwrap()))
}

pub async fn handle_salesforce_get_list(setup: Setup) -> Result<impl warp::Reply, warp::Rejection> {
    let res = tokio_02::task::spawn_blocking(move || {
        let print_func = |obj: (u32, &String, &String, bool, bool, bool)| {
            json!({
                "num":  obj.0,
                "name":  obj.1,
                "label": obj.2,
                "custom_setting": obj.3,
                "createable":  obj.4,
                "synched": obj.5
            })
        };
        setup.list_salesforce_objects(print_func).unwrap()
    })
    .await;
    Ok(json(&res.unwrap()))
}

pub async fn handle_ws(ws: WebSocket, receiver: Receiver<Box<dyn Message>>) {
    let (mut user_ws_tx, mut _user_ws_rx) = ws.split();

    tokio_02::task::spawn(async move {
        info!("Starting websocket ...");
        let mut interval = time::interval(Duration::from_millis(1000));
        'outer: loop {
            let i = interval.tick().await;
            while let Ok(to_message) = receiver.try_recv() {
                match user_ws_tx
                    .send(warp::ws::Message::text(to_message.to_string()))
                    .await
                {
                    Ok(()) => debug!("Send Message: {:?}", i),
                    Err(e) => {
                        info!("Ws ended with: {}", e);
                        break 'outer;
                    }
                }
                let _ = user_ws_tx.flush().await;
            }
            match user_ws_tx.send(warp::ws::Message::text("{}")).await {
                Ok(()) => (),
                Err(e) => {
                    info!("Ws ended with: {}", e);
                    break 'outer;
                }
            }
        }
    });
}