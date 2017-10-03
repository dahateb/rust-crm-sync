use std::sync::Arc;
use db::Db;
use salesforce::Salesforce;

pub struct Executer {
    db: Arc<Db>,
    salesforce: Arc<Salesforce>,
    messages: Vec<String>
}

impl Executer {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>) -> Executer {
        Executer {
            db: db,
            salesforce: salesforce,
            messages: vec![]
        }
    }

    pub fn execute(& self) {
        println!("executing.... ");
        let objects : Vec<String> = self.db.get_selected_objects();
        for i in 0.. objects.len() {
            println!("{} {}", i+1, objects[i]);
            self.salesforce.get_last_updated_records(objects[i].as_str(),30);
        }        
    }
}