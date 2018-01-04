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

use std::env;
use std::thread;
use config::Config;
use block_metrics::BlockMetrics;

fn main() {
    let config = Config::load("./config.json");
    for url in config.amqp_urls {
        env::set_var("AMQP_URL", url.clone());
        thread::spawn(move || BlockMetrics::new(&url).process());
    }
}
