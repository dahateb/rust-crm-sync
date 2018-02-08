use chrono::NaiveDateTime;
use postgres::rows::Row;
use postgres::types::{TEXT, INT4, INT8, VARCHAR, FLOAT8, FLOAT4, BOOL, TIMESTAMP};
use std::collections::HashMap;
use serde_json;

#[derive(Serialize)]
#[serde(untagged)]
enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    STR(String)
}

pub struct Record {
    pub id: String,
    data: HashMap<String,Option<Value>>
}

impl Record {

    pub fn new(row: &Row) -> Record{
        Record{
            id: row.get(0),
            data: Record::parse_data(row)
        }
    }

    fn parse_data(row: &Row) -> HashMap<String, Option<Value>>{
        let mut map = HashMap::new();
        let mut idx = 0;
        for column in row.columns().iter() {
            //println!("{:?}", column);
            let value = match column.type_() {
                &INT4 => {
                    match row.get::<_, Option<i32>>(idx) {
                        Some(val) => Some(Value::I32(val)),
                        None => None
                    }
                },
                &INT8 => {
                    match row.get::<_, Option<i64>>(idx) {
                        Some(val) => Some(Value::I64(val)),
                        None => None
                    }
                },
                &TEXT | &VARCHAR => {
                    match row.get::<_, Option<String>>(idx) {
                        Some(d) => Some(Value::STR(d)),
                        None => None
                    }
                },
                &FLOAT8 => {
                    match row.get::<_, Option<f64>>(idx) {
                        Some(val) => Some(Value::F64(val)),
                        None => None
                    }
                },
                &FLOAT4 => {
                    match row.get::<_, Option<f32>>(idx) {
                        Some(val) => Some(Value::F32(val)),
                        None => None
                    }
                },
                &BOOL => Some(Value::Bool(row.get::<_, bool>(idx))),
                &TIMESTAMP =>  {
                    match row.get::<_, Option<NaiveDateTime>>(idx) {
                        Some(d) => Some(Value::STR(d.to_string())),
                        None => None
                    }
                },
                _ => {
                    match row.get::<_, Option<String>>(idx) {
                        Some(d) => Some(Value::STR(d)),
                        None => None
                    }
                },  
            };
            
            map.insert(column.name().to_string(), value);
            idx += 1;
        }
        map
    }

    pub fn to_json(&self) -> String{
        serde_json::to_string(&self.data).unwrap()
    }
}