use jsonrpc_metrics::JsonrpcMetrics;
use block_metrics::BlockMetrics;
use auth_metrics::AuthMetrics;
use prometheus::proto::MetricFamily;
use std::sync::mpsc::Receiver;

/// Subscribe message from amqp_url, then dispatch message according to topic
pub struct Dispatcher {
    receiver: Receiver<(String, Vec<u8>)>,
    block_metrics: BlockMetrics,
    jsonrpc_metrics: JsonrpcMetrics,
    auth_metrics: AuthMetrics,
}

impl Dispatcher {
    pub fn new(receiver: Receiver<(String, Vec<u8>)>,
               block_metrics: BlockMetrics,
               jsonrpc_metrics: JsonrpcMetrics,
               auth_metrics: AuthMetrics) -> Self {
        Dispatcher {
            receiver: receiver,
            block_metrics: block_metrics,
            jsonrpc_metrics: jsonrpc_metrics,
            auth_metrics: auth_metrics,
        }
    }

    pub fn start(&mut self) {
        loop {
            let (key, content) = self.receiver.recv().unwrap();
            match key.as_ref() {
                "jsonrpc.metrics" => self.jsonrpc_metrics.process(content),
                "auth.metrics" => self.auth_metrics.process(content),
                _ => self.block_metrics.process(content),
            }
        }
    }

    pub fn gather(&mut self) -> Vec<MetricFamily> {
        let mut metric_families = vec![];
        self.block_metrics.gather().into_iter().for_each(|metrics| metric_families.push(metrics));
        self.jsonrpc_metrics.gather().into_iter().for_each(|metrics| metric_families.push(metrics));
        self.auth_metrics.gather().into_iter().for_each(|metrics| metric_families.push(metrics));
        metric_families
    }
}
