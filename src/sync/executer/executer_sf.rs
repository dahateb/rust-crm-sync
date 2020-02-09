use crate::config::SyncConfig;
use crate::db::objects::ObjectConfig;
use crate::db::Db;
use crate::salesforce::Salesforce;
use crate::sync::executer::{send_with_clear, ExecuterInner};
use crate::util::{Message, SyncMessage};
use crossbeam_channel::{Receiver, Sender};
use std::fmt;
use std::sync::{Arc, Mutex};

pub struct ExecuterInnerSF {
    db: Arc<Db>,
    salesforce: Arc<Salesforce>,
    synch_switch: Arc<Mutex<bool>>,
    pub config: &'static SyncConfig,
}

impl ExecuterInnerSF {
    pub fn new(
        salesforce: Arc<Salesforce>,
        db: Arc<Db>,
        config: &'static SyncConfig,
    ) -> ExecuterInnerSF {
        ExecuterInnerSF {
            db: db,
            salesforce: salesforce,
            synch_switch: Arc::new(Mutex::new(false)),
            config: config,
        }
    }
}

impl ExecuterInner for ExecuterInnerSF {
    fn execute(&self, sender: Sender<Box<dyn Message>>, receiver: Receiver<Box<dyn Message>>) {
        //println!("executing.... ");
        let objects: Vec<ObjectConfig> = self.db.get_selected_objects(1).unwrap();
        for i in 0..objects.len() {
            let _fields = objects[i].get_field_names();
            let row_result = self
                .salesforce
                .get_last_updated_records(&objects[i], 1)
                .unwrap_or_else(|err| {
                    panic!("sf_executer: {}", err);
                });
            let note = format!("num rows to synch: {}", row_result.rows.len());
            send_with_clear(
                SyncMessage::new(note.as_str(), &objects[i].name, row_result.rows.len()),
                &sender,
                &receiver,
            );
            let result = self
                .db
                .upsert_object_rows(&row_result)
                .map_err(|err| println!("{}", err));
            let mut row_count = result.unwrap();
            let mut next_wrapper_opt = self.salesforce.get_next_records(&objects[i], &row_result);
            while let Some(next_wrapper) = next_wrapper_opt {
                row_count += self.db.populate(&next_wrapper).unwrap();
                let note = format!("Synched {} rows", row_count);
                send_with_clear(SyncMessage::new(note.as_str(), "", 0), &sender, &receiver);
                if !next_wrapper.done {
                    let note = format!("Next Path: {}", next_wrapper.next_url);
                    send_with_clear(SyncMessage::new(note.as_str(), "", 0), &sender, &receiver);
                } else {
                    let note = format!("Done: {} rows", row_count);
                    send_with_clear(
                        SyncMessage::new(note.as_str(), "", row_count as usize),
                        &sender,
                        &receiver,
                    );
                }
                next_wrapper_opt = self.salesforce.get_next_records(&objects[i], &next_wrapper);
            }

            let note = format!("{}", result.unwrap());
            send_with_clear(SyncMessage::new(note.as_str(), "", 0), &sender, &receiver);
            self.db.update_last_sync_time(objects[i].id);
        }
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

impl fmt::Display for ExecuterInnerSF {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "sf_executer")
    }
}
