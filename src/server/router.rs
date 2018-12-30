use config::Config;
use db::Db;
use futures::{future, Future, Stream};
use hyper::{Body, Method, Request, Response, StatusCode};
use salesforce::Salesforce;
use server::response;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use sync::setup::Setup;
use url::form_urlencoded;

pub struct Router {
    pub setup: Setup,
    sender: Mutex<Sender<(String, u16)>>,
    receiver: Mutex<Receiver<(String, u16)>>,
}

impl Router {
    pub fn new(config: &'static Config) -> Router {
        let (sender, receiver) = channel();
        let db_arc = Arc::new(Db::new(&config.db));
        let sf_arc = Arc::new(Salesforce::new(&config.salesforce));
        Router {
            setup: Setup::new(db_arc, sf_arc),
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
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
                let sender = self.sender.lock().unwrap().clone();
                let response_fut = req.into_body().concat2().map(move |chunk| {
                    let params = form_urlencoded::parse(chunk.as_ref())
                        .into_owned()
                        .collect::<HashMap<String, String>>();
                    let number = if let Some(n) = params.get("number") {
                        if let Ok(v) = n.parse::<u16>() {
                            v
                        } else {
                            return response::response_unprocessable(response::NOTNUMERIC);
                        }
                    } else {
                        return response::response_unprocessable(response::MISSING);
                    };
                    let _ = sender.send((String::from("setup/new"), number));

                    *response.body_mut() = Body::from("OK");
                    *response.status_mut() = StatusCode::CREATED;
                    response
                });
                return Box::new(response_fut);
            },
            (&Method::POST, "/setup/delete") => {
                let sender = self.sender.lock().unwrap().clone();
                let response_fut = req.into_body().concat2().map(move |chunk| {
                    let params = form_urlencoded::parse(chunk.as_ref())
                        .into_owned()
                        .collect::<HashMap<String, String>>();
                    let number = if let Some(n) = params.get("number") {
                        if let Ok(v) = n.parse::<u16>() {
                            v
                        } else {
                            return response::response_unprocessable(response::NOTNUMERIC);
                        }
                    } else {
                        return response::response_unprocessable(response::MISSING);
                    };
                    let _ = sender.send((String::from("setup/delete"), number));

                    *response.body_mut() = Body::from("OK");
                    *response.status_mut() = StatusCode::OK;
                    response
                });
                return Box::new(response_fut);
            },
            _ => {
                // Return 404 not found response.
                *response.body_mut() = Body::from(response::NOTFOUND);
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        }
        Box::new(future::ok(response))
    }

    pub fn handle_async(&self, _instant: std::time::Instant) {
        let recv = self.receiver.lock().unwrap();
        while let Ok(message) = recv.try_recv() {
            println!("{}:{}", message.0, message.1);
            match message.0.as_ref() {
                "setup/new" => {
                    let notify = || println!(".");
                    let _res = self.setup.setup_sf_object(message.1 as usize, true, notify);
                },
                "setup/delete" => {
                    let _res = self.setup.delete_db_object(message.1 as usize);
                },
                _ => println!(""),
            }
        }
    }
}
