use std::cell::RefCell;
use std::rc::Rc;
use config::SalesforceConfig;
use tokio_core::reactor::Core;
use futures::{Future, Stream};
use hyper::Client as HyperClient;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use hyper::{Method, Request};
use hyper::header::{ContentLength, ContentType, Authorization};
use serde_json::{self,Value};

#[derive(Serialize, Deserialize)]
pub struct LoginData {
    pub access_token: String,
    pub instance_url: String,
    id: String,
    token_type: String,
    issued_at: String,
    signature: String
}

pub struct Client {
    login_data:  Option<LoginData>,
    client: HyperClient<HttpsConnector<HttpConnector>>,
    //use RefCell because core needs to be mutable for some reason
    core: RefCell<Core>
}

impl Client {
    
    pub fn new() -> Client {
        let core = Core::new().unwrap();
        let client = HyperClient::configure()
        .connector(HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());
        Client {
            login_data: None,
            client: client,
            core: RefCell::new(core)
        }
    }

    pub fn connect(mut self, config: Rc<SalesforceConfig>) -> Client {
        
        let uri = config.uri.parse().unwrap();
        let params = format!(
            "grant_type=password&client_id={}&client_secret={}&username={}&password={}{}",
            config.client_id,
            config.client_secret,
            config.username,
            config.password,
            config.sec_token, 
        );
        let mut req = Request::new(Method::Post, uri);
        req.headers_mut().set(ContentType::form_url_encoded());
        req.headers_mut().set(ContentLength(params.len() as u64));
        req.set_body(params);
        let posted_str = self.call(req);
        let ld: LoginData = serde_json::from_str(posted_str.unwrap().as_str()).unwrap();
        self.login_data = Some(ld);
        self
    }
    
    pub fn print_login_data(&self) {
        let ld = self.login_data.as_ref().unwrap();
        println!("Access Token: {}", ld.access_token);
        println!("Instance Url: {}", ld.instance_url);
    }

    pub fn get_resource<F> (&self, req_builder: F) -> Result<String, String> where
        F: Fn(&String) -> String  {
        let req = self.build_auth_request(req_builder);
        self.call(req)
    }

    fn build_auth_request<F>(&self, req_builder: F) -> Request where
        F: Fn(&String) -> String  {
        let ld = self.login_data.as_ref().unwrap();
        let uri = req_builder(&ld.instance_url);
        let mut req:Request = Request::new(Method::Get, uri.parse().unwrap());
        let auth = format!("Bearer {}", ld.access_token);
        req.headers_mut().set(Authorization(auth));
        return req;
    }

    fn call(& self, req: Request) -> Result<String, String>{
        let client = &self.client;        
        let mut core = &mut self.core.borrow_mut();
        let method =  req.method().clone();
        let post = client.request(req).and_then(|res| {
            println!("{}: {}", method, res.status());
            res.body().concat2()
        });
        let posted = core.run(post).unwrap();
        let posted = String::from_utf8(posted.to_vec()).unwrap();
        Ok(posted)
    }
}
