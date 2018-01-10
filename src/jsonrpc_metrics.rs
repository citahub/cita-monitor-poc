use prometheus::proto::{LabelPair, MetricFamily, MetricFamilyVec};
use protobuf::core::parse_from_bytes;
use util::snappy;

pub struct JsonrpcMetrics {
    amqp_url: String,
    metrics: Option<MetricFamilyVec>,
}

impl JsonrpcMetrics {
    pub fn new(amqp_url: &str) -> Self {
        JsonrpcMetrics {
            amqp_url: String::from(amqp_url),
            metrics: None,
        }
    }

    pub fn process(&mut self, data: Vec<u8>) {
        let decompressed_data = snappy::cita_decompress(data);
        let metrics = parse_from_bytes::<MetricFamilyVec>(&decompressed_data).unwrap();
        self.metrics = Some(metrics);
    }

    pub fn gather(&mut self) -> Vec<MetricFamily> {
        let mut metric_families: Vec<MetricFamily> = self.metrics
            .take()
            .map_or(vec![], |t| t.get_metrics().to_vec());
        for metrics in metric_families.as_mut_slice() {
            for metric in metrics.mut_metric().as_mut_slice() {
                let mut label = LabelPair::new();
                label.set_name(String::from("amqp_url"));
                label.set_value(self.amqp_url.clone());
                metric.mut_label().push(label);
            }
        }
        metric_families
    }
}
