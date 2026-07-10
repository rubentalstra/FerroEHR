//! Our own ECC conformance cases, one module per catalogue area (design §4.1).
//! The registry is assembled from [`entries`].
//!
//! Every case is a native ECC case with its own `<area>/<case>` slug, human
//! title, declared [`crate::catalog::Area`], and spec citation — no legacy CNF
//! ids, no runtime mapping to the frozen upstream corpus (that corpus was
//! design-time reference reading only). Each module's cases cite the current
//! pinned specifications (ITS-REST 1.0.3, RM 1.2.0, AM 1.4, AQL 1.1, SM) their
//! assertions concretize; where a design-time reading of the old corpus is the
//! only grounding, the citation records it as a reference.

use crate::registry::CaseEntry;

pub mod content;
pub mod signing;
pub mod support;

mod admin;
mod composition;
mod contribution;
mod definition_adl14;
mod definition_query;
mod demographic;
mod directory;
mod ehr;
mod message;
mod query;

/// All implemented case entries, in registration order.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut all = Vec::new();
    all.extend(ehr::entries()); // master06
    all.extend(composition::entries()); // master07
    all.extend(contribution::entries()); // master08
    all.extend(directory::entries()); // master09
    all.extend(definition_adl14::entries()); // master04
    all.extend(definition_query::entries()); // master05
    all.extend(query::entries()); // master11
    all.extend(admin::entries()); // master12 (OPTIONS)
    all.extend(demographic::entries()); // master10 (OPTIONS)
    all.extend(content::entries()); // master15/16/17.x
    all.extend(signing::entries()); // runner-defined SIGN-* (§4.6)
    all.extend(message::entries()); // master13 (SM-5 Messaging; native-API-only, skipped)
    all
}
