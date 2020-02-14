pub mod executer;
pub mod http_routes;
pub mod response;
pub mod router;

use crate::config::Config;
use crate::db::Db;
use crate::salesforce::Salesforce;
use crate::server::executer::Executer2;
use crate::server::http_routes::Router as Router2;
use crate::server::router::Router;
use crate::sync::executer::MESSAGE_CHANNEL_SIZE;
use crossbeam_channel::bounded;
use futures::{future, lazy, Future};
use hyper::service::{NewService, Service};
use hyper::{Body, Error, Request, Response, Server};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_01::prelude::*;
use tokio_01::timer::Interval;

pub struct ApiServer {
    config: &'static Config,
    router: Arc<Router>,
}

impl ApiServer {
    pub fn start(config: &'static Config) {
        let sf_arc = Arc::new(Salesforce::new(&config.salesforce));
        let db_arc = Arc::new(Db::new(&config.db));
        let (tx, rx) = bounded(MESSAGE_CHANNEL_SIZE);
        let executer = Executer2::new(sf_arc.clone(), db_arc.clone(), &config.sync);
        let router2 = Arc::new(Router2::new(
            sf_arc.clone(),
            db_arc.clone(),
            rx.clone(),
            executer.toggle_switch().clone(),
        ));
        let router = Arc::new(Router::new(
            sf_arc.clone(),
            db_arc.clone(),
            rx.clone(),
            executer.toggle_switch().clone(),
        ));
        let addr = config.server.url.parse().unwrap();
        let async_router = router.worker();
        let server = ApiServer {
            config: config,
            router: router,
        };
        let server = Server::bind(&addr)
            .serve(server)
            .map_err(|e| eprintln!("server error: {}", e));
        // setup worker
        let worker = Interval::new(Instant::now(), Duration::from_millis(1000))
            .for_each(move |instant| {
                async_router.handle_async(instant);
                Ok(())
            })
            .map_err(|e| eprintln!("worker errored; err={:?}", e));
        let skip_switch = executer.toggle_switch().clone();
        //sync worker
        let executer_worker =
            Interval::new(Instant::now(), Duration::from_millis(config.sync.timeout))
                .for_each(move |_instant| {
                    {
                        if !*skip_switch.lock().unwrap() {
                            return Ok(());
                        }
                    }
                    executer.execute(tx.clone(), rx.clone());
                    //  let note = format!("{:?}", instant);
                    //  send_with_clear(SyncMessage::new(note.as_str(), ""), &tx, &rx);
                    Ok(())
                })
                .map_err(|e| eprintln!("executer errored; err={:?}", e));
        tokio_compat::run(lazy(move || {
            println!("Serving at {}", addr);
            tokio_02::spawn(async move {
                warp::serve(router2.build_routes())
                    .run(([127, 0, 0, 1], 3030))
                    .await;
            });
            tokio_01::spawn(lazy(|| server)); //<======
            tokio_01::spawn(lazy(|| worker));
            tokio_01::spawn(lazy(|| executer_worker));
            Ok(())
        }));
    }
}

impl NewService for ApiServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type InitError = Error;
    type Service = ApiServer;
    type Future = Box<dyn Future<Item = Self::Service, Error = Self::InitError> + Send>;
    fn new_service(&self) -> Self::Future {
        Box::new(future::ok(Self {
            config: self.config,
            router: self.router.clone(),
        }))
    }
}

impl Service for ApiServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = Box<dyn Future<Item = Response<Body>, Error = Error> + Send>;
    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        self.router.handle(req)
    }
}
