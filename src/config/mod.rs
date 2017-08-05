use serde_json;
use std::io::prelude::*;
use std::fs::File;

#[derive(Serialize, Deserialize)]
pub struct SalesforceConfig {
    pub uri: String,
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
    pub sec_token: String
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub salesforce : SalesforceConfig
}

impl Config {
    pub fn new(file: &str) -> Result<Self, String> {

        let mut file = File::open(file)
        .map_err(|err| format!("Problem while loading config: {}", err))
        .unwrap();
        let mut input = String::new();
        let size = file.read_to_string(&mut input);
        println!("Read {:?} bytes", size);
        let config: Config = serde_json::from_str(input.as_str())
        .map_err(|e| format!("Could not parse JSON: {}", e))?;
        
        return Ok(config);
    }
}