pub mod executer;
pub mod logger;
pub mod setup;

use crate::config::Config;
use crate::db::Db;
use crate::salesforce::Salesforce;
use crate::sync::executer::Executer;
use crate::sync::logger::Logger;
use crate::sync::setup::Setup;
use std::cell::RefCell;
use std::io::{self, Write};
use std::str::FromStr;
use std::string::String;
use std::sync::Arc;

const STATE_START: u8 = 0;
const STATE_SETUP: u8 = 49;
const STATE_SYNC: u8 = 50;
const STATE_EXIT: u8 = 51;
const STATE_LIST_OBJECTS: u8 = 52;
const STATE_SELECTED_OBJECTS: u8 = 53;
const STATE_START_SYNC: u8 = 49;
const STATE_STOP_SYNC: u8 = 50;
const STATE_SYNC_STATUS: u8 = 51;

pub struct Sync {
    level: u8,
    command: u8,
    input: String,
    executer: Executer,
    setup: Setup,
    logger: RefCell<Logger>,
    _config: &'static Config,
}

impl Sync {
    pub fn new(config: &'static Config) -> Sync {
        let sf = Salesforce::new(&config.salesforce);
        let db_arc = Arc::new(Db::new(&config.db));
        let sf_arc = Arc::new(sf);
        Sync {
            level: STATE_START,
            command: STATE_START,
            input: String::new(),
            executer: Executer::new(db_arc.clone(), sf_arc.clone(), &config.sync),
            setup: Setup::new(db_arc, sf_arc),
            logger: RefCell::new(Logger::new()),
            _config: config,
        }
    }

    pub fn run(&mut self) {
        let mut input = String::new();

        loop {
            match *self {
                Sync {
                    level: STATE_START,
                    command: STATE_START,
                    ..
                } => self.start(),
                Sync {
                    level: STATE_START,
                    command: STATE_SETUP,
                    ..
                } => self.setup(),
                Sync {
                    level: STATE_START,
                    command: STATE_SYNC,
                    ..
                } => self.sync(),
                Sync {
                    level: STATE_SETUP,
                    command: STATE_LIST_OBJECTS,
                    ..
                } => self.list(),
                Sync {
                    level: STATE_SETUP,
                    command: STATE_SELECTED_OBJECTS,
                    ..
                } => self.show_selected_objects(),
                Sync {
                    level: STATE_SYNC,
                    command: STATE_START_SYNC,
                    ..
                } => self.start_sync(),
                Sync {
                    level: STATE_SYNC,
                    command: STATE_STOP_SYNC,
                    ..
                } => self.stop_sync(),
                Sync {
                    level: STATE_SYNC,
                    command: STATE_SYNC_STATUS,
                    ..
                } => self.start_show_log(),
                Sync {
                    level: STATE_SYNC_STATUS,
                    ..
                } => {
                    self.stop_show_log();
                    self.command = STATE_START;
                }
                Sync {
                    level: STATE_START,
                    command: STATE_EXIT,
                    ..
                } => {
                    println!("Exiting ...");
                    break;
                }
                Sync {
                    level: STATE_LIST_OBJECTS,
                    ..
                } => {
                    self.select_object();
                }
                Sync {
                    level: STATE_SELECTED_OBJECTS,
                    ..
                } => {
                    self.delete_object();
                    println!("Deleted Object: {}", self.command);
                }
                _ => {
                    self.start();
                    println!("Error: {}", self.level);
                    self.command = STATE_START;
                }
            }

            match io::stdin().read_line(&mut input) {
                Ok(n) => {
                    self.level = self.command;
                    self.command = input.as_bytes()[0];
                    self.input = String::from_str(input.trim()).unwrap_or_else(|err| {
                        println!("{}", err);
                        String::new()
                    });
                    input.clear();
                    drop(n);
                }
                Err(error) => println!("error: {}", error),
            }
        }
    }

    fn start(&self) {
        println!("Syncher:");
        println!("1. Setup");
        println!("2. Sync");
        println!("3. Exit");
    }

    fn setup(&self) {
        println!("Setup:");
        println!("4. List available Objects");
        println!("5. Show synchronized Objects");
    }

    fn sync(&self) {
        println!("Synch:");
        println!("1. Start Synch");
        println!("2. Stop Synch");
        println!("3. Show Status");
    }

    fn start_sync(&mut self) {
        println!("Starting ... ");
        self.executer.start_sync();
    }

    fn stop_sync(&mut self) {
        println!("Stopping ... ");
        self.executer.stop_sync();
    }

    fn start_show_log(&self) {
        println!("Status: ");
        let opt = self.executer.receiver.as_ref();
        match opt {
            Some(recv) => {
                let mut logger = self.logger.borrow_mut();
                logger.add_receiver(Some(recv.clone()));
                logger.start();
            }
            None => {
                println!("Sync not running");
            }
        }
    }

    fn stop_show_log(&mut self) {
        let logger = self.logger.borrow();
        logger.stop();
    }

    fn list(&self) {
        println!("List:");
        let print_func = |obj: (u32, &String, &String, bool, bool, bool)| {
            println!("{}.\t{}\t\t\t\t{}", obj.0, obj.1, obj.4);
            String::new()
        };
        let _ = self
            .setup
            .list_salesforce_objects(print_func)
            .map_err(|err| println!("{}", err));
        println!("Select Object:");
    }

    fn show_selected_objects(&self) {
        println!("Selected Objects");
        let print_func = |obj: (u32, &String, u32, usize)| {
            println!("{}.\t{}\t\t\t{}", obj.0, obj.1, obj.2);
            String::new()
        };
        let _ = self
            .setup
            .list_db_objects(print_func)
            .map_err(|err| println!("{}", err));
    }

    fn select_object(&self) {
        let index = self.input.parse::<isize>().unwrap_or_else(|_err| -1);
        if index == -1 {
            println!("Input invalid");
            return;
        }
        println!("Selected Object: {}", self.command);
        let notify = |_: &str, _: u64| {
            print!(".");
            io::stdout().flush().unwrap();
        };
        let (name, row_count) = self
            .setup
            .setup_sf_object(index as usize, true, notify)
            .unwrap();
        println!("Selected object: {}", name);
        println!("Synched {} rows", row_count);
    }

    fn delete_object(&self) {
        let index = self.input.parse::<isize>().unwrap_or_else(|_err| -1);
        if index == -1 {
            println!("Input invalid");
            return;
        }
        let name = self.setup.delete_db_object(index as usize).unwrap();
        println!("Delete Object: {}", name);
    }
}
