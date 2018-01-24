#[derive(Debug)]
pub struct CreateQueryBuilder<'query> {
    table_name: &'query String,
    fields: Vec<String>
}

impl<'query> CreateQueryBuilder<'query> {

    pub fn new(object_name: & String) -> CreateQueryBuilder {
        CreateQueryBuilder {
            table_name: object_name,
            fields: Vec::new()
        }
    }

    pub fn add_field(&mut self, name: &'query str, field_type: String) {
        self.fields.push(format!("{} {}", name.to_lowercase(), field_type));
    }

    pub fn build(&self) -> String {
        let mut query = String::new();
        query.push_str("CREATE TABLE ");
        query.push_str(self.table_name.as_str());
        query.push_str("(");
        query.push_str(self.fields.join(",").as_str());
        query.push_str(")");
        query
    }
}

#[derive(Debug)]
pub struct UpdateQueryBuilder<'update> {
    table_name: &'update String,
    fields: Vec<String>,
    and_where: Vec<String>
}

impl<'update> UpdateQueryBuilder<'update> {
    
    pub fn new(object_name: & String) -> UpdateQueryBuilder {
        UpdateQueryBuilder {
            table_name: object_name,
            fields: Vec::new(),
            and_where: Vec::new()
        }
    }

    pub fn add_field(&mut self, name: &'update String, value: &'update String) {
        self.fields.push(format!("{}={}", name, escape_single_quote(value)));
    }

    pub fn add_and_where(&mut self, name: &'update str, value: &'update String, operator: String) {
        self.and_where.push(format!("{} {} '{}'", name, operator, escape_single_quote(value)));
    }

    pub fn build(&self) -> String {
        let mut query = String::new();
        query.push_str("UPDATE ");
        query.push_str(self.table_name.as_str());
        query.push_str(" SET ");
        query.push_str(self.fields.join(",").as_str());
        if self.and_where.len() > 0 {
            query.push_str(" WHERE ");
            query.push_str(self.and_where.join(" AND ").as_str());
        }
        
        query
    }
}

pub fn escape_single_quote(elem: &String) -> String {
    if elem.starts_with("'") && elem.ends_with("'") {
        let tmp = elem.as_str();
        let tmp_slice = &tmp[1..elem.len() - 1];
        let tmp_str = tmp_slice.to_string().replace("'", "''");
        return String::from("'") + tmp_str.as_str() + "'";
    }
    return elem.to_string();
}