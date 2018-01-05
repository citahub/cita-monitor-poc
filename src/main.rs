extern crate libproto;
#[macro_use]
extern crate prometheus;
extern crate proof;
extern crate pubsub;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate dotenv;

mod block_metrics;
mod jsonrpc_metrics;
mod config;
mod proto;

use std::env;
use std::thread;
use std::sync::mpsc::channel;
use config::Config;
use block_metrics::BlockMetrics;
use pubsub::start_pubsub;

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
