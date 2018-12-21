extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate time;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate fallible_iterator;
extern crate hyper;
extern crate pretty_env_logger;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

mod salesforce;
mod config;
mod sync;
mod db;
mod server;
use config::Config;
use sync::Sync;
use server::Server;
use std::env;

lazy_static! {
    static ref CONF:Config = Config::new("config\\config.1.json").unwrap();
}

fn main() {

    let args: Vec<String> = env::args().collect();
    //println!("{}", args[1]);
    if args.len() > 1 && args[1] == "-i" {
        let mut syncher = Sync::new(&CONF);
        syncher.run();
    }else{
        let server = Server::new(&CONF);
        server.run();
    }

}
