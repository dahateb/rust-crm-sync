use postgres::{Connection, TlsMode};
use postgres::rows::Rows;
use salesforce::objects::{SObjectDescribe, Field};
use serde_json;

pub mod mapping;

#[derive(Debug)]
pub struct Db {
    conn: Connection
}

impl Db {
    
    pub fn new() -> Db {
        let conn = Connection::connect("postgres://postgres@localhost", TlsMode::None)
        .map_err(|err| panic!("DB Error: Cannot connect - {}", err.to_string()))
        .unwrap();
        Db {
            conn: conn
        }
    }

    pub fn save_config_data(&self, item: &SObjectDescribe ) {
        let field_json = serde_json::to_string(&item.fields).unwrap();
        self.conn.execute("INSERT INTO config.objects (name, fields) VALUES ($1, $2)",
                 &[&item.name, &field_json]).unwrap();
    }

    pub fn create_object_table(&self, object_name: &String, fields: Vec<Field>) {
        let mut query = format!("CREATE TABLE salesforce.{}", object_name.to_lowercase());
        query += "(";
        query += " id SERIAL,";
        query += " sfid  varchar(18),";
        for field in fields {
            let field_name = field.name.to_lowercase();
            let mut mapping = mapping::sf_type_mapping(field.sf_type).unwrap();
            if field_name == "id" {continue;}
            if mapping == "varchar" && field.length > 255 {mapping = "text"}
            query += &format!("{} {},", field_name, mapping);
        }

        query += " created timestamp,";
        query += " updated timestamp";
        query += ")";
        println!("{}", query);
        self.conn.execute(query.as_str(),&[])
        .unwrap();
    }

    pub fn get_selected_objects(&self) -> Vec<String> {
        let rows: Rows = self.conn.query("SELECT id, name, fields FROM config.objects", &[]).unwrap();
        return rows.iter()
        .map(|row| row.get(1))
        .collect();
    }

}