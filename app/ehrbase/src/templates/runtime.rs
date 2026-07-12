//! Derived-runtime resolution: the cached [`WebTemplate`] and example
//! COMPOSITION surfaces (S-08 / S-09), thin over `openehr-flat`.
//!
//! # Spec basis
//!
//! `docs/specs/openehr/BASE/docs/architecture_overview/master10-archetypes.adoc`:
//!
//! - §Archetypes and Templates at Runtime (S-08): a template's runtime function
//!   is (a) to validate data at capture/import against the RM + archetypes, and
//!   (b) to be the design basis for AQL paths. The validation/commit path
//!   consumes [`EhrbaseService::web_template_for`].
//! - §Deploying Archetypes and Templates (S-09): the spec blesses a *compiled
//!   near-runtime form* that incorporates copies of the relevant archetypes for
//!   performance and to guarantee only validated artefacts run. Our derived form
//!   is the [`WebTemplate`], memoised in a `moka` cache (G-T05).
//!
//! PORT NOTE (G-T06 — WebTemplate format is spec-silent): the concrete
//! WebTemplate JSON shape is **not openEHR-normative** — it is the Better
//! `web-template` SDT format and lives entirely in `openehr-flat` (a
//! hand-written spec-adjacent crate). This module only *stores, resolves, and
//! caches* it; it never presents WebTemplate as canonical openEHR, and the
//! builder's own id-sanitisation is a **vendor** rule, distinct from the
//! §Composite Identifiers and Case identity law applied to the cache key here
//! (see [`crate::templates::identity`]).

use std::sync::Arc;

use openehr_flat::{DetailLevel, ExampleType, WebTemplate};
use serde_json::Value;

use super::identity;
use crate::service::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Resolve the (cached) [`WebTemplate`] for a stored operational template,
    /// building it from the stored OPT 1.4 XML on first use (S-09).
    ///
    /// A template that is not in the store is reported as **`Unprocessable`**
    /// (→ ITS-REST `422`), not `NotFound` (G-T08): on a composition commit an
    /// unknown referenced template is a *semantic* error, per
    /// `docs/specs/openehr/ITS-REST/specifications/responses/422_COMPOSITION.yaml`
    /// ("the underlying template is not known"), and the CNF Robot case
    /// `I_EHR_COMPOSITION.create_composition-event_bad_opt` asserts `422`.
    ///
    /// The cache is keyed by the §Composite Identifiers and Case canonical form
    /// of `template_id` (G-T04), so case variants of one stored template resolve
    /// to a single cached [`WebTemplate`].
    pub(crate) async fn web_template_for(
        &self,
        template_id: &str,
    ) -> Result<Arc<WebTemplate>, ServiceError> {
        let xml = match self.get_template_xml(template_id).await {
            Ok(xml) => xml,
            Err(ServiceError::NotFound(_)) => {
                return Err(ServiceError::Unprocessable(format!(
                    "operational template not known: {template_id}"
                )));
            }
            Err(e) => return Err(e),
        };
        // Key the cache on the identity-canonical form so a case variant does not
        // build (and store) a second entry for the same template (G-T04).
        let key = identity::canonical_key(template_id);
        // Record cache hit/miss (§1.2 webtemplate_cache_events_total). The peek is
        // approximate under concurrency; good enough for a rate metric.
        let event = if self.web_templates.contains(&key) {
            "hit"
        } else {
            "miss"
        };
        metrics::counter!(
            crate::telemetry::prometheus::WEBTEMPLATE_CACHE_EVENTS,
            "event" => event,
        )
        .increment(1);

        self.web_templates
            .get_or_build(&key, || {
                let opt = openehr_its::opt14::from_xml(&xml)
                    .map_err(|e| openehr_flat::FlatError::OptParse(e.to_string()))?;
                openehr_flat::build_web_template(&opt)
            })
            .await
            .map_err(|e| {
                ServiceError::Unprocessable(format!(
                    "operational template {template_id} could not be built into a WebTemplate: {e}"
                ))
            })
    }

    /// Generate an example COMPOSITION for a stored operational template
    /// (`GET /definition/template/adl1.4/{template_id}/example`).
    ///
    /// PORT NOTE: example generation is **not spec-mandated** — it is a
    /// convenience surface (S-08(a)-adjacent). The example is produced from the
    /// template's (cached) [`WebTemplate`] by [`openehr_flat::example_composition`]
    /// at the requested [`DetailLevel`], with a deterministic `uid` populated for
    /// the `output` ([`ExampleType::Output`]) form.
    ///
    /// An unknown `template_id` is a **`NotFound`** (→ ITS-REST `404`), matching
    /// the `adl1.4/{id}` GET surface (its `404_unknown_template_id` response)
    /// rather than the `422` [`web_template_for`](Self::web_template_for) maps for
    /// an unknown template on a *commit* path; a stored-but-unbuildable template
    /// stays a `422` (`Unprocessable`).
    pub(crate) async fn template_example(
        &self,
        template_id: &str,
        level: DetailLevel,
        kind: ExampleType,
    ) -> Result<Value, ServiceError> {
        // Resolve existence first so an unknown id is a 404 (not the 422 the
        // WebTemplate cache maps for a commit-time unknown template).
        let _ = self.get_template_xml(template_id).await?;
        let wt = self.web_template_for(template_id).await?;
        let mut composition = openehr_flat::example_composition(&wt, level);
        if kind == ExampleType::Output {
            openehr_flat::apply_output_uid(&mut composition, template_id);
        }
        Ok(composition)
    }
}
