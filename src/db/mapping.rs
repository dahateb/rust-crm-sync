use std::collections::HashMap;

static DEFAULT: &str = "varchar";

lazy_static! {
    static ref TYPEMAP: HashMap<String, &'static str> = {
        let mut m = HashMap::new();
        m.insert("id".to_owned(), "varchar");
        m.insert("string".to_owned(), "varchar");
        m.insert("picklist".to_owned(), "varchar");
        m.insert("double".to_owned(), "double precision");
        m.insert("int".to_owned(), "integer");
        m.insert("datetime".to_owned(), "timestamp");
        m.insert("date".to_owned(), "timestamp");
        m.insert("boolean".to_owned(), "boolean");
        m
    };
}

pub fn sf_type_mapping(field_type: &String) -> Result<&'static str, String> {
    Ok(TYPEMAP.get(field_type).unwrap_or(&DEFAULT))
}
