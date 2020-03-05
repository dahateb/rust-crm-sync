pub mod mapping;
pub mod objects;
pub mod query;
pub mod record;

use crate::config::DbConfig;
use crate::db::objects::ObjectConfig;
use crate::db::query::{
    escape_single_quote, get_lock_query, CreateQueryBuilder, UpdateQueryBuilder,
};
use crate::db::record::Record;
use crate::salesforce::objects::{Field, SObjectDescribe, SObjectRowResultWrapper};
use fallible_iterator::FallibleIterator;
use postgres::rows::Rows;
use r2d2::config::Builder;
use r2d2::Pool;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};
use serde_json;
use std::collections::HashMap;

pub struct Db {
    pub pool: Pool<PostgresConnectionManager>,
    config: &'static DbConfig,
}

impl Db {
    pub fn new(db_config: &'static DbConfig) -> Db {
        let config = Builder::new().pool_size(1).build();
        let manager = PostgresConnectionManager::new(db_config.url.clone(), TlsMode::None).unwrap();
        let pool = Pool::new(config, manager)
            .map_err(|err| panic!("DB Error: Cannot connect - {}", err.to_string()))
            .unwrap();
        println!("Connected to db: {}", db_config.url);
        Db {
            pool: pool,
            config: db_config,
        }
    }

    pub fn save_config_data(&self, item: &SObjectDescribe) {
        let field_json = serde_json::to_string(&item.fields).unwrap();
        let conn = self.pool.get().unwrap();
        conn.execute("INSERT INTO config.objects (name, db_name, fields, last_sync_time) VALUES ($1, $2, $3, now())",
                 &[&item.name, &item.name.to_lowercase(), &field_json]).unwrap();
    }

    pub fn create_object_table(
        &self,
        object_name: &String,
        fields: &Vec<Field>,
    ) -> Result<u64, postgres::Error> {
        let table_name = format!("salesforce.{}", object_name);
        let mut query_builder = CreateQueryBuilder::new(&table_name);
        query_builder.add_field("id", "SERIAL PRIMARY KEY".to_string());
        query_builder.add_field("sfid", "varchar(18)".to_string());
        for field in fields {
            if field.name == "Id" || field.sf_type == "address" {
                continue;
            }
            let mapping = mapping::sf_type_mapping(&field).unwrap();
            query_builder.add_field(field.name.as_str(), mapping);
        }
        query_builder.add_field("_s_error", "TEXT".to_string());
        query_builder.add_field("_s_state", "varchar(20) DEFAULT 'OK'".to_string());
        query_builder.add_field("_s_created", "TIMESTAMP DEFAULT NOW()".to_string());
        query_builder.add_field("_s_updated", "TIMESTAMP".to_string());
        let query = query_builder.build();

        // println!("{}", query);
        let conn = self.pool.get().unwrap();
        conn.execute(query.as_str(), &[])
    }

    pub fn add_channel_trigger(&self, object_name: &String) {
        let query = format!(
            "CREATE TRIGGER {}_notify
         AFTER INSERT OR UPDATE 
         ON salesforce.{}
         FOR EACH ROW
         EXECUTE PROCEDURE salesforce.notify_change();",
            object_name, object_name
        );
        let conn = self.pool.get().unwrap();
        conn.execute(query.as_str(), &[]).unwrap();
    }

    pub fn get_selected_objects(&self, interval: i16) -> Result<Vec<ObjectConfig>, String> {
        let conn = self.pool.get().unwrap();
        let query = format!("SELECT id, name, fields, last_sync_time FROM config.objects WHERE last_sync_time < current_timestamp - interval '{} minutes'",
                            interval);
        let rows: Rows = conn.query(query.as_str(), &[]).unwrap();
        debug!("get_selected_objects: Num rows: {}", rows.len());
        let result = rows
            .iter()
            .map(|row| {
                let name: String = row.get(1);
                let query = format!(
                    "SELECT count(*)::int FROM salesforce.{:?}",
                    name.to_lowercase()
                );
                let count_rows: Rows = conn.query(query.as_str(), &[]).unwrap();
                let count: i32 = count_rows.get(0).get(0);
                ObjectConfig::new(row.get(0), name, count as u32, row.get(2))
            })
            .collect();
        Ok(result)
    }

    pub fn get_object_config(&self, object_name: &String) -> Option<ObjectConfig> {
        let conn = self.pool.get().unwrap();
        let query = "SELECT id, db_name, fields FROM config.objects WHERE db_name = $1";
        let rows = conn.query(query, &[&object_name.to_lowercase()]).unwrap();
        if rows.len() == 0 {
            return None;
        }
        let row = rows.iter().next().unwrap();
        let config: ObjectConfig = ObjectConfig::new(row.get(0), row.get(1), 0, row.get(2));
        Some(config)
    }

    pub fn get_object_data_by_id(
        &self,
        object_name: &String,
        ids: &Vec<i32>,
    ) -> Result<Vec<Record>, String> {
        let conn = self.pool.get().unwrap();
        let mut config;
        match self.get_object_config(object_name) {
            Some(conf) => config = conf,
            None => return Err("Object not found".to_owned()),
        }
        config.count = ids.len() as u32;
        let fieldnames = config.get_db_field_names();
        let mut query = format!(
            "SELECT id, sfid, {} FROM salesforce.{}",
            fieldnames.join(","),
            object_name
        );
        if ids.len() > 0 {
            query.push_str(" WHERE id IN(");
            let mut tmp = vec![];
            ids.iter().for_each(|id| {
                tmp.push(format!("{}", id));
            });
            query.push_str(tmp.join(",").as_str());
            query.push_str(")");
        }
        let result = conn.query(&query, &[]).unwrap();
        let mut res = vec![];
        for row in result.iter() {
            //println!("{:?}",row);
            let record = Record::new(&row);
            //println!("{}",record.get_json());
            res.push(record);
        }
        Ok(res)
    }

    pub fn update_last_sync_time(&self, id: i32) {
        let conn = self.pool.get().unwrap();
        let _result = conn.query(
            "Update config.objects set last_sync_time = now() WHERE id = $1",
            &[&id],
        );
    }

    pub fn set_error_state(&self, object_name: &str, id: &i32, error: &str) {
        let id_str = id.to_string();
        let error_str = format!("'{}'", error);
        let table_name = format!("salesforce.{}", object_name);
        let mut builder = UpdateQueryBuilder::new(&table_name);
        builder.add_field("_s_error", &error_str);
        builder.add_field("_s_state", "'ERROR'");
        builder.add_and_where("id", &id_str, "=");
        let query = builder.build();
        println!("{}", query);
        let _ = self.query_with_lock(&query, object_name);
    }

    pub fn upsert_object_rows(&self, wrapper: &SObjectRowResultWrapper) -> Result<u64, String> {
        let mut count = 0;
        for id in wrapper.rows.keys() {
            let mut result =
                { self.update_rows(id, &wrapper.object_name, &wrapper.rows.get(id).unwrap()) }?;
            if result == 0 {
                result = { self.insert_rows(&wrapper.object_name, &wrapper.rows.get(id).unwrap()) }?
            }
            count += result;
        }
        Ok(count)
    }

    pub fn populate(&self, wrapper: &SObjectRowResultWrapper) -> Result<u64, String> {
        let mut count = 0;
        for row in wrapper.rows.values() {
            count += (self
                .insert_rows(&wrapper.object_name, &row)
                .map_err(|err| err.to_string()))?;
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

    pub fn update_ids(&self, object_name: &String, ids_map: &HashMap<i32, String>) {
        let mut id_str;
        let mut sfid;
        let table_name = format!("salesforce.{}", object_name);
        for id in ids_map.keys() {
            let mut builder = UpdateQueryBuilder::new(&table_name);
            id_str = id.to_string();
            sfid = format!("'{}'", ids_map[id]);
            builder.add_field("sfid", &sfid);
            builder.add_and_where("id", &id_str, "=");
            let _ = self.query_with_lock(&builder.build(), object_name);
        }
    }

    fn insert_rows(
        &self,
        object_name: &String,
        row: &(Vec<String>, Vec<String>),
    ) -> Result<u64, String> {
        let row_values = row
            .1
            .iter()
            .map(|val| escape_single_quote(&val))
            .collect::<Vec<String>>();
        let query = format!(
            "INSERT INTO salesforce.{} ({}) VALUES ({});",
            object_name,
            row.0.join(","),
            row_values.join(",")
        );
        //println!("{}", query);
        self.query_with_lock(&query, &object_name)
    }

    fn update_rows(
        &self,
        id: &String,
        object_name: &String,
        row: &(Vec<String>, Vec<String>),
    ) -> Result<u64, String> {
        let table_name = format!("salesforce.{}", object_name);
        let mut builder = UpdateQueryBuilder::new(&table_name);
        for i in 0..row.0.len() {
            builder.add_field(&row.0[i], &row.1[i]);
        }
        builder.add_and_where("sfid", id, "=");
        let query = builder.build();
        // println!("{}", query);
        self.query_with_lock(&query, &object_name)
    }

    fn query_with_lock(&self, query: &String, object_name: &str) -> Result<u64, String> {
        //add channel lock flag here
        let conn = self.pool.get().unwrap();
        let _ = (conn
            .execute(&get_lock_query(object_name, true), &[])
            .map_err(|err| err.to_string()))?;
        let result = (conn.execute(&query, &[]).map_err(|err| err.to_string()))?;
        let _ = (conn
            .execute(&get_lock_query(object_name, false), &[])
            .map_err(|err| err.to_string()))?;
        Ok(result)
    }

    pub fn get_notifications(&self) -> Vec<String> {
        let mut result = vec![];
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
        } else {
            let _ = conn.execute("UNLISTEN salesforce_data", &[]);
        }
    }
}

impl Clone for Db {
    fn clone(&self) -> Db {
        Db::new(self.config)
    }
}
