use std::sync::{Mutex, Arc};
use db::Db;
use salesforce::Salesforce;
use config::SyncConfig;
use std::sync::mpsc::{Sender};
use std::collections::HashMap;
use sync::executer::ExecuterInner;
use r2d2_postgres::{TlsMode, PostgresConnectionManager};
use r2d2::{Config, Pool, PooledConnection};

pub struct ExecuterInnerDB {
    db: Arc<Db>,
    salesforce: Arc<Salesforce>,
    synch_switch: Arc<Mutex<bool>>,
    config: &'static SyncConfig,
}

impl ExecuterInnerDB {
    pub fn new(salesforce: Arc<Salesforce>,db: Arc<Db>,config: &'static SyncConfig)
        -> ExecuterInnerDB {
        ExecuterInnerDB {
            db: db,
            salesforce: salesforce,
            synch_switch: Arc::new(Mutex::new(false)),
            config: config,
        }
    }
}

impl ExecuterInner for ExecuterInnerDB{
    fn execute(&self, sender: Sender<String>) {
        let _ = sender.send("Executer DB".to_owned());
        let mut records_map: HashMap<String, Vec<i32>> = HashMap::new();
        for note in self.db.get_notifications().iter() {
            let object: Vec<&str> = note.split("::").collect();
            //println!("{}",note);
            let _ = sender.send(note.clone());
            let name = object[0].to_owned();
            let _name = name.clone();
            let id = object[1].parse::<i32>().unwrap();
            if !records_map.contains_key(&name){
                records_map.insert(name,vec!() );
            }
            records_map.get_mut(&_name).unwrap().push(id);
        }
        //println!("{:?}", records_map);
        for key in records_map.keys() {
            let records = self.db.get_object_data_by_id(&key, records_map.get::<str>(&key).unwrap());
            for rec in records{
                println!("{}", rec.to_json());
            }
        }
    }
    
    fn start(&self) {
        self.db.toggle_listen(true);
        *self.synch_switch.lock().unwrap() = true;
    }

    fn is_running(&self) -> bool {
        *self.synch_switch.lock().unwrap()
    }

    fn stop(&self) {
        self.db.toggle_listen(true);
        *self.synch_switch.lock().unwrap() = false;
    }

    fn get_timeout(&self) -> u64 {
        self.config.timeout
    }
}

