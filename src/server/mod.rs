pub mod response;
pub mod router;

use config::Config;
use futures::{future, Future};
use hyper::service::{NewService, Service};
use hyper::{Body, Error, Request, Response, Server};
use server::router::Router;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Interval;

pub struct ApiServer {
    config: &'static Config,
    router: Arc<Router>,
}

impl ApiServer {
    pub fn start(config: &'static Config) {
        let router = Arc::new(Router::new(config));
        let addr = config.server.url.parse().unwrap();
        let server = ApiServer {
            config: config,
            router: router.clone(),
        };
        let server = Server::bind(&addr)
            .serve(server)
            .map_err(|e| eprintln!("error: {}", e));

        let worker = Interval::new(Instant::now(), Duration::from_millis(1000))
            .for_each(move |instant| {
                router.handle_async(instant);
                Ok(())
            })
            .map_err(|e| panic!("interval errored; err={:?}", e));

        hyper::rt::run(hyper::rt::lazy(move || {
            println!("Serving at {}", addr);
            hyper::rt::spawn(server); //<======
            hyper::rt::spawn(worker);
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
    type Future = Box<Future<Item = Self::Service, Error = Self::InitError> + Send>;
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
    type Future = Box<Future<Item = Response<Body>, Error = Error> + Send>;
    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        self.router.handle(req)
    }
}
