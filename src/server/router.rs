use config::Config;
use db::Db;
use futures::future;
use hyper::{Body, Method, Request, Response, StatusCode};
use salesforce::Salesforce;
use server::response;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use sync::setup::Setup;

pub struct Router {
    sync_toggle_switch: Arc<Mutex<bool>>,
    setup: Setup,
    trigger_sender: Mutex<Sender<(String, usize)>>,
    trigger_receiver: Mutex<Receiver<(String, usize)>>,
    message_sender: Mutex<Sender<(String, u64, Instant)>>,
    message_receiver: Mutex<Receiver<(String, u64, Instant)>>,
}

impl Router {
    pub fn new(sf_arc: Arc<Salesforce>, db_arc: Arc<Db>, sync_toggle_switch: Arc<Mutex<bool>>) -> Router {
        let (sender, receiver) = channel();
        let (tx, rx) = channel();
        Router {
            sync_toggle_switch,
            setup: Setup::new(db_arc, sf_arc),
            trigger_sender: Mutex::new(sender),
            trigger_receiver: Mutex::new(receiver),
            message_sender: Mutex::new(tx),
            message_receiver: Mutex::new(rx),
        }
    }

    pub fn handle(&self, req: Request<Body>) -> response::BoxFut {
        let mut response = Response::new(Body::empty());
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") | (&Method::GET, "/index.html") => {
                *response.body_mut() = Body::from(response::INDEX);
            }
            (&Method::GET, "/setup/list") => {
                let print_func = |obj: (u32, &String, bool)| {
                    let row = json!({
                        "num":  obj.0,
                        "name":  obj.1,
                        "creatable":  obj.2
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
                let print_func = |obj: (u32, &String, u32)| {
                    let row = json!({
                        "num":  obj.0,
                        "name":  obj.1,
                        "count":  obj.2
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
                    self.trigger_sender.lock().unwrap().clone(),
                    self.setup.clone(),
                );
            }
            (&Method::POST, "/setup/delete") => {
                return response::response_notify(
                    req.into_body(),
                    "/setup/delete".to_owned(),
                    StatusCode::OK,
                    self.trigger_sender.lock().unwrap().clone(),
                    self.setup.clone(),
                );
            }
            (&Method::GET, "/messages") => {
                let mut result = Vec::new();
                let recv = self.message_receiver.lock().unwrap();
                while let Ok(message) = recv.try_recv() {
                    let timestamp = format!("{:?}", message.2);
                    let json = json!({
                        "message": message.0,
                        "count": message.1,
                        "timestamp": timestamp
                    });
                    result.push(json.to_string());
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
            _ => {
                // Return 404 not found response.
                *response.body_mut() = Body::from(response::NOTFOUND);
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }
        Box::new(future::ok(response))
    }

    pub fn handle_async(&self, _instant: std::time::Instant) {
        let recv = self.trigger_receiver.lock().unwrap();
        while let Ok(message) = recv.try_recv() {
            println!("{}:{}", message.0, message.1);
            match message.0.as_ref() {
                "/setup/new" => {
                    let sender = self.message_sender.lock().unwrap().clone();
                    let setup = self.setup.clone();
                    //asynchronous to allow for multiple objects
                    std::thread::spawn(move || {
                        let notify = |notification: &str, count: u64| {
                            let _ = sender.send((notification.to_owned(), count, Instant::now()));
                        };
                        let _res = setup.setup_sf_object(message.1, true, notify);
                    });
                }
                "/setup/delete" => {
                    let _res = self
                        .setup
                        .delete_db_object(message.1)
                        .map_err(|err| println!("{}", err));
                }
                _ => println!(""),
            }
        }
    }
}
