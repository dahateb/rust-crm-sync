
struct ExecuterInner {
    db: Arc<Db>,
    salesforce: Arc<Salesforce>,
    synch_switch: Arc<Mutex<bool>>,
    config: &'static SyncConfig,
}

impl ExecuterInner {
    pub fn execute(&self, sender: Sender<String>) {

    }
}

pub struct Executer {
    inner: Arc<ExecuterInner>,
    receiver: Option<Receiver<String>>,
}

impl Executer {
    pub fn new(db: Arc<Db>, salesforce: Arc<Salesforce>, config: &'static SyncConfig) -> Executer {
        let inner = ExecuterInner {
            db: db,
            salesforce: salesforce,
            synch_switch: Arc::new(Mutex::new(false)),
            config: config,
        };
        Executer {
            inner: Arc::new(inner),
            receiver: None,
        }
    }
}