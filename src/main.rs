//#![allow(unused_must_use, unused_imports)]
#![feature(mpsc_select)]
extern crate amqp;
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
mod amqp_adapter;

use amqp_adapter::start_sub;
use clap::App;
use config::Config;
use dispatcher::Dispatcher;
use hyper::server::Http;
use server::Server;
use std::collections::HashMap;
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

    for url in &config.amqp_urls {
        env::set_var("AMQP_URL", url.clone());
        let (send_to_main, receive_from_mq) = channel();
        start_sub(
            "monitor_consensus",
            vec![
                "consensus.blk",
                "net.blk",
                "jsonrpc.metrics",
                "auth.metrics",
                "network.metrics",
            ],
            send_to_main,
        );
        let dispatcher = Arc::new(Dispatcher::new(url.clone()));
        vec_rx.push((receive_from_mq, dispatcher.clone()));
        dispatchers.push(dispatcher);
    }

    thread::spawn(move || {
        let select = Select::new();
        let mut handlers = HashMap::new();
        for item in &mut vec_rx {
            let handler = select.handle(&item.0);
            handlers.insert(handler.id(), (handler, item.1.clone()));
        }

        // Note: we have to put handler.add() here, it doesn't work if we
        // put handler.add() in previous for loop. I don't know why!!!
        for (_, value) in handlers.iter_mut() {
            let (ref mut handler, _) = *value;
            unsafe {
                handler.add();
            }
        }

        loop {
            let id = select.wait();
            trace!("handle {} ready to process", id);
            let (ref mut handler, ref dispatcher) = *handlers.get_mut(&id).unwrap();
            dispatcher.process(handler.recv().unwrap());
        }
    });

    let server = Server {
        dispatchers: dispatchers,
        timeout: Duration::from_secs(config.duration),
    };

    info!("http listening: {:?}", config.ip_port);
    let mut http = Http::new();
    http.pipeline(true);
    let _ = http.bind(&config.ip_port.parse().unwrap(), server)
        .unwrap()
        .run();
}
