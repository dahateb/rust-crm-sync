use config::SyncConfig;
use db::Db;
use salesforce::Salesforce;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use sync::executer::{executer_db::ExecuterInnerDB, executer_sf::ExecuterInnerSF, ExecuterInner};

pub struct Executer2 {
    toggle_switch: Arc<Mutex<bool>>,
    inners: Vec<Arc<dyn ExecuterInner + Send + Sync>>,
}

impl Executer2 {
    pub fn new(sf_arc: Arc<Salesforce>, db_arc: Arc<Db>, config: &'static SyncConfig) -> Executer2 {
        let inner_sf = ExecuterInnerSF::new(sf_arc.clone(), db_arc.clone(), config);
        let inner_db = ExecuterInnerDB::new(sf_arc, db_arc, config);
        Executer2 {
            toggle_switch: Arc::new(Mutex::new(false)),
            inners: vec![Arc::new(inner_sf), Arc::new(inner_db)],
        }
    }

    pub fn execute(&self, _instant: Instant) {
        println!("executer {}", *self.toggle_switch.lock().unwrap());
        for val in self.inners.iter() {
            let local_self = val.clone();
            thread::spawn(move || {
                println!("{}", local_self);
            });
        }
    }

    pub fn toggle_switch(&self) -> Arc<Mutex<bool>> {
        self.toggle_switch.clone()
    }
}