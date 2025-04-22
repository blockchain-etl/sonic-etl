

use alloy::json_abi::Event;

/// Compares two [Event]s and returns an [EventComparison].  If the two [Event]s are equal, it will
/// return an [EventComparison::ExactlyEqual].  If the two [Event]s aren't the same, but they would
/// have the same decoding, it will return an [EventComparison::SameDecoding].
pub fn compare_events(event_a: &Event, event_b: &Event) -> EventComparison {

    // These are some pretty critical pieces in determining the signature, if not equivalent, we
    // terminate early.
    if event_a.name != event_b.name || event_a.inputs.len() != event_b.inputs.len() {
        return EventComparison::NoEquivalence;
    }

    // Tracks whether events are exactly the same.  Useful for differentiating between exactly equal
    // and same decoding.
    let mut exactly_same = false;
    
    // Iterate through the Event parameters.
    for (param_a, param_b) in event_a.inputs.iter().zip(event_b.inputs.iter()) {

        // If the types are not the same, they're effectively not the same
        if param_a.ty != param_b.ty {
            return EventComparison::NoEquivalence;
        }

        // If one is Indexed and the other is not, they would have the same signature but different
        // decoding processes, return this error.
        if param_a.indexed != param_b.indexed {
            return EventComparison::MismatchedParamIndexing;
        }

        // If exactly the same, we need to keep checking.
        if exactly_same && (param_a.name != param_b.name) {
            exactly_same = false;
        }
    }

    // At this point, the decoding is effectively the same, now we return whether it is exactly
    // the same or a same decoding
    if exactly_same {
        EventComparison::ExactlyEqual
    } else {
        EventComparison::SameDecoding
    }

    
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventComparison {
    /// Returned when two [Event]s are exactly the same
    ExactlyEqual,
    /// Returned whenever there is no reason to compare them
    NoEquivalence,
    /// Returns whenever the events may vary, but they would be decoded in the same way.
    SameDecoding,
    /// Returns whenever the events' parameters differ in whether they're indexed or not.  Returned
    /// on the first mismatched indexing assuming all else is accurate
    MismatchedParamIndexing
}

impl EventComparison {

    /// This function returns true if these two events would lead to the same decoded output.
    #[inline]
    pub fn decodes_same(&self) -> bool {
        matches!(self, Self::ExactlyEqual | Self::SameDecoding)
    }

}