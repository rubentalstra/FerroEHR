//! The case universe, one module per schedule chapter (+ the cross-cutting
//! profile capabilities), fresh-authored from the W-10 registers
//! (`docs/design/conformance/01–13`). Every module exposes
//! `entries() -> Vec<CaseEntry>`; [`entries`] aggregates them for the
//! registry.
//!
//! Authoring law (register 90): ids carried from the pre-W-10 instrument
//! keep their slugs (the ECC number persists — baseline deltas stay
//! per-case explainable); wire ids come ONLY from [`crate::wire`]; per-SUT
//! facts from [`crate::engine::harness::RunContext::sut`]; edition-variant
//! assertions through [`crate::engine::assert::status_ladder`] /
//! [`crate::wire::headers`]; schedule data-set bounds declared via
//! [`crate::engine::harness::DataSetReport::of_schedule_rows`].

use crate::engine::registry::CaseEntry;

pub mod adl2;
pub mod admin;
pub mod aql_terminology;
pub mod composition;
pub mod content;
pub mod contribution;
pub mod definition_adl14;
pub mod definition_query;
pub mod demographic;
pub mod directory;
pub mod ehr;
pub mod message;
pub mod query;
pub mod query_golden;
pub mod security;
pub mod signing;
pub mod simplified_formats;
pub mod support;
pub mod terminology;

/// Every registered case, in suite order.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    let mut out = Vec::new();
    out.extend(ehr::entries());
    out.extend(composition::entries());
    out.extend(contribution::entries());
    out.extend(directory::entries());
    out.extend(definition_adl14::entries());
    out.extend(definition_query::entries());
    out.extend(query::entries());
    out.extend(query_golden::entries());
    out.extend(content::entries());
    out.extend(demographic::entries());
    out.extend(admin::entries());
    out.extend(message::entries());
    out.extend(security::entries());
    out.extend(signing::entries());
    out.extend(terminology::entries());
    out.extend(simplified_formats::entries());
    out.extend(adl2::entries());
    out.extend(aql_terminology::entries());
    out
}
