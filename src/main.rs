extern crate dotenv;
extern crate libproto;
#[macro_use]
extern crate prometheus;
extern crate proof;
extern crate pubsub;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod block_metrics;
mod jsonrpc_metrics;
mod config;
mod proto;

use block_metrics::BlockMetrics;
use config::Config;
use pubsub::start_pubsub;
use std::env;
use std::sync::mpsc::channel;
use std::thread;

fn main() {
    let config = Config::load("./config.json");
    let mut threads = vec![];
    for url in config.amqp_urls {
        env::set_var("AMQP_URL", url.clone());
        let (send_to_main, receive_from_mq) = channel();
        let (_send_to_mq, receive_from_main) = channel();
        start_pubsub(
            "monitor_consensus",
            vec!["consensus.blk", "net.blk"],
            send_to_main,
            receive_from_main,
        );

        let t = thread::spawn(move || BlockMetrics::new(&url, receive_from_mq).process());
        threads.push(t);
    }

    threads.into_iter().for_each(|t| {
        let _ = t.join();
    });
}
