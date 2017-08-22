use std::str::{self};
use std::ops::Sub;
use serde_json::{self,Value};
use config::{SalesforceConfig};
use self::objects::{SObject, SObjectList, SObjectDescribe};
use chrono::prelude::*;
use time::Duration;
use std::rc::Rc;

pub mod objects;
pub mod client;
pub mod pool;

use salesforce::client::Client;
use salesforce::pool::SalesforceConnectionManager;

pub struct Salesforce {
    config: Rc<SalesforceConfig>,
    client: Client
}

impl Salesforce {
    
    pub fn new(config: Rc<SalesforceConfig>) -> Salesforce {
        let client: Client = Client::new(None).connect(config.clone());
        client.print_login_data();
        Salesforce {
            config : config,
            client: client
        }
    }
    pub fn get_objects(& self) -> Result<Vec<SObject>,String> {
        let req_builder = |uri : &String| {
            format!(
                "{}/services/data/v40.0/sobjects", 
                uri
            )
        };
        let posted_str = self.client.get_resource(req_builder).unwrap();
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
        let posted_str = self.client.get_resource(req_builder).unwrap();
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
        let posted_str = self.client.get_resource(req_builder).unwrap();
        let v: Value = serde_json::from_str(posted_str.as_str()).unwrap();
        let fields = vec!["Id","Name"];
        for field in fields {
            println!("{}", v["records"][0][field]);
        }
        
    }
}

