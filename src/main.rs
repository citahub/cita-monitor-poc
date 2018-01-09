#![allow(unused_must_use, unused_imports)]
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
extern crate pubsub;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate spin;
extern crate tokio_core;
extern crate protobuf;
#[macro_use]
extern crate util;

mod server;
mod consensus_metrics;
mod jsonrpc_metrics;
mod auth_metrics;
mod config;
mod dispatcher;

use consensus_metrics::ConsensusMetrics;
use clap::App;
use jsonrpc_metrics::JsonrpcMetrics;
use auth_metrics::AuthMetrics;
use dispatcher::Dispatcher;
use config::Config;
use hyper::server::Http;
use pubsub::start_pubsub;
use server::Server;
use std::env;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use util::panichandler::set_panic_handler;

fn new_dispatcher(url: String) -> Arc<Dispatcher> {
    let block_metrics = ConsensusMetrics::new(&url);
    let jsonrpc_metrics = JsonrpcMetrics::new(&url);
    let auth_metrics = AuthMetrics::new(&url);
    Arc::new(Dispatcher::new(
        block_metrics,
        jsonrpc_metrics,
        auth_metrics
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
            vec!["consensus.blk", "net.blk", "jsonrpc.metrics", "auth.metrics"],
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
        timeout: Duration::from_secs(10),
    };

    info!("http listening");
    let mut http = Http::new();
    http.pipeline(true);
    http.bind(&"127.0.0.1:8080".parse().unwrap(), server)
        .unwrap()
        .run();
}
