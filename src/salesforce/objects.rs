use serde_json::value::Value;

#[derive(Serialize, Deserialize)]
pub struct SObjectList{
    encoding: String, 
    pub sobjects: Vec<SObject>
}

#[derive(Serialize, Deserialize)]
pub struct SObject {
    label: String,
    pub createable: bool,
    pub updateable: bool,
    pub queryable: bool,
    pub layoutable: bool,
    #[serde(rename="customSetting")]
    pub custom_setting: bool,
    pub name: String
}

#[derive(Serialize, Deserialize)]
pub struct SObjectDescribe {
    label: String,
    pub createable: bool,
    pub updateable: bool,
    pub name: String,
    pub fields: Vec<Field>
}

#[derive(Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub length: u32,
    pub label: String,
    #[serde(rename="type")]
    pub sf_type: String
}

pub struct SObjectRowResultWrapper {
    pub rows: Vec<(Vec<String>, Vec<String>)>,
    pub object_name: String
}

impl SObjectRowResultWrapper {

    pub fn new(name: &String, fields: &Vec<Field>, rows: Value) -> SObjectRowResultWrapper {
        let rows_raw =  rows["records"].as_array().unwrap();
        let mut result: Vec<(Vec<String>, Vec<String>)> = Vec::new();
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
                }else {
                    field_names.push(field.name.to_lowercase().clone());
                }
                let value = &row[&field.name];
                match value {
                    //add single quotes for Strings
                    &Value::String(ref val) => {
                        let str_val = val.to_string().clone();
                        field_values.push(format!("'{}'", str_val));
                    },
                    _ => field_values.push(value.to_string().clone())
                }                
            }
            result.push((field_names, field_values));
        }    
        SObjectRowResultWrapper {
            rows: result,
            object_name: name.clone()
        }
    }
}