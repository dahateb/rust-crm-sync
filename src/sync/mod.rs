use std::io;
use config::Config;
use db::objects::ObjectConfig;
use salesforce::Salesforce;
use std::string::String;
use std::str::FromStr;
use db::Db;
use std::sync::Arc;
use std::cell::RefCell;


const STATE_START: u8 = 0;
const STATE_SETUP: u8 = 49;
const STATE_SYNC: u8 = 50;
const STATE_EXIT: u8 = 51;
const STATE_LIST_OBJECTS: u8 = 52;
const STATE_SELECTED_OBJECTS: u8 = 53;
const STATE_START_SYNC: u8 = 49;
const STATE_STOP_SYNC: u8 = 50;
const STATE_SYNC_STATUS: u8 = 51;

pub mod executer;
pub mod mappings;
pub mod cache;

use sync::executer::Executer;
use sync::cache::SyncObjectCache;

pub struct Sync {
    level: u8,
    command:  u8,
    salesforce: Arc<Salesforce>,
    input: String,
    db: Arc<Db>,
    executer: Executer,
    config: &'static Config,
    cache: RefCell<SyncObjectCache>
}


impl Sync {

    pub fn new(config: &'static Config) -> Sync {
        let sf = Salesforce::new(&config.salesforce);
        let db_arc = Arc::new(Db::new(&config.db));
        let sf_arc = Arc::new(sf);
        Sync {
            level: STATE_START,
            command: STATE_START,
            salesforce: sf_arc.clone(),
            input: String::new(),
            db: db_arc.clone(),
            executer: Executer::new(db_arc, sf_arc, &config.sync),
            config: config,
            cache: Default::default()
        }
    }
    
    pub fn run(&mut self) {
        let mut input = String::new();

        loop {
            match *self {
                Sync {level: STATE_START, command: STATE_START, ..} => self.start(),
                Sync {level: STATE_START, command: STATE_SETUP, ..} => self.setup(),
                Sync {level: STATE_START, command: STATE_SYNC, ..} => self.sync(),
                Sync {level: STATE_SETUP, command: STATE_LIST_OBJECTS, ..} => self.list(),
                Sync {level: STATE_SETUP, command: STATE_SELECTED_OBJECTS, ..} => self.show_selected_objects(),
                Sync {level: STATE_SYNC, command: STATE_START_SYNC, ..} => self.start_sync(),
                Sync {level: STATE_SYNC, command: STATE_STOP_SYNC, ..} => self.stop_sync(),
                Sync {level: STATE_SYNC, command: STATE_SYNC_STATUS, ..} => self.show_sync_status(),
                Sync {level: STATE_START, command: STATE_EXIT, ..} => {
                    println!("Exiting ...");
                    break;
                },
                Sync {level: STATE_LIST_OBJECTS, ..} => {
                    println!("Selected Object: {}", self.command);
                    self.select_object();
                },
                Sync {level: STATE_SELECTED_OBJECTS, ..} => {
                    self.delete_object();
                    println!("Deleted Object: {}" , self.command);
                },
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
                        println!("{}",err);
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
    
    fn sync(& self) {
        println!("Synch:");
        println!("1. Start Synch");
        println!("2. Stop Synch");
        println!("3. Show Status");
    }

    fn start_sync(&mut  self) {
        println!("Starting ... ");
        self.executer.start_sync();
    }

    fn stop_sync(&mut self) {
        println!("Stopping ... ");
        self.executer.stop_sync();
    }

    fn show_sync_status(& self) {
        println!("Status: ");
        self.executer.show_status();
    }

    fn list(& self) {
        println!("List:");
        let sf_objects = self.salesforce.get_objects().unwrap();
        for i in 0..sf_objects.len() {
            println!("{}.\t{}\t\t\t\t{}",i, sf_objects[i].name, sf_objects[i].createable);
        }
        println!("Select Object:");
        self.cache.borrow_mut().sf_objects = Some(sf_objects);
    }

    fn show_selected_objects(& self) {
        println!("Selected Objects");
        let objects : Vec<ObjectConfig> = self.db.get_selected_objects(-1).unwrap();
        for i in 0.. objects.len() {
            println!("{}.\t{}\t\t\t{}", i+1, objects[i].name, objects[i].count);
        }
        self.cache.borrow_mut().db_objects = Some(objects);
    }

    fn select_object(& self) {
        let cache = &self.cache.borrow();
        let sf_objects = cache.sf_objects.as_ref().unwrap();
        let index =  self.input.parse::<isize>().unwrap_or_else(|_err| -1);
        if index == -1 {
            println!("Input invalid");
            return;
        }
        let item = & sf_objects[index as usize];
        println!("selected object: {}", item.name);
        let describe = self.salesforce.describe_object(&item.name).unwrap();
        self.db.save_config_data(&describe);
        self.db.create_object_table(&item.name, &describe.fields);
        let wrapper = self.salesforce.get_records_from_describe(&describe, &item.name).unwrap();
        let mut row_count = 0;
        row_count += self.db.populate(&wrapper).unwrap();
        println!("Synched {} rows", row_count);
        let mut next_wrapper_opt = self.salesforce.get_next_records(&describe, &wrapper);
        while let Some(next_wrapper) = next_wrapper_opt {
            row_count += self.db.populate(&next_wrapper).unwrap();
            println!("Synched {} rows", row_count);
            if !next_wrapper.done {
                println!("Next Path: {}", next_wrapper.next_url);
            } else {
                println!("Done: {} rows", row_count);
            }
            next_wrapper_opt = self.salesforce.get_next_records(&describe, &next_wrapper);
        }
    }

    fn delete_object(&self) {
        let cache = &self.cache.borrow();
        let db_objects = cache.db_objects.as_ref().unwrap();
        let index =  self.input.parse::<isize>().unwrap_or_else(|_err| -1);
        if index == -1 || index as usize > db_objects.len() {
            println!("Input invalid");
            return;
        }
        let obj =&db_objects[(index-1) as usize];
        println!("Delete Object: {}", obj.name);
        self.db.destroy(obj.id, &obj.name);
    }
}

