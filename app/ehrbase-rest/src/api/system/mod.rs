//! The ITS-REST **System API** (development edition, STABLE) —
//! `docs/specs/openehr/ITS-REST/specifications/docs/system/` + the
//! `system-*.openapi.yaml` OAS group.
//!
//! One operation: `OPTIONS /` — the System Options and Conformance manifest.
//! Register: `docs/design/its-rest/system.md` (G-1 live endpoint list, G-2
//! runner-derived conformance profile, G-3 base-path mount, G-5 Accept).

pub mod options;
