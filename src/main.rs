//#![allow(unused_must_use, unused_imports)]
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
use metrics::Metrics;
use pubsub::start_pubsub;
use server::Server;
use std::env;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use util::set_panic_handler;

fn new_dispatcher(url: String) -> Arc<Dispatcher> {
    let block_metrics = Metrics::new(&url);
    let jsonrpc_metrics = Metrics::new(&url);
    let auth_metrics = Metrics::new(&url);
    let network_metrics = Metrics::new(&url);
    Arc::new(Dispatcher::new(
        block_metrics,
        jsonrpc_metrics,
        auth_metrics,
        network_metrics,
    ))
}

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
        let dispatcher = new_dispatcher(url);
        let dup_dispatcher = dispatcher.clone();
        dispatchers.push(dispatcher);

        thread::spawn(move || loop {
            dup_dispatcher.process(receive_from_mq.recv().unwrap())
        });
    }

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
