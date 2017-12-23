use std::sync::{Mutex, Arc};
use db::Db;
use salesforce::Salesforce;
use std::thread::{self, sleep};
use std::time::Duration;
use config::SyncConfig;
use db::objects::ObjectConfig;
use std::sync::mpsc::{Receiver, Sender, channel};


struct ExecuterInner {
    db: Arc<Db>,
    salesforce: Arc<Salesforce>,
    synch_switch: Arc<Mutex<bool>>,
    config: &'static SyncConfig
}

impl ExecuterInner {
    pub fn execute(& self, sender: Sender<String>) {
        //println!("executing.... ");
        let objects : Vec<ObjectConfig> = self.db.get_selected_objects(1).unwrap();
        for i in 0.. objects.len() {
            let fields = objects[i].get_field_names();
            let _ = sender.send(format!("{} {} {:?}", i+1, objects[i].name, fields.len()));
            let row_result = self.salesforce.get_last_updated_records(&objects[i],1).unwrap();
            let _ = sender.send(format!("num rows to synch: {}", row_result.rows.len()));
            let result = self.db.upsert_object_rows( &row_result)
                                .map_err(|err| println!("{}", err));
            let _ = sender.send(format!("{}", result.unwrap()));
            self.db.update_last_sync_time(objects[i].id);
        }        
    }
}

pub struct Executer {
    inner: Arc<ExecuterInner>,
    receiver: Option<Receiver<String>>
}

impl Executer {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>, config: &'static SyncConfig) -> Executer {
        let inner = ExecuterInner {
            db: db,
            salesforce: salesforce,
            synch_switch: Arc::new(Mutex::new(false)),
            config: config
        };
        Executer {
            inner: Arc::new(inner),
            receiver: None
        }
    }

    pub fn start_sync(&mut self) {
        let local_self = self.inner.clone();
        *local_self.synch_switch.lock().unwrap() = true;
        let (send, recv) = channel::<String>();
        self.receiver = Some(recv);
        thread::spawn(move || {            
            for i in 1.. {
                local_self.execute(send.clone());
                {
                    let data = local_self.synch_switch.lock().unwrap();
                    if !*data {
                        let _ = send.send(format!("Stopped Thread after {} loops", i));
                        return 0;   
                    }
                    let _ = send.send(format!("hi number {} from the spawned thread! state: {}", i, *data));
                }
                sleep(Duration::from_millis(local_self.config.timeout));
            }
            return 0;
        });
    }

    pub fn stop_sync(&mut self) {
        let mut data = self.inner.synch_switch.lock().unwrap();
        *data = false;
        self.receiver = None;
    }

    pub fn show_status(& self) {
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