use std::collections::HashMap;
use util::H256;
use cratedb::{Cluster, NoParams};
use cratedb::sql::QueryRunner;
use amqp_client::start_sub;
use std::sync::mpsc::{channel, Select};
use std::thread;
use libproto::{Message, MsgClass, Response, Request};

type NodeID = u32;

enum Trace {
    /// Original(tx_hash, A): A is the original node which request the transaction tx_hash
    Original(H256, NodeID),
    /// SendTo(tx_hash, A, B :: C): A send transaction tx_hash to B and C
    SendTo(H256, NodeID, Vec<NodeID>),
    /// ReceiveFrom(tx_hash, A, B): A receive transaction tx_hash from B
    ReceiveFrom(H256, NodeID, NodeID),
}

pub struct TransactionsTrack {
    /// Record original node which request the transaction
    original: HashMap<H256, NodeID>,
    cratedb_client: Cluster,
}

impl TransactionsTrack {
    pub fn new(cratedb_url: &str) -> Self {
        TransactionsTrack {
            original: HashMap::new(),
            cratedb_client: Cluster::from_string(cratedb_url).unwrap(),
        }
    }
}

struct Node {
    amqp_url: String,
    id: NodeID,
}

impl Node {
    pub fn new(amqp_url: &str) -> Self {
        Node {
            amqp_url: String::from(amqp_url),
            // TODO: amqp_url -> node_id
            id: 0,
        }
    }

    /// Topics are "auth.tx", "net.tx", "auth.rpc"
    pub fn handle(message: (String, Vec<u8>)) {
        let (topic, content) = message;
        let msg_class = Message::try_from(content).unwrap();
        match msg_class {
            MsgClass::REQUEST(request) => {

            },
            // We don't know the failed tx hash, TODO: auth pub failed msg include tx hash
            MsgClass::RESPONSE(response) => {

            }
        }
    }

    fn process_response(response: Response) {
    }
}

pub fn start(amqp_urls: &Vec<String>, cratedb_url: &str) {
    let mut receivers = vec![];
    for url in amqp_urls {
        let (send_to_main, receive_from_mq) = channel();
        start_sub(url, "monitor_txs", vec!["auth.tx", "net.tx", "auth.rpc"], send_to_main);
        receivers.push(receive_from_mq);
    }

    thread::spawn(move || {
        let select = Select::new();
        let mut handlers = HashMap::new();
        for receiver in &receivers {
            let handler = select.handle(receiver);
            handlers.insert(handler.id(), handler);
        }

        for (_, handler) in handlers.iter_mut() {
            unsafe {
                handler.add();
            }
        }

        loop {
            let id = select.wait();
            trace!("handle {} ready to process", id);
            let ref mut handler = handlers.get_mut(&id).unwrap();
            let msg = handler.recv().unwrap();
            info!("receive transaction track message");
        }
    });
}
