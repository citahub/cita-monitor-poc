use consensus_metrics::ConsensusMetrics;
use metrics::Metrics;
use prometheus::proto::MetricFamily;
use util::Mutex;

/// Subscribe message from amqp_url, then dispatch message according to topic
pub struct Dispatcher {
    consensus_metrics: Mutex<ConsensusMetrics>,
    jsonrpc_metrics: Mutex<Metrics>,
    auth_metrics: Mutex<Metrics>,
    network_metrics: Mutex<Metrics>,
}

impl Dispatcher {
    pub fn new(
        consensus_metrics: ConsensusMetrics,
        jsonrpc_metrics: Metrics,
        auth_metrics: Metrics,
        network_metrics: Metrics,
    ) -> Self {
        Dispatcher {
            consensus_metrics: Mutex::new(consensus_metrics),
            jsonrpc_metrics: Mutex::new(jsonrpc_metrics),
            auth_metrics: Mutex::new(auth_metrics),
            network_metrics: Mutex::new(network_metrics),
        }
    }

    pub fn process(&self, msg: (String, Vec<u8>)) {
        let (key, content) = msg;
        trace!("receive message from topic: {}", key);
        match key.as_ref() {
            "jsonrpc.metrics" => self.jsonrpc_metrics.lock().process(content),
            "auth.metrics" => self.auth_metrics.lock().process(content),
            "network.metrics" => self.network_metrics.lock().process(content),
            _ => self.consensus_metrics.lock().process(content),
        }
    }

    pub fn gather(&self) -> Vec<MetricFamily> {
        let mut metric_families = vec![];
        {
            self.consensus_metrics
                .lock()
                .gather()
                .into_iter()
                .for_each(|metrics| metric_families.push(metrics));
        }
        {
            self.jsonrpc_metrics
                .lock()
                .gather()
                .into_iter()
                .for_each(|metrics| metric_families.push(metrics));
        }
        {
            self.auth_metrics
                .lock()
                .gather()
                .into_iter()
                .for_each(|metrics| metric_families.push(metrics));
        }
        {
            self.network_metrics
                .lock()
                .gather()
                .into_iter()
                .for_each(|metrics| metric_families.push(metrics));
        }
        metric_families
    }
}
