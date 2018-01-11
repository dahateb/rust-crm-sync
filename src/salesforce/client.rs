use std::collections::HashMap;
use std::io::Read;
use config::SalesforceConfig;
use reqwest::{Client as ReqClient, Request, Response};
use reqwest::header::{Headers, Authorization, Bearer};

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
            client: ReqClient::new().unwrap(),
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
        let mut req = self.client.post(config.uri.as_str()).unwrap();
        let req = req.form(&params).unwrap().build();
        let mut response = self.call(req);
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
        where F: Fn(&String) -> String
    {
        let req = self.build_auth_request(req_builder);
        let mut response = self.call(req);
        let mut result = String::new();
        let bytes_read = response.read_to_string(&mut result);
        Ok(result)
    }

    fn call(&self, req: Request) -> Response {
        let mut response = self.client
            .execute(req)
            .map_err(|err| println!("{:?}", err))
            .unwrap();
        if !response.status().is_success() {
            let mut result = String::new();
            response.read_to_string(&mut result);
            panic!("{} {}", response.status(), result);
        }
        response
    }

    fn build_auth_request<F>(&self, req_builder: F) -> Request
        where F: Fn(&String) -> String
    {
        let ld = self.login_data.as_ref().unwrap();
        let uri = req_builder(&ld.instance_url);
        let mut req = self.client.get(uri.as_str()).unwrap();
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer { token: ld.access_token.clone() }));
        req.headers(headers);
        req.build()
    }
}
