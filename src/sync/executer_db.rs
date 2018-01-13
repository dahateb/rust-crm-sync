
struct ExecuterInnerDb {
    db: Arc<Db>,
    salesforce: Arc<Salesforce>,
    synch_switch: Arc<Mutex<bool>>,
    config: &'static SyncConfig,
}

impl ExecuterInner for E{
    pub fn execute(&self, sender: Sender<String>) {

    }
}

