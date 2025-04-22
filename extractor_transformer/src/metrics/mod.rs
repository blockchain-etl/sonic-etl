

#[cfg(feature="METRICS")]
pub mod prometheus;
#[cfg(feature="METRICS")]
pub use prometheus::Metrics;


#[cfg(not(feature="METRICS"))]
pub type Metrics = ();