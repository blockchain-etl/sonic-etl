mod basic;
mod debug;

pub use basic::*;
pub use debug::*;

use alloy::{
    network::{Ethereum, Network},
    providers::Provider,
    transports::{BoxTransport, Transport},
};

#[allow(async_fn_in_trait)]
pub trait Extractor<P: Provider<T, N>, T: Transport + Clone = BoxTransport, N: Network = Ethereum> {
    /// Return the underlying provider used for data extraction
    fn provider(&self) -> &P;
}
