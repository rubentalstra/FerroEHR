//! The transcribed conformance cases, one module per schedule chapter
//! (design §4.1). The registry is assembled from [`entries`].
//!
//! The suite set grows chapter-by-chapter (design §8): the framework is valuable
//! from the honest zero state onward. Each entry cites its schedule reference and
//! the ITS-REST section its assertions concretize.

use crate::registry::CaseEntry;

mod ehr;

/// All implemented case entries, in registration order.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    ehr::entries()
}
