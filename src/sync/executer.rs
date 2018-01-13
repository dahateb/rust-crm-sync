use std::sync::{Arc};
use db::Db;
use salesforce::Salesforce;
use std::thread::{self, sleep};
use std::time::Duration;
use config::SyncConfig;
use std::sync::mpsc::{Receiver, Sender, channel};
use sync::executer_sf::ExecuterInnerSF;
pub struct Executer {
    inner: Arc<ExecuterInnerSF>,
    receiver: Option<Receiver<String>>,
}

impl Executer {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>, config: &'static SyncConfig) -> Executer {
        let inner = ExecuterInnerSF::new(salesforce,db,config);
        Executer {
            inner: Arc::new(inner),
            receiver: None,
        }
    }

    pub fn start_sync(&mut self) {
        let local_self = self.inner.clone();
        local_self.start();
        let (send, recv) = channel::<String>();
        self.receiver = Some(recv);
        thread::spawn(move || {
            for i in 1.. {
                local_self.execute(send.clone());
                {
                    let data = local_self.is_running();
                    if !data {
                        let _ = send.send(format!("Stopped Thread after {} loops", i));
                        return 0;
                    }
                    let _ = send.send(format!("hi number {} from the spawned thread! state: {}",
                                              i,
                                              data));
                }
                sleep(Duration::from_millis(local_self.get_timeout()));
            }
            return 0;
        });
    }

    pub fn stop_sync(&mut self) {
        self.inner.stop();
        self.receiver = None;
    }

    pub fn show_status(&self) {
        if self.receiver.is_none() {
            println!("No Sync Running");
            return;
        }
        println!("Sync is Running: ");
        let recv = &self.receiver.as_ref().unwrap();
        while let Ok(message) = recv.try_recv() {
            println!("{}", message);
        }
    }
}

pub trait ExecuterInner {

    fn execute(&self, Sender<String>);

    fn get_timeout(&self) -> u64;

    fn start(&self);

    fn is_running(&self) -> bool;

    fn stop(&self);
}