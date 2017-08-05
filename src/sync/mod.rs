use std::io;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use config::Config;
use salesforce::Salesforce;
use std::string::String;
use std::str::FromStr;
use db::Db;

const STATE_START: u8 = 0;
const STATE_SETUP: u8 = 49;
const STATE_SYNC: u8 = 50;
const STATE_EXIT: u8 = 51;
const STATE_LIST_OBJECTS: u8 = 52;
const STATE_SELECTED_OBJECTS: u8 = 53;

pub struct Sync<'a> {
    prev_state: u8,
    state:  u8,
    salesforce: Salesforce<'a>,
    input: String,
    db: Db
}


impl<'a> Sync<'a> {

    pub fn new(config: &Config) -> Sync {
        let mut sf = Salesforce::new(&config);
        sf.login();
        sf.print_login_data();
        Sync {
            prev_state: STATE_START,
            state: STATE_START,
            salesforce: sf,
            input: String::new(),
            db: Db::new()
        }
    }

    pub fn run(&mut self) {
        let mut input = String::new();

        loop {
            match *self {
                Sync {state: STATE_START, ..} => self.start(),
                Sync {prev_state: STATE_START, state: STATE_SETUP, ..} => self.setup(),
                Sync {prev_state: STATE_START, state: STATE_SYNC, ..} => self.sync(),
                Sync {prev_state: STATE_SETUP, state: STATE_LIST_OBJECTS, ..} => self.list(),
                Sync {prev_state: STATE_SETUP, state: STATE_SELECTED_OBJECTS, ..} => self.show_selected_objects(),
                Sync {state: STATE_EXIT, ..} => {
                    println!("Exiting ...");
                    break;
                },
                Sync {prev_state: STATE_LIST_OBJECTS, ..} => {
                    println!("Test: {}", self.state);
                    self.select_object();
                },
                _ => {
                    self.start();
                    println!("Error: {}", self.prev_state);
                    self.state = STATE_START;
                }
            }
            
            match io::stdin().read_line(&mut input) {
                Ok(n) => {
                    self.prev_state = self.state;
                    self.state = input.as_bytes()[0];
                    self.input = String::from_str(input.trim()).unwrap();
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
    
    fn sync(&mut self) {
        println!("Synch:");
        thread::spawn(|| {
            for i in 1..100 {
                println!("hi number {} from the spawned thread!", i);
                sleep(Duration::from_millis(500));
            }
        });

        self.salesforce.get_last_updated_records("Account",30)
    }

    fn list(& mut self) {
        println!("List:");
        let sf_objects = self.salesforce.get_objects().unwrap();
        let mut counter = 0;
        for obj in &sf_objects {
            println!("{}. {} - {}",counter, obj.name, obj.createable);
            counter += 1 ;
        }
    }

    fn show_selected_objects(& mut self) {
        println!("Selected Objects");
        let objects : Vec<String> = self.db.get_selected_objects();
        for i in 0.. objects.len() {
            println!("{} {}", i+1, objects[i]);
        }
    }

    fn select_object(& mut self) {
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
    }
}

