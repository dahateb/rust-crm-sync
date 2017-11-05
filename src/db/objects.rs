use serde_json;
use serde_json::value::Value;
use salesforce::objects::Field;

pub struct ObjectConfig {
    pub id: i32,
    pub name: String,
    pub count: u32,
    pub fields: Vec<Field>
}

impl ObjectConfig {

    pub fn new(id: i32, name: String, count: u32, fields: String) -> ObjectConfig {
        let field_list: Vec<Field> = serde_json::from_str(fields.as_str()).unwrap();
        ObjectConfig {
               id: id,
               name: name,
               count: count as u32,
               fields: field_list
        } 
    }

    pub fn get_field_names(&self) -> Vec<String> {
        self.fields.iter().map(|field|{
            field.name.clone()
        }).collect()
    }
}