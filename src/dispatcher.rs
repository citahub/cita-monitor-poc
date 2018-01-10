use auth_metrics::AuthMetrics;
use consensus_metrics::ConsensusMetrics;
use jsonrpc_metrics::JsonrpcMetrics;
use network_metrics::NetworkMetrics;
use prometheus::proto::MetricFamily;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;

/// Subscribe message from amqp_url, then dispatch message according to topic
pub struct Dispatcher {
    block_metrics: Mutex<ConsensusMetrics>,
    jsonrpc_metrics: Mutex<JsonrpcMetrics>,
    auth_metrics: Mutex<AuthMetrics>,
    network_metrics: Mutex<NetworkMetrics>,
}

impl Dispatcher {
    pub fn new(
        block_metrics: ConsensusMetrics,
        jsonrpc_metrics: JsonrpcMetrics,
        auth_metrics: AuthMetrics,
        network_metrics: NetworkMetrics,
    ) -> Self {
        Dispatcher {
            block_metrics: Mutex::new(block_metrics),
            jsonrpc_metrics: Mutex::new(jsonrpc_metrics),
            auth_metrics: Mutex::new(auth_metrics),
            network_metrics: Mutex::new(network_metrics),
        }
    }

    pub fn process(&self, msg: (String, Vec<u8>)) {
        let (key, content) = msg;
        trace!("receive message from topic: {}", key);
        match key.as_ref() {
            "jsonrpc.metrics" => self.jsonrpc_metrics.lock().unwrap().process(content),
            "auth.metrics" => self.auth_metrics.lock().unwrap().process(content),
            "network.metrics" => self.network_metrics.lock().unwrap().process(content),
            _ => self.block_metrics.lock().unwrap().process(content),
        }
    }

    pub fn gather(&self) -> Vec<MetricFamily> {
        let mut metric_families = vec![];
        self.block_metrics
            .lock()
            .unwrap()
            .gather()
            .into_iter()
            .for_each(|metrics| metric_families.push(metrics));
        self.jsonrpc_metrics
            .lock()
            .unwrap()
            .gather()
            .into_iter()
            .for_each(|metrics| metric_families.push(metrics));
        self.auth_metrics
            .lock()
            .unwrap()
            .gather()
            .into_iter()
            .for_each(|metrics| metric_families.push(metrics));
        self.network_metrics
            .lock()
            .unwrap()
            .gather()
            .into_iter()
            .for_each(|metrics| metric_families.push(metrics));
        metric_families
    }
}
