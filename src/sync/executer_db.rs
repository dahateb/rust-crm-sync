use std::sync::{Mutex, Arc};
use db::Db;
use salesforce::Salesforce;
use config::SyncConfig;
use std::sync::mpsc::{Sender};
use sync::executer::ExecuterInner;


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
    }
    
    fn start(&self) {
        *self.synch_switch.lock().unwrap() = true;
    }

    fn is_running(&self) -> bool {
        *self.synch_switch.lock().unwrap()
    }

    fn stop(&self) {
        *self.synch_switch.lock().unwrap() = false;
    }

    fn get_timeout(&self) -> u64 {
        self.config.timeout
    }
}

