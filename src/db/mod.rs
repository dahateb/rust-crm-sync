use postgres::rows::Rows;
use salesforce::objects::{SObjectDescribe, Field, SObjectRowResultWrapper};
use serde_json;
use serde_json::value::Value;
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
        conn.execute("INSERT INTO config.objects (name, fields, last_sync_time) VALUES ($1, $2, now())",
                 &[&item.name, &field_json]).unwrap();
    }

    pub fn create_object_table(&self, object_name: &String, fields: &Vec<Field>) {
        let mut query = format!("CREATE TABLE salesforce.{}", object_name.to_lowercase());
        query += "(";
        query += " id SERIAL,";
        query += " sfid  varchar(18),";
        for field in fields {
            let field_name = field.name.to_lowercase();
            let sf_type = &field.sf_type;
            let mut mapping = mapping::sf_type_mapping(sf_type).unwrap();
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

    pub fn get_selected_objects(&self, interval: i16) -> Result<Vec<ObjectConfig>, String> {
        let conn = self.pool.get().unwrap();
        let query = format!("SELECT id, name, fields, last_sync_time FROM config.objects WHERE last_sync_time < current_timestamp - interval '{} minutes'", interval);
        let rows: Rows = conn.query(query.as_str(), &[]).unwrap();
        let result = rows.iter()
        .map(|row| {
           let name: String = row.get(1);
           let query = format!("SELECT count(*)::int FROM salesforce.{:?}", name.to_lowercase()); 
           let count_rows: Rows = conn.query(query.as_str(), &[]).unwrap();
           let count: i32 = count_rows.get(0).get(0) ;
           ObjectConfig::new(
               row.get(0),
               name,
               count as u32,
               row.get(2)
           )
        })
        .collect();
        Ok(result)
    }

    pub fn update_last_sync_time(&self, id: i32 ) {
        let conn = self.pool.get().unwrap();
        let _result = conn.query("Update config.objects set last_sync_time = now() WHERE id = $1", &[&id]);
    }

    pub fn upsert_object_rows(&self, wrapper: &SObjectRowResultWrapper) {
        let conn = self.pool.get().unwrap();

    }

    pub fn populate(&self, wrapper: &SObjectRowResultWrapper) -> u32{
        let mut count = 0;
        for row in &wrapper.rows {
            let query = format!(
                "INSERT INTO salesforce.{} ({}) VALUES ({});",
                wrapper.object_name,
                row.0.join(","),
                row.1.join(",")
            );
            println!("{}", query);
            let conn = self.pool.get().unwrap();
            conn.execute(query.as_str(),&[])
            .unwrap();
            count += 1;
        };
        count
    }

    pub fn destroy(&self, id: i32, name: &String) {
        let query = format!("DROP TABLE salesforce.{}", name.to_lowercase());
        let conn = self.pool.get().unwrap();
        let _result = conn.execute(query.as_str(),&[]).unwrap();
        let query = format!("DELETE FROM config.objects where id = {}", id);
        let _result = conn.execute(query.as_str(),&[]).unwrap();
    }
}