use postgres::{Connection, TlsMode};
use postgres::rows::Rows;
use salesforce::objects::SObjectDescribe;
use serde_json;

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

    pub fn get_selected_objects(&self) -> Vec<String> {
        let rows: Rows = self.conn.query("SELECT id, name, fields FROM config.objects", &[]).unwrap();
        return rows.iter()
        .map(|row| row.get(1))
        .collect();
    }

}