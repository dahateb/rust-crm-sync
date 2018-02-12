use std::str;
use std::ops::Sub;
use serde_json::{self, Value};
use config::SalesforceConfig;
use self::objects::{SObject, SObjectList, SObjectDescribe, SObjectConfiguration,
                    SObjectRowResultWrapper};
use chrono::prelude::*;
use time::Duration;

pub mod objects;
pub mod client;

use salesforce::client::Client;
use db::objects::ObjectConfig;

pub struct Salesforce {
    config: &'static SalesforceConfig,
    client: Client,
}

impl Salesforce {
    pub fn new(config: &'static SalesforceConfig) -> Salesforce {
        let client: Client = Client::new(None).connect(config);
        client.print_login_data();
        Salesforce {
            config: config,
            client: client,
        }
    }
    pub fn get_objects(&self) -> Result<Vec<SObject>, String> {
        let req_builder = |uri: &String| format!("{}/services/data/v40.0/sobjects", uri);
        let posted_str = self.client.get_resource(req_builder).unwrap();
        let list: SObjectList = serde_json::from_str(posted_str.as_str()).unwrap();
        let filtered_list: Vec<SObject> = list.sobjects
            .into_iter()
            .filter(|x| (x.createable && x.queryable && x.layoutable) || x.custom_setting)
            .collect();
        Ok(filtered_list)
    }

    pub fn describe_object(&self, object_name: &str) -> Result<SObjectDescribe, String> {
        let req_builder = |uri: &String| {
            format!("{}/services/data/v40.0/sobjects/{}/describe",
                    uri,
                    object_name)
        };
        let posted_str = self.client.get_resource(req_builder).unwrap();
        let object: SObjectDescribe = serde_json::from_str(posted_str.as_str()).unwrap();
        Ok(object)
    }

    pub fn get_last_updated_records(&self,
                                    object_config: &ObjectConfig,
                                    time_sec: i64)
                                    -> Result<SObjectRowResultWrapper, String> {
        let date_diff: DateTime<Utc> = Utc::now().sub(Duration::minutes(time_sec));
        let query = format!("SELECT+{}+FROM+{}+WHERE+lastmodifieddate>{}",
                            object_config.get_field_names().join(","),
                            object_config.name,
                            date_diff.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        //println!("{}",query);
        let req_builder = |uri: &String| format!("{}/services/data/v40.0/query/?q={}", uri, query);
        let posted_str = self.client.get_resource(req_builder).unwrap();
        //println!("{}",posted_str);
        let v: Value = serde_json::from_str(posted_str.as_str()).unwrap();
        if !v["records"].is_array() {
            return Err("Error fetching data".to_owned());
        }
        Ok(SObjectRowResultWrapper::new(&object_config.name, &object_config.fields, v))
    }

    pub fn get_records_from_describe(&self,
                                     describe: &SObjectConfiguration,
                                     object_name: &String)
                                     -> Result<SObjectRowResultWrapper, String> {
        let all_fields: Vec<String> = describe
            .get_fields()
            .iter()
            .map(|field| field.name.clone())
            .collect();
        let query = format!("SELECT+{}+FROM+{}", all_fields.join(","), object_name);
        //println!("{}",query);
        let req_builder = |uri: &String| format!("{}/services/data/v40.0/query/?q={}", uri, query);
        let posted_str = try!(self.client
                                  .get_resource(req_builder)
                                  .map_err(|err| err.to_string()));
        //println!("{}",posted_str);
        let v: Value = serde_json::from_str(posted_str.as_str()).unwrap();
        Ok(SObjectRowResultWrapper::new(&describe.get_name(), &describe.get_fields(), v))
    }

    pub fn get_next_records(&self,
                            describe: &SObjectConfiguration,
                            wrapper: &SObjectRowResultWrapper)
                            -> Option<SObjectRowResultWrapper> {
        if wrapper.done {
            return None;
        }
        let req_builder = |uri: &String| format!("{}{}", uri, wrapper.next_url);
        let posted_str = self.client
            .get_resource(req_builder)
            .map_err(|err| err.to_string());
        match posted_str {
            Ok(res) => {
                let result: Value = serde_json::from_str(res.as_str()).unwrap();
                return Some(SObjectRowResultWrapper::new(&describe.get_name(),
                                                         &describe.get_fields(),
                                                         result));
            }
            Err(str) => {
                return None;
            }
        }
    }
}
