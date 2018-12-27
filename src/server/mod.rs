pub mod router;

use config::Config;
use db::Db;
use futures::{future, Future};
use hyper::service::{NewService, Service};
use hyper::{Body, Error, Request, Response, Server};
use salesforce::Salesforce;
use server::router::Router;
use std::sync::Arc;
use sync::setup::Setup;

pub struct ApiServer {
    config: &'static Config,
    router: Arc<Router>,
}

impl ApiServer {
    pub fn start(config: &'static Config) {
        let db_arc = Arc::new(Db::new(&config.db));
        let sf_arc = Arc::new(Salesforce::new(&config.salesforce));
        let router = Router {
            setup: Setup::new(db_arc, sf_arc),
        };
        let addr = config.server.url.parse().unwrap();
        let server = ApiServer {
            config: config,
            router: Arc::new(router),
        };
        let server = Server::bind(&addr)
            .serve(server)
            .map_err(|e| eprintln!("error: {}", e));
        println!("Serving at {}", addr);

        hyper::rt::run(server); //<======
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
        Box::new(future::ok(self.router.handle(req)))
    }
}
