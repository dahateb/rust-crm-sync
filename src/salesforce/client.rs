use crate::config::SalesforceConfig;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client as ReqClient, Method, Request, RequestBuilder, Response};
use serde_aux::prelude::deserialize_number_from_string;
use std::collections::HashMap;
use std::io::Read;

#[derive(Serialize, Deserialize, Clone)]
pub struct LoginData {
    pub access_token: String,
    pub instance_url: String,
    pub id: String,
    pub token_type: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub issued_at: i64,
    pub signature: String,
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

    pub fn get_login_data(&self) -> &LoginData {
        self.login_data.as_ref().unwrap()
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
        let req = self.client.post(config.uri.as_str());
        let req = req.form(&params).build().unwrap();
        let mut response = self.call(req).unwrap();
        //println!("{}", response.text().unwrap());
        let ld: LoginData = response.json().map_err(|err| println!("{}", err)).unwrap();
        self.login_data = Some(ld);
        self
    }

    pub fn print_login_data(&self) {
        let ld = self.login_data.as_ref().unwrap();
        println!("Access Token: {}", ld.access_token);
        println!("Instance Url: {}", ld.instance_url);
    }

    pub fn get_resource<F>(&self, req_builder: F) -> Result<String, String>
    where
        F: Fn(&String) -> String,
    {
        let req = self.build_auth_request(Method::GET, req_builder);
        let mut response = (self.call(req.build().unwrap()))?;
        let mut result = String::new();
        let _bytes_read = response.read_to_string(&mut result);
        Ok(result)
    }

    pub fn update_resource<F>(&self, data: String, req_builder: F) -> Result<String, String>
    where
        F: Fn(&String) -> String,
    {
        let builder = self.build_auth_request(Method::PATCH, req_builder);
        let mut req = builder.body(data).build().unwrap();
        req.headers_mut()
            .insert(CONTENT_TYPE, "application/json".parse().unwrap());
        let mut response = self.call(req)?;
        let mut result = String::new();
        let _bytes_read = response.read_to_string(&mut result);
        Ok(result)
    }

    pub fn create_resource<F>(&self, data: String, req_builder: F) -> Result<String, String>
    where
        F: Fn(&String) -> String,
    {
        let builder = self.build_auth_request(Method::POST, req_builder);
        let mut req = builder.body(data).build().unwrap();
        req.headers_mut()
            .insert(CONTENT_TYPE, "application/json".parse().unwrap());
        let mut response = self.call(req)?;
        let mut result = String::new();
        let _bytes_read = response.read_to_string(&mut result);
        Ok(result)
    }

    fn call(&self, req: Request) -> Result<Response, String> {
        let mut response = self.client.execute(req).map_err(|err| err.to_string())?;
        if !response.status().is_success() {
            let mut result = String::new();
            let _ = response.read_to_string(&mut result);
            return Err(format!("{} {}", response.status(), result));
        }
        Ok(response)
    }

    fn build_auth_request<F>(&self, method: Method, req_builder: F) -> RequestBuilder
    where
        F: Fn(&String) -> String,
    {
        let ld = self.login_data.as_ref().unwrap();
        let uri = req_builder(&ld.instance_url);
        let req = self.client.request(method, uri.as_str());
        req.bearer_auth(ld.access_token.clone())
    }
}
