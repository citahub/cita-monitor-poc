//#![allow(unused_must_use, unused_imports)]
#![feature(mpsc_select)]
extern crate clap;
extern crate dotenv;
extern crate futures;
extern crate hyper;
extern crate libproto;
#[macro_use]
extern crate log;
extern crate logger;
#[macro_use]
extern crate prometheus;
extern crate proof;
extern crate protobuf;
extern crate pubsub;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate spin;
extern crate tokio_core;
#[macro_use]
extern crate util;

mod server;
mod consensus_metrics;
mod metrics;
mod config;
mod dispatcher;

use clap::App;
use config::Config;
use dispatcher::Dispatcher;
use hyper::server::Http;
use pubsub::start_pubsub;
use server::Server;
use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;
use std::sync::mpsc::{channel, Select};
use std::thread;
use std::time::Duration;
use util::set_panic_handler;

fn main() {
    micro_service_init!("cita-monitor", "CITA:Monitor");
    // load config
    let matches = App::new("monitor")
        .version("0.1")
        .author("Cryptape")
        .about("CITA Monitor by Rust")
        .args_from_usage("-c, --config=[FILE] 'Sets a custom config file'")
        .get_matches();

    let mut config_path = "./monitor.json";
    if let Some(c) = matches.value_of("monitor") {
        info!("Value for config: {}", c);
        config_path = c;
    }

    let config = Config::load(config_path);
    info!("CITA:monitor config \n {:?}", config);
    let mut dispatchers = vec![];
    let mut vec_rx = vec![];

    let mut i: usize = 0;
    for url in config.amqp_urls {
        env::set_var("AMQP_URL", url.clone());
        let (send_to_main, receive_from_mq) = channel();
        let (_send_to_mq, receive_from_main) = channel();
        start_pubsub(
            "monitor_consensus",
            vec![
                "consensus.blk",
                "net.blk",
                "jsonrpc.metrics",
                "auth.metrics",
                "network.metrics",
            ],
            send_to_main,
            receive_from_main,
        );
        let dispatcher = Arc::new(Dispatcher::new(url));
        let dup_dispatcher = dispatcher.clone();
        vec_rx.insert(i, (0, receive_from_mq, dup_dispatcher));
        dispatchers.push(dispatcher);
    }

    thread::spawn(move || {
        let select = Select::new();
        if let Some(item) = vec_rx.get_mut(i) {
            let mut receive_from_mq1 = select.handle(&item.1);
            unsafe {
                receive_from_mq1.add();
            }
            item.0 = receive_from_mq1.id();
        }
        i += 1;

        let mut btree_map = BTreeMap::new();
        for (id, receive_from_mq, dup_dispatcher) in vec_rx {
            btree_map.insert(id, (receive_from_mq, dup_dispatcher));
        }

        loop {
            let id = select.wait();
            let &(ref receive_from_mq, ref dup_dispatcher) = btree_map.get(&id).unwrap();
            dup_dispatcher.process(receive_from_mq.recv().unwrap())
        }
    });

    let server = Server {
        dispatchers: dispatchers,
        timeout: Duration::from_secs(config.duration),
    };

    info!("http listening: 0.0.0.0:8000");
    let mut http = Http::new();
    http.pipeline(true);
    let _ = http.bind(&"0.0.0.0:8000".parse().unwrap(), server)
        .unwrap()
        .run();
}
