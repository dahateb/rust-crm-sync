use crate::config::SyncConfig;
use crate::db::Db;
use crate::salesforce::Salesforce;
use crate::sync::executer::{send_with_clear, ExecuterInner};
use crate::util::{Message, SyncMessage};
use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

pub struct ExecuterInnerDB {
    db: Arc<Db>,
    salesforce: Arc<Salesforce>,
    synch_switch: Arc<Mutex<bool>>,
    config: &'static SyncConfig,
}

impl ExecuterInnerDB {
    pub fn new(
        salesforce: Arc<Salesforce>,
        db: Arc<Db>,
        config: &'static SyncConfig,
    ) -> ExecuterInnerDB {
        ExecuterInnerDB {
            db: db,
            salesforce: salesforce,
            synch_switch: Arc::new(Mutex::new(false)),
            config: config,
        }
    }
}

impl ExecuterInner for ExecuterInnerDB {
    fn execute(&self, sender: Sender<Box<dyn Message>>, receiver: Receiver<Box<dyn Message>>) {
        let mut records_map: HashMap<String, Vec<i32>> = HashMap::new();
        for note in self.db.get_notifications().iter() {
            let object: Vec<&str> = note.split("::").collect();
            //println!("{}",note);
            send_with_clear(
                SyncMessage::new("triggered new db sync", object[0], 0),
                &sender,
                &receiver,
            );
            let name = object[0].to_owned();
            let _name = name.clone();
            let id = object[1].parse::<i32>().unwrap();
            if !records_map.contains_key(&name) {
                records_map.insert(name, vec![]);
            }
            records_map.get_mut(&_name).unwrap().push(id);
        }
        //println!("{:?}", records_map);
        for key in records_map.keys() {
            let result = self
                .db
                .get_object_data_by_id(&key, records_map.get::<str>(&key).unwrap());
            if result.is_ok() {
                let records = result.unwrap();
                for rec in &records {
                    println!("{}", rec.to_json());
                }
                let ids = self.salesforce.push_records(&key, &records);
                self.db.update_ids(&key, &ids.0);
                for (err_id, error) in &ids.1 {
                    self.db.set_error_state(&key, err_id, &error);
                }
                println!("{:?}", ids.0);
                send_with_clear(
                    SyncMessage::new("updated from db", key.as_str(), records.len()),
                    &sender,
                    &receiver,
                );
            } else {
                let _ = result.map_err(|err| println!("{}", err));
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

impl fmt::Display for ExecuterInnerDB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "db_executer")
    }
}
