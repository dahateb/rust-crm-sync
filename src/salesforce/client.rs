use std::collections::HashMap;
use std::io::Read;
use config::SalesforceConfig;
use reqwest::{Client as ReqClient, Request, RequestBuilder, Response, Method};
use reqwest::header::{Headers, Authorization, Bearer, ContentType};
use serde_json::value::Value;

#[derive(Serialize, Deserialize)]
pub struct LoginData {
    pub access_token: String,
    pub instance_url: String,
    id: String,
    token_type: String,
    issued_at: String,
    signature: String,
}

pub struct Client {
    login_data: Option<LoginData>,
    client: ReqClient,
}

impl Client {
    pub fn new(login_data: Option<LoginData>) -> Client {
        Client {
            login_data: login_data,
            client: ReqClient::new(),
        }
    }

    pub fn get_login_data(self) -> LoginData {
        self.login_data.unwrap()
    }

    pub fn is_connected(&self) -> bool {
        let option = self.login_data.as_ref();
        match option {
            None => false,
            Some(_value) => true,
        }
    }

    pub fn connect(mut self, config: &'static SalesforceConfig) -> Client {
        if self.is_connected() {
            return self;
        }

        let password = format!("{}{}", config.password, config.sec_token);
        let mut params = HashMap::new();
        params.insert("grant_type", "password");
        params.insert("client_id", config.client_id.as_str());
        params.insert("client_secret", config.client_secret.as_str());
        params.insert("username", config.username.as_str());
        params.insert("password", password.as_str());
        let mut req = self.client.post(config.uri.as_str());
        let req = req.form(&params).build().unwrap();
        let mut response = self.call(req).unwrap();
	//println!("{:?}",response);
	//let mut res = String::new();
	//response.read_to_string(&mut res);
	//println!("{}", res);
        let ld: LoginData = response.json().map_err(|err| println!("Login failed: {}", err)).unwrap();
        self.login_data = Some(ld);
        self
    }

    pub fn print_login_data(&self) {
        let ld = self.login_data.as_ref().unwrap();
        println!("Access Token: {}", ld.access_token);
        println!("Instance Url: {}", ld.instance_url);
    }

    pub fn get_resource<F>(&self, req_builder: F) -> Result<String, String>
        where F: Fn(&String) -> String
    {
        let mut req = self.build_auth_request(Method::Get, req_builder);
        let mut response = try!(self.call(req.build().unwrap()));
        let mut result = String::new();
        let _bytes_read = response.read_to_string(&mut result);
        Ok(result)
    }

    pub fn update_resource<F>(&self, data: String, req_builder: F) -> Result<String,String>
        where F: Fn(&String) -> String
    {
        let mut builder = self.build_auth_request(Method::Patch, req_builder);
        builder.body(data);
        let mut req = builder.build().unwrap();
        req.headers_mut().set(ContentType::json());
        let mut response = try!(self.call(req));
        let mut result = String::new();
        let _bytes_read = response.read_to_string(&mut result);
        Ok(result)
    }

    pub fn create_resource<F>(&self,  data: String, req_builder: F) -> Result<String,String>
        where F: Fn(&String) -> String
    {
        let mut builder = self.build_auth_request(Method::Post, req_builder);
        builder.body(data);
        let mut req = builder.build().unwrap();
        req.headers_mut().set(ContentType::json());
        let mut response = try!(self.call(req));
        let mut result = String::new();
        let _bytes_read = response.read_to_string(&mut result);
        Ok(result)
    }

    fn call(&self, req: Request) -> Result<Response,String> {
        let mut response = try!(self.client
            .execute(req)
            .map_err(|err| err.to_string()));
        if !response.status().is_success() {
            let mut result = String::new();
            let _= response.read_to_string(&mut result);
            return Err(format!("{} {}", response.status(), result));
        }
        Ok(response)
    }

    fn build_auth_request<F>(&self,method: Method, req_builder: F) -> RequestBuilder
        where F: Fn(&String) -> String
    {
        let ld = self.login_data.as_ref().unwrap();
        let uri = req_builder(&ld.instance_url);
        let mut req = self.client.request(method, uri.as_str());
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer { token: ld.access_token.clone() }));
        req.headers(headers);
        req
    }
}
