//! The identity + claim model: catalogue, case metadata, profiles, spec
//! versions, adjudication + fairness registers. Pure data — no I/O toward the
//! SUT lives here.

pub mod adjudication;
pub mod case;
pub mod catalog;
pub mod fairness;
pub mod profile;
pub mod provenance;
pub mod versions;
