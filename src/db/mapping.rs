use std::collections::HashMap;

static DEFAULT: &str = "varchar";

lazy_static! {
    static ref TYPEMAP: HashMap<String, &'static str> = {
        let mut m = HashMap::new();
        m.insert("id".to_owned(), "varchar");
        m.insert("string".to_owned(), "varchar");
        m.insert("picklist".to_owned(), "varchar");
        m.insert("double".to_owned(), "double precision");
        m.insert("currency".to_owned(), "double precision");
        m.insert("percent".to_owned(), "double precision");
        m.insert("int".to_owned(), "integer");
        m.insert("datetime".to_owned(), "timestamp");
        m.insert("date".to_owned(), "date");
        m.insert("boolean".to_owned(), "boolean");
        m
    };
}

pub fn sf_type_mapping(field_type: &String, length: u32) -> Result<String, String> {
    let db_type = TYPEMAP.get(field_type).unwrap_or(&DEFAULT);
    match *db_type {
        "varchar" => {
            if length > 255 {
                return Ok(String::from("text"));
            }
            Ok(format!("{}({})", db_type, length))
        }
        _ => Ok(db_type.to_string()),
    }
}
