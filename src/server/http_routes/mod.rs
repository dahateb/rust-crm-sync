pub mod handlers;
pub mod worker;

use crate::db::Db;
use crate::salesforce::Salesforce;
use crate::server::http_routes::handlers::{handle_salesforce_get_list, handle_setup_list, handle_ws};
use crate::server::http_routes::worker::AsyncRouter;
use crate::server::response;
use crate::sync::setup::Setup;
use crate::util::Message;
use chrono::prelude::Utc;
use chrono::TimeZone;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::str;
use std::sync::{Arc, Mutex};
use warp::http::StatusCode;

use warp::reply::{html, json};
use warp::{http::Method, Filter};

pub struct Router {
    sync_toggle_switch: Arc<Mutex<bool>>,
    setup: Setup,
    trigger_sender: Sender<(String, usize)>,
    trigger_receiver: Receiver<(String, usize)>,
    message_sender: Sender<Box<dyn Message>>,
    message_receiver: Receiver<Box<dyn Message>>,
    sync_receiver: Receiver<Box<dyn Message>>,
}

impl Router {
    pub fn new(
        sf_arc: Arc<Salesforce>,
        db_arc: Arc<Db>,
        sync_receiver: Receiver<Box<dyn Message>>,
        sync_toggle_switch: Arc<Mutex<bool>>,
    ) -> Router {
        let (sender, receiver) = unbounded();
        let (tx, rx) = unbounded();
        Router {
            sync_toggle_switch,
            setup: Setup::new(db_arc, sf_arc),
            trigger_sender: sender,
            trigger_receiver: receiver,
            message_sender: tx,
            message_receiver: rx,
            sync_receiver,
        }
    }

    pub fn worker(&self) -> AsyncRouter {
        AsyncRouter::new(
            self.setup.clone(),
            self.trigger_receiver.clone(),
            self.message_sender.clone(),
        )
    }

    pub fn build_routes(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec![
                "User-Agent",
                "Sec-Fetch-Mode",
                "Referer",
                "Origin",
                "Access-Control-Request-Method",
                "Access-Control-Request-Headers",
                "Host",
                "Connection",
                "Accept",
                "Accept-Encoding",
                "Accept-Language",
                "Content-Type",
            ])
            .allow_methods(&[
                Method::GET,
                Method::POST,
                Method::DELETE,
                Method::OPTIONS,
                Method::PUT,
            ]);

        let index = warp::get()
            .and(warp::path::end())
            .map(|| html(str::from_utf8(response::INDEX).unwrap()));

        index
            .or(self.info())
            .or(self.setup_list())
            .or(self.setup_available())
            .or(self.setup_new())
            .or(self.setup_delete())
            .or(self.get_messages())
            .or(self.get_sync_messages())
            .or(self.start_sync())
            .or(self.stop_sync())
            .or(self.ws_sync_messages())
            .or(self.ws_messages())
            .with(cors)
            .with(warp::log("info"))
    }

    // GET /info
    fn info(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let sf = self.setup.salesforce();
        let client = sf.client();
        let sf_conn_data = client.get_login_data();
        let res = json!({
            "sync_running": *self.sync_toggle_switch.lock().unwrap(),
            "access_token": sf_conn_data.access_token,
            "instance_url": sf_conn_data.instance_url,
            "created": Utc.timestamp_millis(sf_conn_data.issued_at).to_rfc2822()
        });
        warp::get().and(warp::path("info")).map(move || json(&res))
    }

    // GET /setup/list
    fn setup_list(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let setup = self.setup.clone();
        warp::get()
            .and(warp::path!("setup" / "list"))
            .and(warp::any().map(move || setup.clone()))
            .and_then(handle_salesforce_get_list)
    }

    // GET /setup/available
    fn setup_available(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let setup = self.setup.clone();
        warp::get()
            .and(warp::path!("setup" / "available"))
            .and(warp::any().map(move || setup.clone()))
            .and_then(handle_setup_list)
    }
    // POST /setup/new
    fn setup_new(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let sender = self.trigger_sender.clone();
        warp::post()
            .and(warp::path!("setup" / "new"))
            .and(warp::body::form())
            .map(move |params: HashMap<String, String>| {
                response::response_for_sender(
                    params,
                    "/setup/new".to_owned(),
                    sender.clone(),
                    StatusCode::CREATED,
                )
            })
    }

    // POST /setup/delete
    fn setup_delete(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let sender = self.trigger_sender.clone();
        warp::post()
            .and(warp::path!("setup" / "delete"))
            .and(warp::body::form())
            .map(move |params: HashMap<String, String>| {
                response::response_for_sender(
                    params,
                    "/setup/new".to_owned(),
                    sender.clone(),
                    StatusCode::CREATED,
                )
            })
    }

    // GET /messages
    pub fn get_messages(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let recv = self.message_receiver.clone();
        warp::get().and(warp::path!("messages")).map(move || {
            let mut result = Vec::new();
            while let Ok(message) = recv.try_recv() {
                result.push(message.as_value());
            }
            json(&result)
        })
    }

    // GET /sync/messages
    pub fn get_sync_messages(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let recv = self.sync_receiver.clone();
        warp::get()
            .and(warp::path!("sync" / "messages"))
            .map(move || {
                let mut result = Vec::new();
                while let Ok(message) = recv.try_recv() {
                    result.push(message.as_value());
                }
                json(&result)
            })
    }

    // PUT /sync/start
    pub fn start_sync(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let sync_switch = self.sync_toggle_switch.clone();
        warp::put().and(warp::path!("sync" / "start")).map(move || {
            *sync_switch.lock().unwrap() = true;
            json(&json!({"sync_running": true}))
        })
    }

    // PUT /sync/stop
    pub fn stop_sync(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let sync_switch = self.sync_toggle_switch.clone();
        warp::put().and(warp::path!("sync" / "start")).map(move || {
            *sync_switch.lock().unwrap() = false;
            json(&json!({"sync_running": false}))
        })
    }

    pub fn ws_sync_messages(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let receiver = self.sync_receiver.clone();
        warp::get()
            .and(warp::path!("ws" / "sync" / "messages"))
            .and(warp::ws())
            .map(move |ws: warp::ws::Ws| {
                let recv = receiver.clone();
                ws.on_upgrade(move |socket| handle_ws(socket, recv))
            })
    }

    pub fn ws_messages(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let receiver = self.message_receiver.clone();
        warp::get()
            .and(warp::path!("ws" / "messages"))
            .and(warp::ws())
            .map(move |ws: warp::ws::Ws| {
                let recv = receiver.clone();
                ws.on_upgrade(move |socket| handle_ws(socket, recv))
            })
    }
}
