use std::sync::{Arc};
use db::Db;
use salesforce::Salesforce;
use std::thread::{self, sleep};
use std::time::Duration;
use config::SyncConfig;
use std::sync::mpsc::{Receiver, Sender, channel};
use sync::executer_sf::ExecuterInnerSF;
use sync::executer_db::ExecuterInnerDB;

pub struct Executer  {
    inners: Vec<Arc<EIW>>,
    receiver: Option<Receiver<String>>,
}

impl Executer {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>, config: &'static SyncConfig) -> Executer {
        let inner_sf=  EIW::SF(ExecuterInnerSF::new(salesforce.clone(),db.clone(),config));
        let inner_db = EIW::DB(ExecuterInnerDB::new(salesforce,db,config));
        Executer {
            inners: vec!(Arc::new(inner_sf), Arc::new(inner_db)),
            receiver: None,
        }
    }

    pub fn start_sync(&mut self) {
        let (send, recv) = channel::<String>();
        self.receiver = Some(recv);
        for val in self.inners.iter() {
            {     
                val.convert().start();
            }
            let val = val.clone();
            let send = send.clone();
            thread::spawn(move ||{
                let local_self = val.convert();
                for i in 1.. {  
                    local_self.execute(send.clone());
                    {
                        let data = local_self.is_running();
                        if !data {
                            let _ = send.send(format!("Stopped Thread after {} loops", i));
                            return 0;
                        }
                        let _ = send.send(
                            format!("hi number {} from the spawned thread! state: {}",
                                              i,
                                              data)
                        );
                    }    
                    
                    sleep(Duration::from_millis(local_self.get_timeout()));
                }
                return 0;
            });    
        }
    }

    pub fn stop_sync(&mut self) {
        for val in self.inners.iter() {
            val.convert().stop();
        }
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

enum EIW {
    SF(ExecuterInnerSF),
    DB(ExecuterInnerDB)
}

impl EIW {
    fn convert(&self) -> &ExecuterInner  {
        match self {
            &EIW::SF(ref ei) => return ei,
            &EIW::DB(ref ei) => return ei,
        }
    }
}