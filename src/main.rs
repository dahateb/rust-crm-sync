extern crate hyper;
extern crate hyper_tls;
extern crate futures;
extern crate tokio_core;
extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate time;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

#[macro_use]
extern crate serde_derive;

mod salesforce;
mod config;
mod sync;
mod db;
use config::Config;
use sync::Sync;

fn main() {
    let config = Config::new("config\\config.json").unwrap();
    let mut syncher = Sync::new(&config);
    syncher.run();
}


