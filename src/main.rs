
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

mod config;
mod db;
mod salesforce;
mod server;
mod sync;
mod util;
use crate::config::Config;
use crate::server::ApiServer as Server;
use crate::sync::Sync;
use std::env;

lazy_static! {
    static ref CONF: Config = Config::new("config.1.json").unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    //println!("{}", args[1]);
    if args.len() > 1 && args[1] == "-i" {
        let mut syncher = Sync::new(&CONF);
        syncher.run();
    } else {
        Server::new(&CONF).run();
    }
}
