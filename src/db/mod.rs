use postgres::rows::Rows;
use salesforce::objects::{SObjectDescribe, Field};
use serde_json;
use r2d2_postgres::{TlsMode, PostgresConnectionManager};
use r2d2::{Config,Pool};
use config::DbConfig;

pub mod mapping;
pub mod objects;

use db::objects::ObjectConfig;

#[derive(Debug)]
pub struct Db {
    pool: Pool<PostgresConnectionManager>
}

impl Db {
    
    pub fn new(db_config: &'static DbConfig) -> Db {
        let config = Config::default();
        let manager = PostgresConnectionManager::new(db_config.url.clone(),
                                                     TlsMode::None).unwrap();
        let pool = Pool::new(config, manager)
        .map_err(|err| panic!("DB Error: Cannot connect - {}", err.to_string()))
        .unwrap();
        Db {
            pool: pool
        }
    }

    pub fn save_config_data(&self, item: &SObjectDescribe ) {
        let field_json = serde_json::to_string(&item.fields).unwrap();
        let conn = self.pool.get().unwrap();
        conn.execute("INSERT INTO config.objects (name, fields) VALUES ($1, $2)",
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
        let conn = self.pool.get().unwrap();
        conn.execute(query.as_str(),&[])
        .unwrap();
    }

    pub fn get_selected_objects(&self, interval: i16) -> Vec<ObjectConfig> {
        let conn = self.pool.get().unwrap();
        let query = format!("SELECT id, name, fields, last_sync_time FROM config.objects WHERE last_sync_time < current_timestamp - interval '{} minutes'", interval);
        let rows: Rows = conn.query(query.as_str(), &[]).unwrap();
        return rows.iter()
        .map(|row| {
           ObjectConfig {
               id: row.get(0),
               name: row.get(1)
           } 
        })
        .collect();
    }

    pub fn update_last_sync_time(&self, id: i32 ) {
        let conn = self.pool.get().unwrap();
        conn.query("Update config.objects set last_sync_time = now() WHERE id = $1", &[&id]);
    }
}