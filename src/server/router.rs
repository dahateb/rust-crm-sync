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
    setup: Setup,
    trigger_sender: Mutex<Sender<(String, u16)>>,
    trigger_receiver: Mutex<Receiver<(String, u16)>>,
    message_sender: Mutex<Sender<(String, u64, Instant)>>,
    message_receiver: Mutex<Receiver<(String, u64, Instant)>>,
}

impl Router {
    pub fn new(config: &'static Config) -> Router {
        let (sender, receiver) = channel();
        let (tx, rx) = channel();
        let db_arc = Arc::new(Db::new(&config.db));
        let sf_arc = Arc::new(Salesforce::new(&config.salesforce));
        Router {
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
                //check if object already exists
                //self.setup.object_exists(index: usize)
                let sender = self.trigger_sender.lock().unwrap().clone();
                let body = req.into_body();
                return response::response_notify(
                    body,
                    "/setup/new".to_owned(),
                    StatusCode::CREATED,
                    sender,
                );
            }
            (&Method::POST, "/setup/delete") => {
                let sender = self.trigger_sender.lock().unwrap().clone();
                let body = req.into_body();
                return response::response_notify(
                    body,
                    "/setup/delete".to_owned(),
                    StatusCode::OK,
                    sender,
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
                    let notify = |notification: &str, count: u64| {
                        let _ = sender.send((notification.to_owned(), count, Instant::now()));
                    };
                    let _res = self.setup.setup_sf_object(message.1 as usize, true, notify);
                }
                "/setup/delete" => {
                    let _res = self
                        .setup
                        .delete_db_object(message.1 as usize)
                        .map_err(|err| println!("{}", err));
                }
                _ => println!(""),
            }
        }
    }
}
