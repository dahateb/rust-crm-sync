use serde_json;
use salesforce::objects::{Field, SObjectConfiguration};

#[derive(Debug)]
pub struct ObjectConfig {
    pub id: i32,
    pub name: String,
    pub count: u32,
    pub fields: Vec<Field>,
}

impl ObjectConfig {
    pub fn new(id: i32, name: String, count: u32, fields: String) -> ObjectConfig {
        let field_list: Vec<Field> = serde_json::from_str(fields.as_str()).unwrap();
        ObjectConfig {
            id: id,
            name: name,
            count: count as u32,
            fields: field_list,
        }
    }

    pub fn get_field_names(&self) -> Vec<String> {
        self.fields.iter().map(|field| field.name.clone()).collect()
    }

    pub fn get_db_field_names(&self) -> Vec<String> {
        self.fields.iter()
        .filter(|field| field.updateable && field.name != "Id")
        .map(|field| field.name.to_lowercase())
        .collect()
    }
}

impl SObjectConfiguration for ObjectConfig {
    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_fields(&self) -> &Vec<Field> {
        &self.fields
    }
}
