
pub mod generic;
pub mod erc;
pub mod mapped;
pub mod null;
pub mod compare;


#[allow(unused)]
pub use generic::{EventCatalog, GetEventBySigErr, LogDecodeErr, DynEventCatalog, AddEventErr, PopEventErr};
pub use erc::ErcEventCatalog;
#[allow(unused)]
pub use mapped::EventMapCatalog;
#[allow(unused)]
pub use null::NullEventCatalog;
pub use compare::{EventComparison, compare_events};