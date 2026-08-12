// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The Subject Proxy service (`service/subject_proxy/`).
//!
//! Realizes SM `I_SUBJECT_PROXY_SERVICE` / `I_DATA_BINDING`
//! (`docs/specs/openehr/SM/docs/openehr_platform/master10-subject_proxy_service.adoc`
//! and the UML classes `i_subject_proxy_service.adoc`, `i_data_binding.adoc`)
//! over the `sp_*` configuration + sample stores.
//!
//! Layout, one file per concern:
//!
//! - `service` — the public `I_SUBJECT_PROXY_SERVICE` operations
//!   (registration, data sets, variable/data-set reads, bindings, reset,
//!   manual-sample notification) + the sample-resolution glue.
//! - `frames` — `I_DATA_BINDING.get_frame`: frame dispatch with the
//!   primary→fallback pipeline (`data_frame.adoc`).
//! - `extract` — `frame_path` extraction: `DATA_FRAME_SAMPLE` → typed
//!   `VARIABLE_VALUE`.
//! - `freshness` — currency/freshness semantics (master10 §Samples).
//! - `store` — the `sp_*` row mapping (master10 §Persistence).
//! - [`config`] — the `[subject_proxy]` FHIR-frame executor configuration.
//! - [`binding`] / [`data_set`] / [`sample`] / [`value`] / [`variable`] — the
//!   SM information structures (`ENV_BINDING`/`DATA_FRAME`/`SYSTEM_CALL`,
//!   `SUBJECT_DATA_SET`/`DATA_SET_RESULT`, `SAMPLE<T>`, `VARIABLE_VALUE`,
//!   `SUBJECT_VARIABLE`).
//!
//! NOTE (design-filled preconditions/errors). The SM declares only
//! `__Pre_…__` clauses and no error codes; every unmet precondition surfaces as
//! `SmError(PreconditionViolation, …)` (→ `400`).

pub mod config;
mod extract;
mod frames;
mod freshness;
mod service;
mod store;

pub mod binding;
pub mod data_set;
pub mod sample;
pub mod value;
pub mod variable;
