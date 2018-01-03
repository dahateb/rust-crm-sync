use postgres::rows::Rows;
use salesforce::objects::{SObjectDescribe, Field, SObjectRowResultWrapper};
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
        // println!("{}", query);
        let conn = self.pool.get().unwrap();
        conn.execute(query.as_str(),&[]).unwrap();
    }

    pub fn add_channel_trigger(&self, object_name: &String) {
        let query = format!("CREATE TRIGGER {}_notify
         AFTER INSERT OR UPDATE 
         ON salesforce.{}
         FOR EACH ROW
         EXECUTE PROCEDURE salesforce.notify_change();", object_name, object_name);
         let conn = self.pool.get().unwrap();
         conn.execute(query.as_str(),&[]).unwrap();
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

    pub fn upsert_object_rows(&self, wrapper: &SObjectRowResultWrapper)
        ->Result<u64, String> {
        let mut count = 0;
        for id in wrapper.rows.keys() {
            let mut result = try!{
                self.update(id, &wrapper.object_name, &wrapper.rows.get(id).unwrap())
            };
            if result == 0 {
                result = try!{
                    self.insert(&wrapper.object_name, &wrapper.rows.get(id).unwrap())
                }
            }
            count += result;
        }
        Ok(count)
    }

    pub fn populate(&self, wrapper: &SObjectRowResultWrapper) -> Result<u64,String>{
        let mut count  = 0;
        for row in wrapper.rows.values() {
            count += try!(
                self.insert(&wrapper.object_name, &row)
                    .map_err(|err|err.to_string())
            );
        };
        Ok(count)
    }

    pub fn destroy(&self, id: i32, name: &String) {
        let query = format!("DROP TABLE salesforce.{}", name.to_lowercase());
        let conn = self.pool.get().unwrap();
        let _result = conn.execute(query.as_str(),&[]).unwrap();
        let query = format!("DELETE FROM config.objects where id = {}", id);
        let _result = conn.execute(query.as_str(),&[]).unwrap();
    }

    fn insert(&self, object_name: &String, row: &(Vec<String>, Vec<String>)) 
        -> Result<u64,String>{
        let row_values = row.1.iter()
            .map(escape_single_quote)
            .collect::<Vec<String>>();
        let query = format!(
                "INSERT INTO salesforce.{} ({}) VALUES ({});",
                object_name,
                row.0.join(","),
                row_values.join(",")
        );
        //println!("{}", query);
        let conn = self.pool.get().unwrap();
        //add channel lock flag here
        let result = try!(conn.execute(query.as_str(),&[]).map_err(|err| err.to_string()));
        //println!("{:?}", result);
        Ok(result)
    }

    fn update(&self, id: &String, object_name: &String, row: &(Vec<String>, Vec<String>)) 
        -> Result<u64, String> {
        let mut query = String::from("UPDATE salesforce.") + &object_name.to_lowercase() + " ";
        query.push_str("SET ");
        let mut fields: Vec<String> = Vec::new();
        for i in 0..row.0.len() {
           let field = [
               row.0[i].clone(),
               escape_single_quote(&row.1[i].clone())
            ].join("=");
           fields.push(field);
        }
        query.push_str(&fields.join(","));
        query.push_str(" WHERE sfid ='");
        query.push_str(id);
        query.push_str("'");
        // println!("{}", query);
        let conn = self.pool.get().unwrap();
        //add channel lock flag here
        let result = try!(conn.execute(query.as_str(),&[]).map_err(|err| err.to_string()));
        // println!("{:?}", result);
        Ok(result)
    }
}

fn escape_single_quote(elem: &String) -> String{
    if elem.starts_with("'") && elem.ends_with("'") {
        let tmp = elem.as_str();
        let tmp_slice = &tmp[1..elem.len() -1];
        let tmp_str = tmp_slice.to_string().replace("'","''");
        return String::from("'") + tmp_str.as_str() + "'";
     }
    return elem.to_string();
}