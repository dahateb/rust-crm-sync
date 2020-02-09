use crossbeam_channel::Receiver;
use futures::prelude::*;
//use futures::{future, stream};
use crate::util::Message as SyncMessage;
use hyper::header::*;
use hyper::{Body, Request, Response, StatusCode, Version};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;

pub fn mux(req: Request<Body>, receiver: Receiver<Box<dyn SyncMessage>>) -> Response<Body> {
    // All of this stuff should be done by a helper in the websocket library.
    fn convert_key(input: &[u8]) -> String {
        const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
        let mut digest = sha1::Sha1::new();
        digest.update(input);
        digest.update(WS_GUID);
        base64::encode(&digest.digest().bytes())
    }
    fn connection_has(value: &HeaderValue, needle: &str) -> bool {
        if let Ok(v) = value.to_str() {
            v.split(',').any(|s| s.trim().eq_ignore_ascii_case(needle))
        } else {
            false
        }
    }
    let is_http_11 = req.version() == Version::HTTP_11;
    let is_upgrade = req
        .headers()
        .get(CONNECTION)
        .map_or(false, |v| connection_has(v, "upgrade"));
    let is_websocket_upgrade = req
        .headers()
        .get(UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map_or(false, |v| v.eq_ignore_ascii_case("websocket"));
    let is_websocket_version_13 = req
        .headers()
        .get(SEC_WEBSOCKET_VERSION)
        .and_then(|v| v.to_str().ok())
        .map_or(false, |v| v == "13");
    if !is_http_11 || !is_upgrade || !is_websocket_upgrade || !is_websocket_version_13 {
        return Response::builder()
            .status(StatusCode::UPGRADE_REQUIRED)
            .header(SEC_WEBSOCKET_VERSION, "13")
            .body("Expected Upgrade to WebSocket version 13".into())
            .unwrap();
    }
    let is_valid_origin = req
        .headers()
        .get(ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map_or(true, |v| v == "http://localhost:8080");
    if !is_valid_origin {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body("".into())
            .unwrap();
    }
    let key = if let Some(value) = req.headers().get(SEC_WEBSOCKET_KEY) {
        convert_key(value.as_bytes())
    } else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("".into())
            .unwrap();
    };

    let on_upgrade = req
        .into_body()
        .on_upgrade()
        .map_err(|e| {
            eprintln!("upgrade failed: {}", e);
        })
        .map(|upgraded| {
            let x = WebSocketStream::from_raw_socket(upgraded, Role::Server, None);
            x
        });
    tokio_01::spawn(on_upgrade.and_then(move |ws| {
        let (sink, stream) = ws.split();

        let responses = stream.map(move |_from_message| {
            let mut msg = Vec::new();
            while let Ok(to_message) = receiver.try_recv() {
                msg.push(to_message.to_string());
            }
            Message::text(format!("[{}]", msg.join(",")))
        });
        sink.send_all(responses).map(|_| ()).map_err(|e| {
            eprintln!("failed websocket echo: {}", e);
        })
    }));

    Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(UPGRADE, "websocket")
        .header(CONNECTION, "upgrade")
        .header(SEC_WEBSOCKET_ACCEPT, key.as_str())
        .body(Body::empty())
        .unwrap()
}
