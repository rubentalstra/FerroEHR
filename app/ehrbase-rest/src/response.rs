//! The typed response envelope carried out of the service seam (W2-A).
//!
//! The generated ITS-REST server traits exchange a bare `serde_json::Value` at
//! the boundary, which cannot carry the response headers the spec mandates
//! (`ETag`, `Location`) nor drive `Prefer` handling. Rather than scrape those
//! out of the canonical-JSON body (a stringly-typed anti-pattern), the
//! header-bearing EHR operations are served through the [`EhrService`] seam
//! (see [`crate::backend`]), whose methods return a [`ServiceResponse`]: the RM
//! payload **plus** typed resource metadata ([`ResourceMeta`]) from which the
//! HTTP edge derives the wire headers.
//!
//! Header derivation is per-operation and lives in the dispatch layer
//! (`dispatch::ehr`) — the service only surfaces the metadata, it does not know
//! the base path or which segment a resource lives under. This keeps the
//! `ehrbase-rest` → `ehrbase` dependency pointing downward: `ehrbase` (the
//! service) depends on this crate and constructs the envelope; the HTTP edge
//! consumes it.

use jiff::Timestamp;

/// Typed metadata about a versioned resource a write/read produced, from which
/// the HTTP edge derives the spec-mandated `ETag`/`Location` headers
/// (ITS-REST 1.0.3 — `headers/ETag_*.yaml`, `headers/Location_*.yaml`).
#[derive(Debug, Clone)]
pub struct ResourceMeta {
    /// The owning EHR id (the `Location` path root; the `ETag` value for an EHR).
    pub ehr_id: String,
    /// The resource's identifier that is both the `ETag` value (quoted) and the
    /// `Location` path tail: an `OBJECT_VERSION_ID` for a versioned resource
    /// (COMPOSITION / `EHR_STATUS` / FOLDER version), the `ehr_id` for an EHR, or
    /// the contribution uid for a CONTRIBUTION.
    pub uid: String,
    /// The commit time of this version (its audit `time_committed`). ITS-REST
    /// 1.0.3 declares no `Last-Modified` response header, so this is carried for
    /// completeness/observability but is not currently emitted at the wire.
    pub last_modified: Option<Timestamp>,
}

impl ResourceMeta {
    /// Metadata for a versioned resource: the owning EHR plus the resource `uid`
    /// (the `OBJECT_VERSION_ID` / `ehr_id` / contribution uid used as the `ETag`
    /// and `Location` tail).
    #[must_use]
    pub fn new(ehr_id: impl Into<String>, uid: impl Into<String>) -> Self {
        Self {
            ehr_id: ehr_id.into(),
            uid: uid.into(),
            last_modified: None,
        }
    }

    /// Attach the version's commit time (carried, not emitted — see the field).
    #[must_use]
    pub fn with_last_modified(mut self, at: Timestamp) -> Self {
        self.last_modified = Some(at);
        self
    }
}

/// A service response: the canonical-JSON RM payload plus optional resource
/// metadata. A `null` `body` means "no representation" (a logically deleted
/// read, or a resource whose value is not returned); `meta` is present whenever
/// the operation produced/identified a versioned resource, letting the HTTP edge
/// set `ETag`/`Location` per the operation's spec.
#[derive(Debug, Clone)]
pub struct ServiceResponse {
    /// The canonical-JSON RM payload, or `Value::Null` when there is none.
    pub body: serde_json::Value,
    /// Resource metadata for header derivation, if any.
    pub meta: Option<ResourceMeta>,
}

impl ServiceResponse {
    /// A response carrying both an RM body and resource metadata.
    #[must_use]
    pub fn new(body: serde_json::Value, meta: ResourceMeta) -> Self {
        Self {
            body,
            meta: Some(meta),
        }
    }

    /// A plain response with a body but no resource metadata (reads whose spec
    /// response declares no `ETag`/`Location`: VERSION wrappers, revision
    /// histories, EHR/FOLDER retrieval, item tags, contribution retrieval).
    #[must_use]
    pub fn plain(body: serde_json::Value) -> Self {
        Self { body, meta: None }
    }

    /// A bodyless response carrying resource metadata — the delete outcome that
    /// still returns the (now deleted) `version_uid` in `ETag`/`Location`
    /// (`204_COMPOSITION_deleted.yaml`).
    #[must_use]
    pub fn deleted(meta: ResourceMeta) -> Self {
        Self {
            body: serde_json::Value::Null,
            meta: Some(meta),
        }
    }

    /// Whether the body is absent (a logically deleted read → `204`, or a
    /// bodyless delete outcome).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.body.is_null()
    }
}
