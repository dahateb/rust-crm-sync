use std::io;
use std::thread;
use std::thread::{sleep};
use std::time::Duration;
use config::Config;
use salesforce::Salesforce;
use std::string::String;
use std::str::FromStr;
use db::Db;
use std::thread::JoinHandle;
use std::sync::{Mutex, Arc};
use std::rc::Rc;

const STATE_START: u8 = 0;
const STATE_SETUP: u8 = 49;
const STATE_SYNC: u8 = 50;
const STATE_EXIT: u8 = 51;
const STATE_LIST_OBJECTS: u8 = 52;
const STATE_SELECTED_OBJECTS: u8 = 53;
const STATE_START_SYNC: u8 = 49;
const STATE_STOP_SYNC: u8 = 50;

pub mod executer;
pub mod mappings;

pub struct Sync {
    level: u8,
    command:  u8,
    salesforce: Arc<Salesforce>,
    input: String,
    db: Arc<Db>,
    threads: Vec<JoinHandle<u8>>,
    synch_switch: Arc<Mutex<bool>> 
}


impl Sync {

    pub fn new(config: Config) -> Sync {
        let mut sf = Salesforce::new(Rc::new(config.salesforce));
        //sf.client.print_login_data();
        Sync {
            level: STATE_START,
            command: STATE_START,
            salesforce: Arc::new(sf),
            input: String::new(),
            db: Arc::new(Db::new()),
            threads: Vec::with_capacity(1),
            synch_switch: Arc::new(Mutex::new(false))
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
                Sync {level: STATE_START, command: STATE_EXIT, ..} => {
                    println!("Exiting ...");
                    break;
                },
                Sync {level: STATE_LIST_OBJECTS, ..} => {
                    println!("Test: {}", self.command);
                    self.select_object();
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
                    self.input = String::from_str(input.trim()).unwrap_or_else(|err| String::new());
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
    }

    fn start_sync(&mut self) {
        let switch = self.synch_switch.clone();
        *switch.lock().unwrap() = true;
        let executer = executer::Executer::new(self.db.clone());
        let handle = thread::spawn(move || {
            
            for i in 1.. {
                executer.execute();
                {
                    let data = switch.lock().unwrap();
                    if !*data {
                        println!("Stopped Thread after {} loops", i);
                        return 0;   
                    }
                    println!("hi number {} from the spawned thread! state: {}", i, *data);
                }
                sleep(Duration::from_millis(1000));
            }
            return 0;
        });

        self.threads.push(handle);
    }

    fn stop_sync(& self) {
        let mut data = self.synch_switch.lock().unwrap();
        *data = false;
    }

    fn list(& self) {
        println!("List:");
        let sf_objects = self.salesforce.get_objects().unwrap();
        let mut counter = 0;
        for obj in &sf_objects {
            println!("{}. {} - {}",counter, obj.name, obj.createable);
            counter += 1 ;
        }
    }

    fn show_selected_objects(& self) {
        println!("Selected Objects");
        let objects : Vec<String> = self.db.get_selected_objects();
        for i in 0.. objects.len() {
            println!("{} {}", i+1, objects[i]);
        }
    }

    fn select_object(& self) {
        let sf_objects = self.salesforce.get_objects().unwrap();
        let index =  self.input.parse::<usize>().unwrap();
        let item = & sf_objects[index];
        println!("selected object: {}", item.name);
        let describe = self.salesforce.describe_object(&item.name).unwrap();
        let all_fields: String  = describe.fields.iter()
        .map(|field| field.name.clone())
        .fold(String::new(),|field_string, field_name| field_string + "\n" + field_name.as_str());
        println!("{}", all_fields);
        self.db.save_config_data(&describe);
        self.db.create_object_table(&item.name, describe.fields);
    }

}

