use crossbeam_channel::{Receiver, Sender};
use std::time::Instant;
use sync::setup::Setup;
use util::{Message, TriggerMessage};

pub struct AsyncRouter {
    setup: Setup,
    trigger_receiver: Receiver<(String, usize)>,
    message_sender: Sender<Box<Message>>,
}

impl AsyncRouter {
    pub fn new(
        setup: Setup,
        trigger_receiver: Receiver<(String, usize)>,
        message_sender: Sender<Box<Message>>,
    ) -> AsyncRouter {
        AsyncRouter {
            setup: setup,
            trigger_receiver: trigger_receiver,
            message_sender: message_sender,
        }
    }

    pub fn handle_async(&self, _instant: std::time::Instant) {
        let recv = self.trigger_receiver.clone();
        while let Ok(message) = recv.try_recv() {
            println!("{}:{}", message.0, message.1);
            match message.0.as_ref() {
                "/setup/new" => {
                    let sender = self.message_sender.clone();
                    let setup = self.setup.clone();
                    //asynchronous to allow for multiple objects
                    std::thread::spawn(move || {
                        let start_time = Instant::now();
                        let notify = |notification: &str, count: u64| {
                            let message =
                                TriggerMessage::new(notification, count, start_time.elapsed());
                            let _ = sender.send(Box::new(message));
                        };
                        let _res = setup.setup_sf_object(message.1, true, notify);
                    });
                }
                "/setup/delete" => {
                    let _res = self
                        .setup
                        .delete_db_object(message.1)
                        .map_err(|err| println!("{}", err));
                }
                _ => println!(""),
            }
        }
    }
}
