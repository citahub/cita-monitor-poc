use jsonrpc_metrics::JsonrpcMetrics;
use block_metrics::BlockMetrics;
use auth_metrics::AuthMetrics;
use prometheus::proto::MetricFamily;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;

/// Subscribe message from amqp_url, then dispatch message according to topic
pub struct Dispatcher {
    receiver: Mutex<Receiver<(String, Vec<u8>)>>,
    block_metrics: Mutex<BlockMetrics>,
    jsonrpc_metrics: Mutex<JsonrpcMetrics>,
    auth_metrics: Mutex<AuthMetrics>,
}

impl Dispatcher {
    pub fn new(receiver: Receiver<(String, Vec<u8>)>,
               block_metrics: BlockMetrics,
               jsonrpc_metrics: JsonrpcMetrics,
               auth_metrics: AuthMetrics) -> Self {
        Dispatcher {
            receiver: Mutex::new(receiver),
            block_metrics: Mutex::new(block_metrics),
            jsonrpc_metrics: Mutex::new(jsonrpc_metrics),
            auth_metrics: Mutex::new(auth_metrics),
        }
    }

    pub fn start(&self) {
        loop {
            let (key, content) = self.receiver.lock().unwrap().recv().unwrap();
            trace!("receive message from topic: {}", key);
            match key.as_ref() {
                "jsonrpc.metrics" => self.jsonrpc_metrics.lock().unwrap().process(content),
                "auth.metrics" => self.auth_metrics.lock().unwrap().process(content),
                _ => self.block_metrics.lock().unwrap().process(content),
            }
        }
    }

    pub fn gather(&self) -> Vec<MetricFamily> {
        let mut metric_families = vec![];
        self.block_metrics.lock().unwrap().gather().into_iter().for_each(|metrics| metric_families.push(metrics));
        self.jsonrpc_metrics.lock().unwrap().gather().into_iter().for_each(|metrics| metric_families.push(metrics));
        self.auth_metrics.lock().unwrap().gather().into_iter().for_each(|metrics| metric_families.push(metrics));
        metric_families
    }
}
