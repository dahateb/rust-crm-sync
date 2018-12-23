use hyper::service::{NewService, Service};
use hyper::{Body,Error,Response,StatusCode,Request,Server};
use futures::{future, Future, IntoFuture};
use config::Config;
//use futures::future::Future;

pub struct ApiServer {
    config: &'static Config
}

impl ApiServer {
    
    pub fn start(config: &'static Config) {
        let addr = config.server.url.parse().unwrap();
        let server= ApiServer{config};
        let server = Server::bind(&addr)
            .serve(server)
            .map_err(|e| eprintln!("error: {}", e));;
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
        }))
    }
}

impl Service for ApiServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = Box<Future<Item = Response<Body>, Error = Error> + Send>;
    fn call(&mut self, _req: Request<Self::ReqBody>) -> Self::Future {
        //let something = self.something.to_string();
        let test = self.config.server.url.as_str();
        Box::new(future::ok(
            Response::builder()
                .status(StatusCode::OK)
                .body(test.into())
                .unwrap()
        ))
    }
}
