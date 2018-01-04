use std::sync::mpsc::Receiver;
use std::collections::HashMap;
use std::usize::MAX;
use prometheus::{gather, push_metrics, Gauge, Counter};
use libproto::{parse_msg, MsgClass};
use libproto::blockchain::Block;
use proof::TendermintProof;

/// Process message from mq, push metric to prometheus gateway.
pub(crate) struct BlockMetrics {
    /// Receive message from
    receiver: Receiver<(String, Vec<u8>)>,
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

impl BlockMetrics {
    pub fn new(receiver: Receiver<(String, Vec<u8>)>) -> Self {
        let block_interval = register_gauge!(
            "block_interval",
            "Block generated interval in seconds."
        ).unwrap();

        let block_number = register_gauge!(
            "block_number",
            "The current height of chain."
        ).unwrap();

        let last_block_generator = register_gauge!(
            "last_block_generator",
            "Generator of last block, represent with node id."
        ).unwrap();

        let transaction_counter = register_counter!(
            "transaction_counter",
            "Transaction counter"
        ).unwrap();

        BlockMetrics {
            receiver: receiver,
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

    fn record_block(&mut self, block: &Block, url: &str) {
        let block_height = block.get_header().get_height();
        let current_height = self.block_number.get() as u64;
        if block_height == current_height + 1 || current_height == 0 {
            let proof = block.get_header().get_proof();
            // TODO: we need to support other proof type in the future
            self.block_number.set(block_height as f64);
            self.set_last_block_generator(TendermintProof::from(proof.clone()));
            self.set_block_interval(&block);
            self.set_transaction_counter(&block);
            let metric_family = gather();
            let _ = push_metrics("cita-consensus", HashMap::new(), &url, metric_family);
        }
    }

    pub fn process(&mut self) {
        let address = String::from("127.0.0.1:9091");
        loop {
            let (_key, content) = self.receiver.recv().unwrap();
            let (_cmd_id, _origin, msg_type) = parse_msg(&content);
            match msg_type {
                MsgClass::BLOCKWITHPROOF(block_with_proof) => {
                    println!("receive msg");
                    self.record_block(block_with_proof.get_blk(), &address);
                }
                MsgClass::SYNCRESPONSE(sync_response) => {
                    let blocks = sync_response.get_blocks();
                    for block in blocks {
                        self.record_block(&block, &address);
                    }
                }
                _ => {}
            }
        }
    }
}
