use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Executer2 {
    toggle_switch: Arc<Mutex<bool>>,
}

impl Executer2 {
    pub fn new() -> Executer2 {
        Executer2 {
            toggle_switch: Arc::new(Mutex::new(false)),
        }
    }

    pub fn execute(&self, instant: Instant) {
        println!("executer {}", *self.toggle_switch.lock().unwrap());
    }

    pub fn toggle_switch(&self) -> Arc<Mutex<bool>> {
        self.toggle_switch.clone()
    }
}
