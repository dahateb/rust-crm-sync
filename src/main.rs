extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate time;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate fallible_iterator;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

mod salesforce;
mod config;
mod sync;
mod db;
use config::Config;
use sync::Sync;

lazy_static! {
    static ref CONF:Config = Config::new("config\\config.json").unwrap();
}

fn main() {
    let mut syncher = Sync::new(&CONF);
    syncher.run();
}
