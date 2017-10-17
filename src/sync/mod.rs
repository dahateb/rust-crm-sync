use std::io;
use config::Config;
use db::objects::ObjectConfig;
use salesforce::Salesforce;
use std::string::String;
use std::str::FromStr;
use db::Db;
use std::sync::Arc;


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

use sync::executer::Executer;

pub struct Sync {
    level: u8,
    command:  u8,
    salesforce: Arc<Salesforce>,
    input: String,
    db: Arc<Db>,
    executer: Executer,
    config: &'static Config 
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
            config: config
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
    }

    fn start_sync(&mut  self) {
        self.executer.start_sync();
    }

    fn stop_sync(& self) {
        self.executer.stop_sync();
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
        let objects : Vec<ObjectConfig> = self.db.get_selected_objects(-1);
        for i in 0.. objects.len() {
            println!("{} {}", i+1, objects[i].name);
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
        self.db.create_object_table(&item.name, &describe.fields);
        let value = self.salesforce.get_records_from_describe(describe, &item.name);
        println!("{:?}", value);
    }

}

