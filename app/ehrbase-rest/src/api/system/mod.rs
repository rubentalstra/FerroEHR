//! The ITS-REST **System API** (development edition, STABLE) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/system/` + the
//! `system-*.openapi.yaml` OAS group.
//!
//! One operation: `OPTIONS /` — the System Options and Conformance manifest.
//! Register: `docs/design/its-rest/system.md` (G-1 live endpoint list, G-2
//! runner-derived conformance profile, G-3 base-path mount, G-5 Accept).
//!
//! The System API is not part of the generated ITS-REST contract (the
//! `emit-rest` groups are `ehr`/`query`/`definition`/`admin`/`demographic`
//! only, `crates/openehr-its/src/rest/generated/`), so its single operation is
//! hand-written in [`options`]. [`options::route`] builds the `OPTIONS`
//! handler; the wiring layer (`crate::router`) constructs the
//! [`options::SystemManifest`] from config + the live mounted-group set and
//! mounts it at the API base-path root (and a bare-`/` compatibility alias),
//! above the CORS layer.

pub mod options;

pub use options::{SPEC_ENDPOINTS, SystemManifest, SystemOptionsConfig, route};
