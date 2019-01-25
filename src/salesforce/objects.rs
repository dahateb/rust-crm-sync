use serde_json::value::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct SObjectList {
    encoding: String,
    pub sobjects: Vec<SObject>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SObject {
    pub label: String,
    pub createable: bool,
    pub updateable: bool,
    pub queryable: bool,
    pub layoutable: bool,
    #[serde(rename = "customSetting")]
    pub custom_setting: bool,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SObjectDescribe {
    label: String,
    pub createable: bool,
    pub updateable: bool,
    pub name: String,
    pub fields: Vec<Field>,
}

pub trait SObjectConfiguration {
    fn get_name(&self) -> &String;
    fn get_fields(&self) -> &Vec<Field>;
}

impl SObjectConfiguration for SObjectDescribe {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_fields(&self) -> &Vec<Field> {
        &self.fields
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Field {
    pub name: String,
    pub length: u32,
    pub label: String,
    #[serde(rename = "type")]
    pub sf_type: String,
    pub updateable: bool,
    pub calculated: bool,
}

pub struct SObjectRowResultWrapper {
    pub rows: HashMap<String, (Vec<String>, Vec<String>)>,
    pub object_name: String,
    pub next_url: String,
    pub done: bool,
}

impl SObjectRowResultWrapper {
    pub fn new(
        name: &String,
        fields: &Vec<Field>,
        describe_result: Value,
    ) -> SObjectRowResultWrapper {
        let rows_raw = describe_result["records"].as_array().unwrap();
        let mut result: HashMap<String, (Vec<String>, Vec<String>)> = HashMap::new();
        for row in rows_raw {
            let mut field_names: Vec<String> = Vec::new();
            let mut field_values: Vec<String> = Vec::new();

            for field in fields {
                //filter compound address type
                if field.sf_type == "address" {
                    continue;
                }

                if field.name == "Id" {
                    field_names.push("sfid".to_owned());
                } else {
                    field_names.push(field.name.to_lowercase().clone());
                }
                let value = &row[&field.name];
                match value {
                    //add single quotes for Strings
                    &Value::String(ref val) => {
                        let mut str_val = String::new();
                        str_val.push_str("'");
                        str_val.push_str(val.as_str());
                        str_val.push_str("'");
                        field_values.push(str_val);
                    }
                    _ => field_values.push(value.to_string().clone()),
                }
            }
            let id = row["Id"].as_str().unwrap().to_owned();
            result.insert(id, (field_names, field_values));
        }
        SObjectRowResultWrapper {
            rows: result,
            object_name: name.clone(),
            next_url: describe_result["nextRecordsUrl"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            done: describe_result["done"].as_bool().unwrap_or(false),
        }
    }
}
