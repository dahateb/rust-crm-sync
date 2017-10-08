use std::sync::{Mutex, Arc};
use db::Db;
use salesforce::Salesforce;
use std::thread::{self, sleep};
use std::time::Duration;
use config::SyncConfig;


struct ExecuterInner {
    db: Arc<Db>,
    salesforce: Arc<Salesforce>,
    messages: Vec<String>,
    synch_switch: Arc<Mutex<bool>>,
    config: &'static SyncConfig
}

impl ExecuterInner {
    pub fn execute(& self) {
        println!("executing.... ");
        let objects : Vec<String> = self.db.get_selected_objects();
        for i in 0.. objects.len() {
            println!("{} {}", i+1, objects[i]);
            self.salesforce.get_last_updated_records(objects[i].as_str(),30);
        }        
    }
}

pub struct Executer {
    inner: Arc<ExecuterInner>
}

impl Executer {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>, config: &'static SyncConfig) -> Executer {
        let inner = ExecuterInner {
            db: db,
            salesforce: salesforce,
            messages: vec![],
            synch_switch: Arc::new(Mutex::new(false)),
            config: config
        };
        Executer {
            inner: Arc::new(inner)
        }
    }

    pub fn start_sync(&mut self) {
        let local_self = self.inner.clone();
        *local_self.synch_switch.lock().unwrap() = true;
        thread::spawn(move || {            
            for i in 1.. {
                local_self.execute();
                {
                    let data = local_self.synch_switch.lock().unwrap();
                    if !*data {
                        println!("Stopped Thread after {} loops", i);
                        return 0;   
                    }
                    println!("hi number {} from the spawned thread! state: {}", i, *data);
                }
                sleep(Duration::from_millis(local_self.config.timeout));
            }
            return 0;
        });
    }

    pub fn stop_sync(& self) {
        let mut data = self.inner.synch_switch.lock().unwrap();
        *data = false;
    }
}