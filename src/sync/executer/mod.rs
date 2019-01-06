pub mod executer_db;
pub mod executer_sf;

use config::SyncConfig;
use db::Db;
use salesforce::Salesforce;
use std::fmt;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;
use sync::executer::executer_db::ExecuterInnerDB;
use sync::executer::executer_sf::ExecuterInnerSF;

pub struct Executer {
    inners: Vec<Arc<dyn ExecuterInner + Send + Sync>>,
    pub receiver: Option<Arc<Mutex<Receiver<String>>>>,
}

impl Executer {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>, config: &'static SyncConfig) -> Executer {
        let inner_sf = ExecuterInnerSF::new(salesforce.clone(), db.clone(), config);
        let inner_db = ExecuterInnerDB::new(salesforce, db, config);
        Executer {
            inners: vec![Arc::new(inner_sf), Arc::new(inner_db)],
            receiver: None,
        }
    }

    pub fn start_sync(&mut self) {
        let (send, recv) = channel::<String>();
        self.receiver = Some(Arc::new(Mutex::new(recv)));
        for val in self.inners.iter() {
            {
                val.start();
            }
            let val = val.clone();
            let send = send.clone();
            thread::spawn(move || {
                let local_self = val;
                for i in 1.. {
                    local_self.execute(send.clone());
                    {
                        let data = local_self.is_running();
                        if !data {
                            let _ = send.send(format!("Stopped Thread after {} loops", i));
                            return 0;
                        }
                        let _ = send.send(format!("tick: {}, type: {}", i, local_self));
                    }

                    sleep(Duration::from_millis(local_self.get_timeout()));
                }
                return 0;
            });
        }
    }

    pub fn stop_sync(&mut self) {
        for val in self.inners.iter() {
            val.stop();
        }
        self.receiver = None;
    }
}

pub trait ExecuterInner: fmt::Display {
    fn execute(&self, Sender<String>);
    fn get_timeout(&self) -> u64;
    fn start(&self);
    fn is_running(&self) -> bool;
    fn stop(&self);
}
