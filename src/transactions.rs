use amqp_client::start_sub;
use cratedb::Cluster;
use cratedb::sql::QueryRunner;
use libproto::{Message, MsgClass, Request, Response};
use serde_json;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use util::H256;

type NodeID = u32;
static INSERT_STMT: &str = "insert into transactions(hash, original, edges) values (?, ?, ?)";

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TxResponse {
    pub hash: H256,
    pub status: String,
}

pub enum Trace {
    /// Original(tx_hash, A): A is the original node which request the transaction tx_hash
    Original(H256, NodeID),
    /// ReceiveFrom(tx_hash, A, B): A receive transaction tx_hash from B
    ReceiveFrom(H256, NodeID, NodeID),
}

impl Trace {
    pub fn tx_hash(&self) -> H256 {
        match self {
            &Trace::Original(hash, _) => hash.clone(),
            &Trace::ReceiveFrom(hash, _, _) => hash.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
struct Edge {
    pub from_node: NodeID,
    pub to_node: NodeID,
}

#[derive(Debug, Serialize)]
struct TxTrace {
    pub hash: String,
    pub original: NodeID,
    pub edges: Vec<Edge>,
}

pub struct TransactionsTrack {
    /// Record original node which request the transaction
    original: HashMap<H256, NodeID>,
    /// (tx, to) -> from, `to` receive `tx` from `from`
    graph: HashMap<(H256, NodeID), NodeID>,
    /// The number of consensus
    node_num: usize,
    cratedb_client: Cluster,
}

impl TransactionsTrack {
    pub fn new(cratedb_url: String, node_num: usize) -> Self {
        TransactionsTrack {
            original: HashMap::new(),
            graph: HashMap::new(),
            node_num: node_num,
            cratedb_client: Cluster::from_string(cratedb_url).unwrap(),
        }
    }

    pub fn handle(&mut self, trace: Trace) {
        let tx_hash = trace.tx_hash().clone();
        match trace {
            Trace::Original(hash, node_id) => {
                assert!(self.original.get(&hash).is_none());
                self.original.insert(hash, node_id);
            }
            Trace::ReceiveFrom(hash, to, from) => {
                self.graph.entry((hash, to)).or_insert(from);
            }
        }

        if self.is_complete_for(&tx_hash) {
            let tx_trace = self.get_tx_trace(&tx_hash);
            info!("insert tx trace {:?} to cratedb", tx_trace);
            self.insert_to_db(tx_trace);
            self.remove_tx(&tx_hash);
        }
    }

    fn insert_to_db(&self, tx_trace: TxTrace) {
        let value = Box::new((tx_trace.hash, tx_trace.original, tx_trace.edges));
        let _ = self.cratedb_client.query(INSERT_STMT, Some(value)).unwrap();
    }

    fn is_complete_for(&self, hash: &H256) -> bool {
        if self.original.get(&hash).is_some() {
            let edge_size = self.graph.iter().filter(|elem| (elem.0).0 == *hash).count();
            edge_size + 1 == self.node_num
        } else {
            false
        }
    }

    fn get_tx_trace(&self, hash: &H256) -> TxTrace {
        let mut edges = vec![];
        self.graph
            .iter()
            .filter(|elem| (elem.0).0 == *hash)
            .for_each(|elem| {
                let edge = Edge {
                    from_node: *elem.1,
                    to_node: (elem.0).1,
                };
                edges.push(edge);
            });
        TxTrace {
            hash: format!("{:?}", hash),
            original: *self.original.get(&hash).unwrap(),
            edges: edges,
        }
    }

    fn remove_tx(&mut self, hash: &H256) {
        self.original.remove(&hash);
        self.graph.retain(|key, _value| key.0 != *hash);
    }
}

struct Node {
    amqp_url: String,
    id: NodeID,
    sender: Sender<Trace>,
}

impl Node {
    pub fn new(amqp_url: &str, node_id: NodeID, sender: Sender<Trace>) -> Self {
        Node {
            amqp_url: String::from(amqp_url),
            // TODO: amqp_url -> node_id
            id: node_id,
            sender: sender,
        }
    }

    pub fn handle(&self, message: (String, Vec<u8>)) {
        let (_topic, content) = message;
        let message = Message::try_from(&content).unwrap();
        let from = message.get_origin();
        let msg_class = message.get_content();
        match msg_class {
            MsgClass::REQUEST(request) => {
                let txs = get_txs_from_request(request);
                txs.into_iter().for_each(|hash| {
                    trace!("{} receive tx {:?} from {}", self.id, hash, from);
                    let _ = self.sender.send(Trace::ReceiveFrom(hash, self.id, from));
                });
            }
            // We don't know the failed tx hash, TODO: auth pub failed msg include tx hash
            MsgClass::RESPONSE(response) => {
                get_tx_from_response(response).map(|hash| {
                    trace!("the original of tx {:?} is {}", hash, self.id);
                    let _ = self.sender.send(Trace::Original(hash, self.id));
                });
            }
            _ => {}
        }
    }
}

fn get_txs_from_request(request: Request) -> Vec<H256> {
    let mut txs = vec![];
    if request.has_batch_req() {
        let requests = request.get_batch_req().get_new_tx_requests();
        for tx_req in requests {
            let un_tx_hash = tx_req.get_un_tx().crypt_hash();
            txs.push(un_tx_hash);
        }
    } else if request.has_un_tx() {
        let un_tx_hash = request.get_un_tx().crypt_hash();
        txs.push(un_tx_hash);
    }
    txs
}

fn get_tx_from_response(response: Response) -> Option<H256> {
    if response.has_tx_state() {
        let result = serde_json::from_str::<TxResponse>(response.get_tx_state()).unwrap();
        Some(result.hash)
    } else {
        None
    }
}

pub fn start(amqp_urls: &Vec<String>, cratedb_url: &str) {
    let (sender, receiver) = channel();
    let mut node_id: u32 = 0;
    let node_num = amqp_urls.len();
    for url in amqp_urls {
        let url = url.clone();
        let sender = sender.clone();
        thread::spawn(move || {
            let (send_to_main, receive_from_mq) = channel();
            let node = Node::new(&url, node_id, sender);
            start_sub(
                &url,
                "monitor_txs",
                vec!["net.tx", "auth.rpc"],
                send_to_main,
            );
            loop {
                let message = receive_from_mq.recv().unwrap();
                node.handle(message);
            }
        });
        node_id += 1;
    }

    let cratedb_url = String::from(cratedb_url);
    thread::spawn(move || {
        let mut tracker = TransactionsTrack::new(cratedb_url, node_num);
        loop {
            let trace = receiver.recv().unwrap();
            tracker.handle(trace);
        }
    });
}
