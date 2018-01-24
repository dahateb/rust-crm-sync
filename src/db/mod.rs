pub mod mapping;
pub mod objects;
pub mod query;

use postgres::rows::Rows;
use salesforce::objects::{SObjectDescribe, Field, SObjectRowResultWrapper};
use serde_json;
use r2d2_postgres::{TlsMode, PostgresConnectionManager};
use r2d2::{Pool};
use r2d2::config::Builder;
use config::DbConfig;
use fallible_iterator::FallibleIterator;
use db::query::{CreateQueryBuilder, UpdateQueryBuilder, escape_single_quote};
use db::objects::ObjectConfig;

#[derive(Debug)]
pub struct Db {
    pub pool: Pool<PostgresConnectionManager>
}

impl Db {
    pub fn new(db_config: &'static DbConfig) -> Db {
        let config = Builder::new().pool_size(1).build();
        let manager = PostgresConnectionManager::new(db_config.url.clone(), TlsMode::None).unwrap();
        let pool = Pool::new(config, manager)
            .map_err(|err| panic!("DB Error: Cannot connect - {}", err.to_string()))
            .unwrap();
        Db { pool: pool}
    }

    pub fn save_config_data(&self, item: &SObjectDescribe) {
        let field_json = serde_json::to_string(&item.fields).unwrap();
        let conn = self.pool.get().unwrap();
        conn.execute("INSERT INTO config.objects (name, fields, last_sync_time) VALUES ($1, $2, now())",
                 &[&item.name, &field_json]).unwrap();
    }

    pub fn create_object_table(&self, object_name: &String, fields: &Vec<Field>) {
        let table_name = format!("salesforce.{}",object_name);
        let mut query_builder = CreateQueryBuilder::new(&table_name);
        query_builder.add_field("id", "SERIAL PRIMARY KEY".to_string());
        query_builder.add_field( "sfid", "varchar(18)".to_string());
        for field in fields {
            if field.name == "Id" || field.sf_type == "address" {
                continue;
            }
            let mapping = mapping::sf_type_mapping(&field.sf_type, field.length).unwrap();
            query_builder.add_field( field.name.as_str(), mapping);
        }
        query_builder.add_field("created", "timestamp".to_string());
        query_builder.add_field("updated", "timestamp".to_string());
        let query = query_builder.build();
        
        // println!("{}", query);
        let conn = self.pool.get().unwrap();
        conn.execute(query.as_str(), &[]).unwrap();
    }

    pub fn add_channel_trigger(&self, object_name: &String) {
        let query = format!(
            "CREATE TRIGGER {}_notify
         AFTER INSERT OR UPDATE 
         ON salesforce.{}
         FOR EACH ROW
         EXECUTE PROCEDURE salesforce.notify_change();",
            object_name,
            object_name
        );
        let conn = self.pool.get().unwrap();
        conn.execute(query.as_str(), &[]).unwrap();
    }

    pub fn get_selected_objects(&self, interval: i16) -> Result<Vec<ObjectConfig>, String> {
        let conn = self.pool.get().unwrap();
        let query = format!("SELECT id, name, fields, last_sync_time FROM config.objects WHERE last_sync_time < current_timestamp - interval '{} minutes'",
                            interval);
        let rows: Rows = conn.query(query.as_str(), &[]).unwrap();
        let result = rows.iter()
            .map(|row| {
                     let name: String = row.get(1);
                     let query = format!("SELECT count(*)::int FROM salesforce.{:?}",
                                         name.to_lowercase());
                     let count_rows: Rows = conn.query(query.as_str(), &[]).unwrap();
                     let count: i32 = count_rows.get(0).get(0);
                     ObjectConfig::new(row.get(0), name, count as u32, row.get(2))
                 })
            .collect();
        Ok(result)
    }

    pub fn update_last_sync_time(&self, id: i32) {
        let conn = self.pool.get().unwrap();
        let _result = conn.query("Update config.objects set last_sync_time = now() WHERE id = $1",
                                 &[&id]);
    }

    pub fn upsert_object_rows(&self, wrapper: &SObjectRowResultWrapper) -> Result<u64, String> {
        let mut count = 0;
        for id in wrapper.rows.keys() {
            let mut result =
                try!{
                self.update(id, &wrapper.object_name, &wrapper.rows.get(id).unwrap())
            };
            if result == 0 {
                result =
                    try!{
                    self.insert(&wrapper.object_name, &wrapper.rows.get(id).unwrap())
                }
            }
            count += result;
        }
        Ok(count)
    }

    pub fn populate(&self, wrapper: &SObjectRowResultWrapper) -> Result<u64, String> {
        let mut count = 0;
        for row in wrapper.rows.values() {
            count += try!(self.insert(&wrapper.object_name, &row)
                              .map_err(|err| err.to_string()));
        }
        Ok(count)
    }

    pub fn destroy(&self, id: i32, name: &String) {
        let query = format!("DROP TABLE salesforce.{}", name.to_lowercase());
        let conn = self.pool.get().unwrap();
        let _result = conn.execute(query.as_str(), &[]).unwrap();
        let query = format!("DELETE FROM config.objects where id = {}", id);
        let _result = conn.execute(query.as_str(), &[]).unwrap();
    }

    fn insert(&self,
              object_name: &String,
              row: &(Vec<String>, Vec<String>))
              -> Result<u64, String> {
        let row_values = row.1
            .iter()
            .map(escape_single_quote)
            .collect::<Vec<String>>();
        let query = format!("INSERT INTO salesforce.{} ({}) VALUES ({});",
                            object_name,
                            row.0.join(","),
                            row_values.join(","));
        //println!("{}", query);
        let result = self.query_with_lock(&query, &object_name);
        //println!("{:?}", result);
        result
    }

    fn update(&self,
              id: &String,
              object_name: &String,
              row: &(Vec<String>, Vec<String>))
              -> Result<u64, String> {
        let table_name = format!("salesforce.{}",object_name);
        let mut builder = UpdateQueryBuilder::new(&table_name);
        for i in 0..row.0.len() {
            builder.add_field(&row.0[i], &row.1[i]);
        }
        builder.add_and_where("sfid", id, "=".to_string());
        let query = builder.build();
        // println!("{}", query);
        let result = self.query_with_lock(&query, &object_name);
        // println!("{:?}", result);
        result
    }

    fn query_with_lock(&self, query: &String, object_name: &String) -> Result<u64, String>{
        //add channel lock flag here
        let conn = self.pool.get().unwrap();
        let _ = conn.execute(&format!("SELECT set_config('salesforce.{}_lock','lock', false);", object_name), &[]);
        let result = try!(conn.execute(&query, &[])
                              .map_err(|err| err.to_string()));
        let _ = conn.execute(&format!("SELECT set_config('salesforce.{}_lock','', false);", object_name), &[]);
        Ok(result)
    }

    pub fn get_notifications(&self) -> Vec<String>{
        let mut result = vec!();
        let conn = self.pool.get().unwrap();
        let _ = conn.query("", &[]); 
        let notifications = conn.notifications();
        let mut iter = notifications.iter();
        while let Some(note) = iter.next().unwrap() {
            result.push(note.payload);
        }
        result
    }

    pub fn toggle_listen(&self, listening: bool) {
        let conn = self.pool.get().unwrap();
        if listening {
            let _ = conn.execute("LISTEN salesforce_data", &[]);
        }else {
            let _ = conn.execute("UNLISTEN salesforce_data", &[]);
        }
        
    }
}