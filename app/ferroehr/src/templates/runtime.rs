//! Derived-runtime resolution: the cached [`WebTemplate`] and example
//! COMPOSITION surfaces, thin over `openehr_its::flat`.
//!
//! # Spec basis
//!
//! `docs/specs/openehr/BASE/docs/architecture_overview/master10-archetypes.adoc`:
//!
//! - §Archetypes and Templates at Runtime: a template's runtime function
//!   is (a) to validate data at capture/import against the RM + archetypes, and
//!   (b) to be the design basis for AQL paths. The validation/commit path
//!   consumes [`FerroEhrService::web_template_for`].
//! - §Deploying Archetypes and Templates: the spec blesses a *compiled
//!   near-runtime form* ("compiled into a near-runtime form from the sharable
//!   openEHR form") that incorporates copies of the relevant archetypes for
//!   performance and to guarantee only validated artefacts run. Our derived
//!   form is the [`WebTemplate`], memoised in a `moka` cache.
//!
//! NOTE (`WebTemplate` format is spec-silent): the concrete
//! `WebTemplate` JSON shape is **not openEHR-normative** — it is the Better
//! `web-template` SDT format and lives entirely in `openehr_its::flat` (a
//! hand-written spec-adjacent module). This module only *stores, resolves, and
//! caches* it; it never presents `WebTemplate` as canonical openEHR, and the
//! builder's own id-sanitisation is a **vendor** rule, distinct from the
//! §Composite Identifiers and Case identity law applied to the cache key here
//! (see [`crate::templates::identity`]).

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): stored OPT/WebTemplate artefacts served verbatim \
              (families 1/8)"
)]

use std::sync::Arc;

use openehr_its::flat::example::{DetailLevel, ExampleType};
use openehr_its::flat::webtemplate::WebTemplate;
use serde_json::Value;

use super::identity;
use crate::service::FerroEhrService;
use crate::service::error::{ServiceError, Violation};

impl FerroEhrService {
    /// Resolve the (cached) [`WebTemplate`] for a stored operational template,
    /// building it from the stored OPT 1.4 XML on first use.
    ///
    /// The cache is keyed by the §Composite Identifiers and Case canonical form
    /// of `template_id`, so case variants of one stored template resolve
    /// to a single cached [`WebTemplate`].
    ///
    /// # Errors
    ///
    /// - [`ServiceError::Unprocessable`] (→ ITS-REST `422`, **not** `NotFound`)
    ///   when the template is not in the store: on a composition
    ///   commit an unknown referenced template is a *semantic* error, per
    ///   `docs/specs/openehr/ITS-REST/specifications/responses/422.yaml`
    ///   ("semantic validation errors, such as the underlying template is not
    ///   known") and the CNF Robot case
    ///   `I_EHR_COMPOSITION.create_composition-event_bad_opt` asserting `422`.
    /// - [`ServiceError::Unprocessable`] when the stored XML fails to build
    ///   into a [`WebTemplate`].
    /// - [`ServiceError::Database`] — the store read failed.
    pub(crate) async fn web_template_for(
        &self,
        template_id: &str,
    ) -> Result<Arc<WebTemplate>, ServiceError> {
        // Key the cache on the identity-canonical form so a case variant resolves
        // to the single entry for the same template.
        let key = identity::canonical_key(template_id);

        // Fast path: a built WebTemplate is already resident — serve it without
        // touching `template_store`. This is the hot commit path: composition
        // validation calls `web_template_for` once per commit, and once a
        // template is warm every subsequent commit is a pure in-memory hit (no
        // per-commit OPT read). No openEHR spec governs this cache — the spec blesses
        // a compiled near-runtime form; the caching mechanics are our own design.
        if let Some(wt) = self.web_templates.get(&key).await {
            note_cache_event("hit");
            return Ok(wt);
        }
        note_cache_event("miss");

        // Miss: load the stored OPT XML (the one store read, amortised across
        // every future commit for this template). When the id is not an ADL 1.4
        // template, fall back to the ADL2/OPT2 store
        // (`web_template_adl2_cached`), so a FLAT/STRUCTURED commit keyed to an
        // ADL2-registered template resolves and is archetype-constraint-checked.
        // Only after *both* stores miss is the id "operational template not
        // known" (the commit-path 422). No openEHR spec governs the internal
        // resolver wiring — our own design/extension.
        let xml = match self.get_template_xml(template_id).await {
            Ok(xml) => xml,
            Err(ServiceError::NotFound(_)) => {
                return self.web_template_adl2_cached(template_id).await;
            }
            Err(e) => return Err(e),
        };

        self.build_cached_web_template(&key, template_id, &xml)
            .await
    }

    /// Generate an example COMPOSITION for a stored operational template
    /// (`GET /definition/template/adl1.4/{template_id}/example`).
    ///
    /// NOTE: example generation is **not spec-mandated** — it is a
    /// convenience surface. The example is produced from the template's (cached)
    /// [`WebTemplate`] by
    /// [`example_composition`](openehr_its::flat::example::example_composition) at the
    /// requested [`DetailLevel`], with a deterministic `uid` populated for
    /// the `output` ([`ExampleType::Output`]) form.
    ///
    /// The store is read unconditionally — the read doubles as the existence
    /// probe *and* supplies the XML for a cold-cache build in one round-trip —
    /// so a template deleted from the store is never served from a stale cache
    /// entry on this surface.
    ///
    /// # Errors
    ///
    /// - [`ServiceError::NotFound`] (→ ITS-REST `404`,
    ///   `responses/404_unknown_template_id.yaml`) — unknown `template_id`,
    ///   matching the `adl1.4/{template_id}` GET surface rather than the `422`
    ///   [`web_template_for`](Self::web_template_for) maps for an unknown
    ///   template on a *commit* path.
    /// - [`ServiceError::Unprocessable`] (→ `422`) — the template is stored but
    ///   cannot be built into a [`WebTemplate`].
    /// - [`ServiceError::Database`] — the store read failed.
    pub(crate) async fn template_example(
        &self,
        template_id: &str,
        level: DetailLevel,
        kind: ExampleType,
    ) -> Result<Value, ServiceError> {
        // Resolve existence first so an unknown id is a 404 (not the 422 the
        // WebTemplate cache maps for a commit-time unknown template).
        let xml = self.get_template_xml(template_id).await?;
        let key = identity::canonical_key(template_id);
        let wt = if let Some(wt) = self.web_templates.get(&key).await {
            note_cache_event("hit");
            wt
        } else {
            note_cache_event("miss");
            self.build_cached_web_template(&key, template_id, &xml)
                .await?
        };
        let mut composition = openehr_its::flat::example::example_composition(&wt, level);
        if kind == ExampleType::Output {
            openehr_its::flat::example::apply_output_uid(&mut composition, template_id);
        }
        Ok(composition)
    }

    /// Build the [`WebTemplate`] for `xml` and cache it under `key` (the
    /// §Composite Identifiers and Case canonical form of `template_id`).
    ///
    /// The expensive build is single-flighted by the cache's `get_or_build`
    /// (one build per key under contention); only a *successful* build is
    /// cached, so no negative entry can shadow a later upload.
    ///
    /// # Errors
    ///
    /// [`ServiceError::Unprocessable`] (→ `422`) — the stored XML does not
    /// re-parse as an OPT or the OPT does not build into a [`WebTemplate`]
    /// (a stored-but-unbuildable template).
    async fn build_cached_web_template(
        &self,
        key: &str,
        template_id: &str,
        xml: &str,
    ) -> Result<Arc<WebTemplate>, ServiceError> {
        self.web_templates
            .get_or_build(key, || {
                let opt = openehr_its::opt14::from_xml(xml)
                    .map_err(|e| openehr_its::flat::error::FlatError::OptParse(e.to_string()))?;
                openehr_its::flat::webtemplate::build_web_template(&opt)
            })
            .await
            .map_err(|e| {
                ServiceError::content_invalid(Violation::new(format!(
                    "operational template {template_id} could not be built into a WebTemplate: {e}"
                )))
            })
    }
}

/// Record one WebTemplate-cache hit/miss on the
/// [`crate::telemetry::metrics::WEBTEMPLATE_CACHE_EVENTS`] counter. No
/// openEHR spec governs this — our own observability design.
fn note_cache_event(event: &'static str) {
    crate::telemetry::metrics::metrics()
        .webtemplate_cache_events
        .add(1, &[opentelemetry::KeyValue::new("event", event)]);
}
