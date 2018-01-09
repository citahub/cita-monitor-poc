use libproto::{parse_msg, MsgClass};
use libproto::blockchain::Block;
use prometheus::{gather, Counter, Gauge, Opts};
use prometheus::proto::MetricFamily;
use proof::TendermintProof;
use std::usize::MAX;

/// Subscribe message from mq, push metric to prometheus gateway.
pub struct ConsensusMetrics {
    /// Block generated interval gauge
    block_interval: Gauge,
    /// Block number
    block_number: Gauge,
    /// Last block generator
    last_block_generator: Gauge,
    /// Transaction counter
    transaction_counter: Counter,
    last_block_timestamp: u64,
    current_block_timestamp: u64,
}

fn opts_with_amqp_url(name: String, help: String, amqp_url: &str) -> Opts {
    let opts = Opts::new(name, help);
    opts.const_label("amqp_url", amqp_url)
}

impl ConsensusMetrics {
    pub fn new(amqp_url: &str) -> Self {
        let block_interval = register_gauge!(opts_with_amqp_url(
            String::from("block_interval"),
            String::from("Block generate interval"),
            amqp_url
        )).unwrap();

        let block_number = register_gauge!(opts_with_amqp_url(
            String::from("block_number"),
            String::from("The current height of chain."),
            amqp_url
        )).unwrap();

        let last_block_generator = register_gauge!(opts_with_amqp_url(
            String::from("last_block_generator"),
            String::from("Generator of last block, represent with node id."),
            amqp_url
        )).unwrap();

        let transaction_counter = register_counter!(opts_with_amqp_url(
            String::from("transaction_counter"),
            String::from("Transaction counter"),
            amqp_url
        )).unwrap();

        ConsensusMetrics {
            block_interval: block_interval,
            block_number: block_number,
            last_block_generator: last_block_generator,
            transaction_counter: transaction_counter,
            last_block_timestamp: 0,
            current_block_timestamp: 0,
        }
    }

    fn set_last_block_generator(&self, proof: TendermintProof) {
        let height = proof.height;
        if height == MAX {
            return;
        }
        let round = proof.round;
        let node_id = (height + round) % 4;
        self.last_block_generator.set(node_id as f64);
    }

    fn set_block_interval(&mut self, block: &Block) {
        self.last_block_timestamp = self.current_block_timestamp;
        self.current_block_timestamp = block.get_header().get_timestamp();
        let diff = if self.last_block_timestamp == 0 {
            3.0
        } else {
            ((self.current_block_timestamp - self.last_block_timestamp) as f64) / 1000.0
        };
        self.block_interval.set(diff);
    }

    fn set_transaction_counter(&self, block: &Block) {
        let tx_number = block.get_body().get_transactions().len() as f64;
        let _ = self.transaction_counter.inc_by(tx_number);
    }

    fn record_block(&mut self, block: &Block) {
        let block_height = block.get_header().get_height();
        let current_height = self.block_number.get() as u64;
        if block_height == current_height + 1 || current_height == 0 {
            let proof = block.get_header().get_proof();
            // TODO: we need to support other proof type in the future
            self.block_number.set(block_height as f64);
            self.set_last_block_generator(TendermintProof::from(proof.clone()));
            self.set_block_interval(&block);
            self.set_transaction_counter(&block);
            // TODO: use pull instead of push
            // let metric_family = gather();
            // let _ = push_metrics("cita-consensus", HashMap::new(), &url, metric_family);
        }
    }

    pub fn process(&mut self, data: Vec<u8>) {
        let (_cmd_id, _origin, msg_type) = parse_msg(&data);
        match msg_type {
            MsgClass::BLOCKWITHPROOF(block_with_proof) => {
                self.record_block(block_with_proof.get_blk());
            }
            MsgClass::SYNCRESPONSE(sync_response) => {
                let blocks = sync_response.get_blocks();
                for block in blocks {
                    self.record_block(&block);
                }
            }
            _ => {}
        }
    }

    pub fn gather(&self) -> Vec<MetricFamily> {
        gather()
    }
}
