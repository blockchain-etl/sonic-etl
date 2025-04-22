use prometheus::IntCounter;

/// A wrapper struct around each of our metrics.
#[cfg(feature="METRICS")]
#[derive(Clone)]
pub struct Metrics {
    // Total number of requests (including successful and failed)
    pub request_count: IntCounter,
    // Total number of failed requests.
    pub failed_request_count: IntCounter,
}