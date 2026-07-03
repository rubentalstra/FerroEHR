//! **ITS-XML** — canonical XML serialization (openEHR ITS-XML), namespace
//! `http://schemas.openehr.org/v1`.
//!
//! Implemented with `quick-xml` and validated against the vendored XSDs in
//! `schemas/xml/` (v1 = 1.0.2 target, v2 = 2.0.0 reference). This is the one
//! wire format the `OpenEhrType` derive does not cover (it is JSON-only), so it
//! is hand-written here. Implementation lands in P5.
//!
//! `// TODO(port):` quick-xml (de)serializers for the RM classes EHRbase emits,
//! with `xmllint --c14n` as the C14N fallback.
