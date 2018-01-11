use prometheus::proto::{LabelPair, MetricFamily, MetricFamilyVec};
use protobuf::core::parse_from_bytes;
use util::snappy;

pub struct Metrics {
    amqp_url: String,
    metrics: MetricFamilyVec,
}

impl Metrics {
    pub fn new(amqp_url: &str) -> Self {
        Metrics {
            amqp_url: String::from(amqp_url),
            metrics: MetricFamilyVec::new(),
        }
    }

    pub fn process(&mut self, data: Vec<u8>) {
        let decompressed_data = snappy::cita_decompress(data);
        self.metrics = parse_from_bytes::<MetricFamilyVec>(&decompressed_data).unwrap();
    }

    pub fn gather(&mut self) -> Vec<MetricFamily> {
        let mut metric_families: Vec<MetricFamily> = self.metrics.get_metrics().to_vec();
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
