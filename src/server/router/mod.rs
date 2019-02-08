pub mod async;
pub mod websocket;

use chrono::prelude::Utc;
use chrono::TimeZone;
use crossbeam_channel::{unbounded, Receiver, Sender};
use db::Db;
use futures::future;
use hyper::{Body, Method, Request, StatusCode};
use salesforce::Salesforce;
use server::response;
use server::router::async::AsyncRouter;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use sync::setup::Setup;
use util::{Message};

pub struct Router {
    sync_toggle_switch: Arc<Mutex<bool>>,
    setup: Setup,
    trigger_sender: Sender<(String, usize)>,
    trigger_receiver: Receiver<(String, usize)>,
    message_sender: Sender<Box<Message>>,
    message_receiver: Receiver<Box<Message>>,
    sync_receiver: Receiver<Box<Message>>,
}

impl Router {
    pub fn new(
        sf_arc: Arc<Salesforce>,
        db_arc: Arc<Db>,
        sync_receiver: Receiver<Box<Message>>,
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

    pub fn async(&self) -> AsyncRouter {
        AsyncRouter::new(
            self.setup.clone(),
            self.trigger_receiver.clone(),
            self.message_sender.clone(),
        )
    }

    pub fn handle(&self, req: Request<Body>) -> response::BoxFut {
        let mut response = response::default_response().body(Body::empty()).unwrap();
        match (req.method(), req.uri().path()) {
            (&Method::OPTIONS, _) => {
                response = response::cors_response();
            }
            (&Method::GET, "/") | (&Method::GET, "/index.html") => {
                *response.body_mut() = Body::from(response::INDEX);
            }
            (&Method::GET, "/info") => {
                let sf = self.setup.salesforce();
                let client = sf.client();
                let sf_conn_data = client.get_login_data();
                let res = json!({
                    "sync_running": *self.sync_toggle_switch.lock().unwrap(),
                    "access_token": sf_conn_data.access_token,
                    "instance_url": sf_conn_data.instance_url,
                    "created": Utc.timestamp_millis(sf_conn_data.issued_at).to_rfc2822()
                });
                *response.body_mut() = Body::from(res.to_string());
            }
            (&Method::GET, "/setup/list") => {
                let print_func = |obj: (u32, &String, &String, bool, bool, bool)| {
                    let row = json!({
                        "num":  obj.0,
                        "name":  obj.1,
                        "label": obj.2,
                        "custom_setting": obj.3,
                        "createable":  obj.4,
                        "synched": obj.5
                    });
                    row.to_string()
                };

                let res = self
                    .setup
                    .list_salesforce_objects(print_func)
                    .unwrap()
                    .join(",");
                return response::build_json_response(res);
            }
            (&Method::GET, "/setup/available") => {
                let print_func = |obj: (u32, &String, u32, usize)| {
                    let row = json!({
                        "num":  obj.0,
                        "name":  obj.1,
                        "count":  obj.2,
                        "num_fields": obj.3
                    });
                    row.to_string()
                };
                let res = self.setup.list_db_objects(print_func).unwrap().join(",");
                return response::build_json_response(res);
            }
            (&Method::POST, "/setup/new") => {
                return response::response_notify(
                    req.into_body(),
                    "/setup/new".to_owned(),
                    StatusCode::CREATED,
                    self.trigger_sender.clone(),
                    self.setup.clone(),
                );
            }
            (&Method::POST, "/setup/delete") => {
                return response::response_notify(
                    req.into_body(),
                    "/setup/delete".to_owned(),
                    StatusCode::OK,
                    self.trigger_sender.clone(),
                    self.setup.clone(),
                );
            }
            (&Method::GET, "/messages") => {
                let mut result = Vec::new();
                let recv = self.message_receiver.clone();
                while let Ok(message) = recv.try_recv() {
                    result.push(message.to_string());
                }
                return response::build_json_response(result.join(","));
            }
            (&Method::PUT, "/sync/start") => {
                *self.sync_toggle_switch.lock().unwrap() = true;
                *response.body_mut() = Body::from(json!({"sync_running": true}).to_string());
            }
            (&Method::PUT, "/sync/stop") => {
                *self.sync_toggle_switch.lock().unwrap() = false;
                *response.body_mut() = Body::from(json!({"sync_running": false}).to_string());
            }
            (&Method::GET, "/sync/messages") => {
                let mut result = Vec::new();
                let recv = self.sync_receiver.clone();
                while let Ok(message) = recv.try_recv() {
                    result.push(message.to_string());
                }
                return response::build_json_response(result.join(","));
            }
            (&Method::GET, "/ws/sync/messages") => {
                response = websocket::mux(req, self.sync_receiver.clone());
            }
            (&Method::GET, "/ws/messages") => {
                response = websocket::mux(req, self.message_receiver.clone());
            }
            _ => {
                // Return 404 not found response.
                *response.body_mut() = Body::from(response::NOTFOUND);
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }
        Box::new(future::ok(response))
    }
}
