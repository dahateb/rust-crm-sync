use hyper::{Body, Request, Response, Server as HyperServer};
use hyper::service::service_fn_ok;
use hyper::rt::{self, Future};
use config::Config;

pub struct Server {
    config: &'static Config
}

impl Server {
    pub fn new(config: &'static Config) -> Server {
        pretty_env_logger::init();
        //let addr = ([127, 0, 0, 1], 3000).into();
        Server {
            config
        }
    }

    pub fn run(&self) {
        let config = self.config;
        let addr = config.server.url.parse().unwrap();
        let server = HyperServer::bind(&addr)
            .serve(move || {
                // This is the `Service` that will handle the connection.
                // `service_fn_ok` is a helper to convert a function that
                // returns a Response into a `Service`.
                
                service_fn_ok(move |_: Request<Body>| {
                    Response::new(Body::from(format!("Hello World from: http://{} ", config.server.url)))
                })
            })
            .map_err(|e| eprintln!("server error: {}", e));

        println!("Listening on http://{}", &addr);
        rt::run(server);
    }
}
