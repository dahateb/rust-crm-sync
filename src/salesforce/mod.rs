use futures::{Future, Stream};
use hyper::Client;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use hyper::{Method, Request};
use hyper::header::{ContentLength, ContentType, Authorization};
use std::str::{self};
use std::ops::Sub;
use serde_json::{self,Value};
use config::{Config, SalesforceConfig};
use self::objects::{SObject, SObjectList, SObjectDescribe};
use chrono::prelude::*;
use time::Duration;
use std::cell::RefCell;

pub mod objects;

#[derive(Serialize, Deserialize)]
pub struct LoginData {
    pub access_token: String,
    pub instance_url: String,
    id: String,
    token_type: String,
    issued_at: String,
    signature: String
}

pub struct Salesforce <'a> {
    config: &'a SalesforceConfig,
    login_data:  Option<LoginData>,
    client: Client<HttpsConnector<HttpConnector>>,
    //use RefCell because core needs to be mutable for some reason
    core: RefCell<Core>
}

impl<'a> Salesforce <'a>{
    
    pub fn new(config: &'a Config) -> Salesforce<'a> {
        let core = Core::new().unwrap();
        let client = Client::configure()
        .connector(HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());
        Salesforce {
            config : &config.salesforce,
            login_data : Option::None,
            client: client,
            core: RefCell::new(core) 
        }
    }
    /*
    pub fn from_login_data(config: &'a Config,ld: Option<LoginData>) -> Salesforce<'a> {

    }
    */

    pub fn login(&mut self) {
        
        let uri = self.config.uri.parse().unwrap();
        let params = format!(
            "grant_type=password&client_id={}&client_secret={}&username={}&password={}{}",
            self.config.client_id,
            self.config.client_secret,
            self.config.username,
            self.config.password,
            self.config.sec_token, 
        );
        let mut req = Request::new(Method::Post, uri);
        req.headers_mut().set(ContentType::form_url_encoded());
        req.headers_mut().set(ContentLength(params.len() as u64));
        req.set_body(params);
        let posted_str = self.call(req);
        let ld: LoginData = serde_json::from_str(posted_str.unwrap().as_str()).unwrap();
        self.login_data = Some(ld);
    }

    pub fn get_objects(& self) -> Result<Vec<SObject>,String> {
        let req_builder = |uri : &String| {
            format!(
                "{}/services/data/v40.0/sobjects", 
                uri
            )
        };
        let req:Request = self.build_auth_request(req_builder);
        let posted_str = self.call(req).unwrap();
        let list: SObjectList = serde_json::from_str(posted_str.as_str()).unwrap();
        let filtered_list: Vec<SObject> = list.sobjects.into_iter()
        .filter(|x|(x.createable && x.queryable && x.layoutable) || x.custom_setting)
        .collect();        
        Ok(filtered_list)
    }

    pub fn describe_object(& self, object_name: &str,) -> Result<SObjectDescribe, String> {
        let req_builder = |uri: &String| {
            format!(
                "{}/services/data/v40.0/sobjects/{}/describe", 
                uri,
                object_name
            )
        };
        let req:Request = self.build_auth_request(req_builder);
        let posted_str = self.call(req).unwrap();
        let object: SObjectDescribe = serde_json::from_str(posted_str.as_str()).unwrap();
        Ok(object)
    }

    pub fn get_last_updated_records(& self, object_name: &str, time_sec: i64) {
        let date_diff: DateTime<Utc> = Utc::now().sub(Duration::minutes(time_sec));
        let query = format!(
            "SELECT+Id,+Name+FROM+{}+WHERE+lastmodifieddate>{}",
            object_name,
            date_diff.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        );
        println!("{}",query);
        let req_builder = |uri: &String| {
            format!(
                "{}/services/data/v40.0/query/?q={}", 
                uri,
                query
            )
        };
        let req:Request = self.build_auth_request(req_builder);
        let posted_str = self.call(req);
        let v: Value = serde_json::from_str(posted_str.unwrap().as_str()).unwrap();
        let fields = vec!["Id","Name"];
        for field in fields {
            println!("{}", v["records"][0][field]);
        }
        
    }

    pub fn print_login_data(&self) {
        let ld = self.login_data.as_ref().unwrap();
        println!("Access Token: {}", ld.access_token);
        println!("Instance Url: {}", ld.instance_url);
        let date_diff: DateTime<Utc> = Utc::now().sub(Duration::minutes(-30)); 
       
        println!("Time: {:?}", date_diff);
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

    fn build_auth_request<F>(&self, req_builder: F) -> Request where
        F: Fn(&String) -> String  {
        let ld = self.login_data.as_ref().unwrap();
        let uri = req_builder(&ld.instance_url);
        let mut req:Request = Request::new(Method::Get, uri.parse().unwrap());
        let auth = format!("Bearer {}", ld.access_token);
        req.headers_mut().set(Authorization(auth));
        return req;
    }
}

