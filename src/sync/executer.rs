use std::sync::Arc;
use db::Db;
use salesforce::Salesforce;

pub struct Executer<'a> {
    db: Arc<Db>,
    salesforce: Arc<Salesforce<'a>>,
    messages: Vec<String>
}

impl<'a> Executer<'a> {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>) -> Executer {
        Executer {
            db: db,
            salesforce: salesforce,
            messages: vec![]
        }
    }

    pub fn execute(& self) {
        self.salesforce.get_last_updated_records("Account",30);
    }
}