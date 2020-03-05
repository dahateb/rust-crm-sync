use crossbeam_channel::Sender;
use std::collections::HashMap;
use warp::reply::{json, with_status};


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