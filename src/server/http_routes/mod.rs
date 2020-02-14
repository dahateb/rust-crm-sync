use crate::db::Db;
use crate::salesforce::Salesforce;
use crate::server::response;
use crate::sync::setup::Setup;
use crate::util::Message;
use chrono::prelude::Utc;
use chrono::TimeZone;
use std::collections::HashMap;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::str;
use std::sync::{Arc, Mutex};
use warp::Filter;

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
            ])
            .allow_methods(vec!["*"]);

        let index = warp::get()
            .and(warp::path::end())
            .map(|| warp::reply::html(str::from_utf8(response::INDEX).unwrap()));

        index
            .or(self.info())
            .or(self.setup_list())
            .or(self.setup_available())
            .or(self.setup_new())
            .with(cors)
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
        warp::get()
            .and(warp::path("info"))
            .map(move || warp::reply::json(&res))
    }

    // GET /setup/list
    fn setup_list(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
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

        let res = self.setup.list_salesforce_objects(print_func).unwrap();
        warp::get()
            .and(warp::path!("setup" / "list"))
            .map(move || warp::reply::json(&res))
    }

    // GET /setup/available
    fn setup_available(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let print_func = |obj: (u32, &String, u32, usize)| {
            json!({
                "num":  obj.0,
                "name":  obj.1,
                "count":  obj.2,
                "num_fields": obj.3
            });
        };
        let res = self.setup.list_db_objects(print_func).unwrap();
        warp::get()
            .and(warp::path!("setup" / "available"))
            .map(move || warp::reply::json(&res))
    }

    fn setup_new(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
    {
        let sender = self.trigger_sender.clone();
        warp::post()
        .and(warp::path!("setup" / "new"))
        .and(warp::body::form())
        .map(move |params: HashMap<String, String>| {
            let number = if let Some(n) = params.get("number") {
                if let Ok(v) = n.parse::<usize>() {
                    v
                } else {
                    return "Error";
                }
            } else {
                return "Error";
            };
            let _ = sender.send(("setup/new".to_owned(), number));
            println!("{}", number);
            "OK"
        })
    }
}
