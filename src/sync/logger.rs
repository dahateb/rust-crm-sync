use crossbeam_channel::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;
use util::Message;

pub struct Logger {
    receiver: Option<Receiver<Box<Message>>>,
    switch: Arc<Mutex<bool>>,
}

impl Logger {
    pub fn new() -> Logger {
        Logger {
            receiver: None,
            switch: Arc::new(Mutex::new(false)),
        }
    }

    pub fn add_receiver(&mut self, receiver: Option<Receiver<Box<Message>>>) {
        //let mut recv = self.receiver.borrow_mut();
        //if recv.is_none() {
        //    *recv = Some(receiver);
        //}
        self.receiver = receiver;
    }

    pub fn start(&self) {
        let switch = self.switch.clone();
        let receiver = self.receiver.clone();
        {
            *switch.lock().unwrap() = true;
        }

        thread::spawn(move || loop {
            let check = *switch.lock().unwrap();
            if !check {
                break;
            }
            let recv = receiver.as_ref().unwrap();
            while let Ok(message) = recv.try_recv() {
                println!("{}", message.to_string());
            }
            sleep(Duration::from_millis(1000));
        });
    }

    pub fn stop(&self) {
        *self.switch.lock().unwrap() = false;
    }
}
