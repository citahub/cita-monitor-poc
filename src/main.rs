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
mod config;

use std::env;
use config::Config;
use pubsub::start_pubsub;
use std::sync::mpsc::channel;
use block_metrics::BlockMetrics;

fn main() {
    // let config = Config::load("./config.json");
    // for url in config.amqp_urls {
    //     env::set_var("AMQP_URL", url);
    // }

    let (send_to_main, receive_from_mq) = channel();
    let (_send_to_mq, receive_from_main) = channel();
    start_pubsub(
        "monitor_consensus",
        vec!["consensus.blk", "net.blk"],
        send_to_main,
        receive_from_main,
    );

    BlockMetrics::new(receive_from_mq).process();
}
