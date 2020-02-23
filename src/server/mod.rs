pub mod executer;
pub mod http_routes;
pub mod response;

use crate::config::Config;
use crate::db::Db;
use crate::salesforce::Salesforce;
use crate::server::executer::Executer2;
use crate::server::http_routes::Router as Router2;
use crate::sync::executer::MESSAGE_CHANNEL_SIZE;
use crossbeam_channel::bounded;
use futures::{lazy, Future};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_01::prelude::*;
use tokio_01::timer::Interval;
use std::net::{SocketAddr, ToSocketAddrs};


pub struct ApiServer {
    config: &'static Config,
}

impl ApiServer {
    pub fn new(config: &'static Config) -> ApiServer {
        ApiServer{
            config
        }
    }
    pub fn run(&self) {
        let sf_arc = Arc::new(Salesforce::new(&self.config.salesforce));
        let db_arc = Arc::new(Db::new(&self.config.db));
        let (tx, rx) = bounded(MESSAGE_CHANNEL_SIZE);
        let executer = Executer2::new(sf_arc.clone(), db_arc.clone(), &self.config.sync);
        let router2 = Arc::new(Router2::new(
            sf_arc.clone(),
            db_arc.clone(),
            rx.clone(),
            executer.toggle_switch().clone(),
        ));
        let addr: SocketAddr = self.config.server.url.to_socket_addrs().unwrap().next().unwrap();
        let async_router = router2.worker();
        // setup worker
        let worker = Interval::new(Instant::now(), Duration::from_millis(1000))
            .for_each(move |instant| {
                async_router.handle_async(instant);
                Ok(())
            })
            .map_err(|e| eprintln!("worker errored; err={:?}", e));
        let skip_switch = executer.toggle_switch().clone();
        //sync worker
        let executer_worker =
            Interval::new(Instant::now(), Duration::from_millis(self.config.sync.timeout))
                .for_each(move |_instant| {
                    {
                        if !*skip_switch.lock().unwrap() {
                            return Ok(());
                        }
                    }
                    executer.execute(tx.clone(), rx.clone());
                    //  let note = format!("{:?}", instant);
                    //  send_with_clear(SyncMessage::new(note.as_str(), ""), &tx, &rx);
                    Ok(())
                })
                .map_err(|e| eprintln!("executer errored; err={:?}", e));
        tokio_compat::run(lazy(move || {
            println!("Serving at {}", addr);
            tokio_02::spawn(async move {
                warp::serve(router2.build_routes())
                    .run(addr)
                    .await;
            });
            tokio_01::spawn(lazy(|| worker));
            tokio_01::spawn(lazy(|| executer_worker));
            Ok(())
        }));
    }
}
