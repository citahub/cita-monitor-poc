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

unsafe impl Sync for Dispatcher {}

impl Dispatcher {
    pub fn new(url: String) -> Dispatcher {
        Dispatcher {
            consensus_metrics: Mutex::new(ConsensusMetrics::new(&url)),
            jsonrpc_metrics: Mutex::new(Metrics::new(&url)),
            auth_metrics: Mutex::new(Metrics::new(&url)),
            network_metrics: Mutex::new(Metrics::new(&url)),
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
