use std::collections::HashMap;

#[derive(Eq, PartialEq, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    // Summary,
}

// receive from mq, topic name as job name
#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct MetricKey {
    /// Metric type
    metric_type: MetricType,
    /// Metric name
    name: String,
    /// Metric label set, include constant labels and variable labels
    labels: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct Metric {
    key: MetricKey,
    // prometheus only support f64
    value: f64,
}
