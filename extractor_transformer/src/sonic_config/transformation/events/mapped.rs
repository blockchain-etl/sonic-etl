use std::collections::{hash_map::Entry, HashMap};

use alloy::{json_abi::Event, primitives::B256};
use log::info;

use super::generic::{DynEventCatalog, EventCatalog};
use super::{compare_events, EventComparison};

#[derive(Debug, Clone, Default)]
pub struct EventMapCatalog {
    map: HashMap<(B256, u8), Event>
}

impl EventMapCatalog {
    pub fn new() -> Self {
        Self {
            map: HashMap::new()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity)
        }
    }
}

impl EventCatalog for EventMapCatalog {
    #[inline]
    fn get_event_by_signature_and_ntopics(&self, signature: &B256, n_topics: u8) -> Result<&Event, super::GetEventBySigErr> {
        match self.map.get(&(*signature, n_topics)) {
            Some(event) => Ok(event),
            None => Err(super::generic::GetEventBySigErr::NotFound)
        }
        
    }
}

impl DynEventCatalog for EventMapCatalog {
    #[inline]
    fn add_event(&mut self, event: &Event) -> Result<(), super::generic::AddEventErr> {
        match self.map.entry((event.selector(), event.num_topics() as u8)) {
            Entry::Vacant(slot) => {
                slot.insert(event.clone());
                Ok(())
            },
            Entry::Occupied(placed_event) => {

                match compare_events(placed_event.get(), event) {
                    EventComparison::ExactlyEqual => (),
                    EventComparison::SameDecoding => {
                        info!("Found events with same signature with some differing data, decodes same, ignoring secondary.");
                    },
                    EventComparison::MismatchedParamIndexing => {
                        log::error!("Critical Event Signature error occured, two signatures point to events with diff param indexing policies {:#?} VS {:#?}", placed_event, event);
                    },
                    EventComparison::NoEquivalence => {
                        unreachable!("The odds of this occuring are almost none, two separate events led to the same hash: {:#?} VS {:#?}", placed_event, event);
                    }
                }

                Ok(())
            },
        }
    }

    #[inline]
    fn pop_event(&mut self, signature: &B256, n_topics: u8) -> Result<Event, super::PopEventErr> {
        match self.map.remove(&(*signature, n_topics)) {
            Some(event) => Ok(event),
            None => Err(super::generic::PopEventErr::DoesNotExist)
        }
    }

    // fn pop_event(&mut self, signature: &B256, n) -> Result<Event, super::generic::PopEventErr> {
    // }
}