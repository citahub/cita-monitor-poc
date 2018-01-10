use prometheus::proto::{LabelPair, MetricFamily, MetricFamilyVec};
use protobuf::core::parse_from_bytes;
use util::snappy;

pub struct AuthMetrics {
    amqp_url: String,
    metrics: Option<MetricFamilyVec>,
}

impl AuthMetrics {
    pub fn new(amqp_url: &str) -> Self {
        AuthMetrics {
            amqp_url: String::from(amqp_url),
            metrics: None,
        }
    }

    pub fn process(&mut self, data: Vec<u8>) {
        let decompressed_data = snappy::cita_decompress(data);
        let metrics = parse_from_bytes::<MetricFamilyVec>(&decompressed_data).unwrap();
        self.metrics = Some(metrics);
    }

    pub fn gather(&self) -> Vec<MetricFamily> {
        let mut metric_families = vec![];
        {
            self.block_metrics
                .lock()
                .unwrap()
                .gather()
                .into_iter()
                .for_each(|metrics| metric_families.push(metrics))
        };
        {
            self.jsonrpc_metrics
                .lock()
                .unwrap()
                .gather()
                .into_iter()
                .for_each(|metrics| metric_families.push(metrics))
        };
        {
            self.auth_metrics
                .lock()
                .unwrap()
                .gather()
                .into_iter()
                .for_each(|metrics| metric_families.push(metrics))
        };
        {
            self.network_metrics
                .lock()
                .unwrap()
                .gather()
                .into_iter()
                .for_each(|metrics| metric_families.push(metrics))
        };
        metric_families
    }
}
