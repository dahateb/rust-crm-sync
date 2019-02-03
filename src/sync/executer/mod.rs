pub mod executer_db;
pub mod executer_sf;

use config::SyncConfig;
use crossbeam_channel::{bounded, Receiver, Sender};
use db::Db;
use salesforce::Salesforce;
use std::fmt;
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;
use sync::executer::executer_db::ExecuterInnerDB;
use sync::executer::executer_sf::ExecuterInnerSF;
use util::{Message, SyncMessage};

pub const MESSAGE_CHANNEL_SIZE: usize = 1000;

pub struct Executer {
    inners: Vec<Arc<dyn ExecuterInner + Send + Sync>>,
    pub receiver: Option<Receiver<Box<Message>>>,
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
        let (send, recv) = bounded::<Box<Message>>(1000);
        self.receiver = Some(recv.clone());
        for val in self.inners.iter() {
            {
                val.start();
            }
            let local_self = val.clone();
            let tx = send.clone();
            let rx = recv.clone();
            thread::spawn(move || {
                for i in 1.. {
                    local_self.execute(tx.clone(), rx.clone());
                    {
                        let data = local_self.is_running();
                        if !data {
                            let note = format!("Stopped Thread after {} loops", i);
                            send_with_clear(&note, &tx, &rx);
                            return 0;
                        }
                        let note = format!("tick: {}, type: {}", i, local_self);
                        send_with_clear(&note, &tx, &rx);
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
    fn execute(&self, Sender<Box<Message>>, Receiver<Box<Message>>);
    fn get_timeout(&self) -> u64;
    fn start(&self);
    fn is_running(&self) -> bool;
    fn stop(&self);
}

pub fn send_with_clear(
    msg: &String,
    sender: &Sender<Box<Message>>,
    receiver: &Receiver<Box<Message>>,
) {
    match sender.try_send(Box::new(SyncMessage::new(msg.as_str()))) {
        Ok(_) => {}
        Err(err) => {
            if err.is_full() {
                for _ in receiver.iter().take(MESSAGE_CHANNEL_SIZE / 2) {}
            }
        }
    }
}
