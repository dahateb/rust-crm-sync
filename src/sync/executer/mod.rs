pub mod executer_sf;
pub mod executer_db;

use std::sync::{Mutex, Arc};
use db::Db;
use salesforce::Salesforce;
use std::thread::{self, sleep};
use std::time::Duration;
use std::fmt::{Display, Formatter, Result};
use config::SyncConfig;
use std::sync::mpsc::{Receiver, Sender, channel};
use sync::executer::executer_sf::ExecuterInnerSF;
use sync::executer::executer_db::ExecuterInnerDB;

pub struct Executer  {
    inners: Vec<Arc<EIW>>,
    pub receiver: Option<Arc<Mutex<Receiver<String>>>>,
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
        self.receiver = Some(Arc::new(Mutex::new(recv)));
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
                            format!("tick: {}, type: {}",
                                              i,
                                              val)
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

impl Display for EIW {    
    fn fmt(&self, f: &mut Formatter) -> Result {
       match *self {
           EIW::DB(_) => write!(f, "db_executer"),
           EIW::SF(_) => write!(f, "sf_executer")        
       }
    }
}