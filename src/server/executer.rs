use crate::config::SyncConfig;
use crate::db::Db;
use crate::salesforce::Salesforce;
use crate::sync::executer::{
    executer_db::ExecuterInnerDB, executer_sf::ExecuterInnerSF, ExecuterInner,
};
use crate::util::Message;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

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

    pub fn execute(&self, sender: Sender<Box<dyn Message>>, receiver: Receiver<Box<dyn Message>>) {
        //  println!("executer {}", *self.toggle_switch.lock().unwrap());
        for val in self.inners.iter() {
            let local_self = val.clone();
            let tx = sender.clone();
            let rx = receiver.clone();
            thread::spawn(move || {
                local_self.execute(tx, rx);
                // println!("{}", local_self);
            });
        }
    }

    pub fn toggle_switch(&self) -> Arc<Mutex<bool>> {
        self.toggle_switch.clone()
    }
}
