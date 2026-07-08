//! Operational-template CRUD (ITS-REST DEFINITION `adl1.4` group), on the
//! `template_store` table. Templates are uploaded as OPT 1.4 canonical XML; the
//! XML is stored verbatim (the authoritative artifact, and what GET returns) and
//! parsed into [`openehr_its::opt14::OperationalTemplate`] to extract the
//! `template_id` / `concept` / root-archetype metadata for indexing and listing.

use std::sync::Arc;

use openehr_flat::{DetailLevel, ExampleType, WebTemplate};
use serde_json::{Value, json};
use sqlx::Row;

use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Resolve the (cached) [`WebTemplate`] for a stored operational template,
    /// building it from the stored OPT 1.4 XML on first use.
    ///
    /// A template that is not in the store is reported as **`Unprocessable`**
    /// (→ ITS-REST `422`), not `NotFound`: on a composition commit an unknown
    /// referenced template is a *semantic* error, per
    /// `docs/specs/openehr/ITS-REST/specifications/responses/422_COMPOSITION.yaml`
    /// ("the underlying template is not known"), and the CNF Robot case
    /// `I_EHR_COMPOSITION.create_composition-event_bad_opt` asserts `422`.
    pub(super) async fn web_template_for(
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
        // Record cache hit/miss (§1.2 webtemplate_cache_events_total). The peek
        // is approximate under concurrency; good enough for a rate metric.
        let event = if self.web_templates.contains(template_id) {
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
            .get_or_build(template_id, || {
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
    /// The example is produced from the template's (cached) [`WebTemplate`] by
    /// [`openehr_flat::example_composition`] at the requested [`DetailLevel`],
    /// with a deterministic `uid` populated for the `output`
    /// ([`ExampleType::Output`]) form.
    ///
    /// An unknown `template_id` is a **`NotFound`** (→ ITS-REST `404`), matching
    /// the `adl1.4/{id}` GET surface (the endpoint's `404_unknown_template_id`
    /// response) rather than the `422` `web_template_for` maps for an unknown
    /// template on a *commit* path; a stored-but-unbuildable template stays a
    /// `422` (`Unprocessable`).
    pub(super) async fn template_example(
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

    /// Store an OPT 1.4 operational template from its canonical XML, returning
    /// the stored template's metadata descriptor.
    ///
    /// The XML is parsed to validate it is a well-formed OPT and to pull the
    /// `template_id` (the unique key), `concept`, and root archetype id.
    ///
    /// Operational templates are **immutable on the `adl1.4` upload endpoint**:
    /// re-uploading an existing `template_id` is a **`Conflict`** (→ ITS-REST
    /// `409`), never a silent overwrite. This matches
    /// `docs/specs/openehr/ITS-REST/specifications/responses/409_template_already_exists.yaml`
    /// ("409 Conflict is returned when a template with same `template_id` …
    /// already exists") and the CNF Robot case
    /// `I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict` ("upload same OPT
    /// again" → status 409). A legitimate replacement path (admin) is a later,
    /// separate concern; this endpoint must not mutate an existing template.
    pub(super) async fn store_template(&self, xml: &str) -> Result<Value, ServiceError> {
        let opt = openehr_its::opt14::from_xml(xml)
            .map_err(|e| ServiceError::Unprocessable(format!("invalid OPT 1.4 XML: {e}")))?;
        // The opt14 codec is tolerant: it skips unknown elements and lets a
        // repeated single-valued element overwrite (last wins). A conformant OPT
        // upload must reject both — a foreign top-level tag and a duplicated
        // single-valued top-level element (CNF master04
        // `upload_opt-invalid_opt`: `alien_tags` / `multiple_elements`).
        validate_opt_structure(xml)?;

        let template_id = opt.template_id.value;
        if template_id.trim().is_empty() {
            return Err(ServiceError::Unprocessable(
                "operational template has no template_id".to_owned(),
            ));
        }
        // `concept` is a mandatory `OPERATIONAL_TEMPLATE` attribute; an empty one
        // is a malformed OPT (CNF `removed_mandatory_elements/…removed_concept_value`).
        if opt.concept.trim().is_empty() {
            return Err(ServiceError::Unprocessable(
                "operational template has an empty concept".to_owned(),
            ));
        }
        let concept = Some(opt.concept);
        let root_archetype = {
            let a = opt.definition.archetype_id.value;
            (!a.trim().is_empty()).then_some(a)
        };

        // Insert-only: `DO NOTHING` on the `template_id` unique key makes the
        // duplicate case race-free (no overwrite, no SQLSTATE parsing) — an
        // affected-row count of 0 means the template already exists → 409.
        let inserted = sqlx::query(
            "INSERT INTO template_store (template_id, concept, root_archetype, content) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (template_id) DO NOTHING",
        )
        .bind(&template_id)
        .bind(&concept)
        .bind(&root_archetype)
        .bind(xml)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if inserted == 0 {
            return Err(ServiceError::Conflict(format!(
                "an operational template with template_id '{template_id}' already exists"
            )));
        }

        self.get_template_meta(&template_id).await
    }

    /// The metadata descriptor for one stored template.
    pub(super) async fn get_template_meta(&self, template_id: &str) -> Result<Value, ServiceError> {
        let row = sqlx::query(
            "SELECT template_id, concept, root_archetype, created_at \
             FROM template_store WHERE template_id = $1",
        )
        .bind(template_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("template {template_id}")))?;
        Ok(Self::template_json(&row))
    }

    /// The stored OPT 1.4 XML for a template (the canonical retrieval artifact).
    pub(super) async fn get_template_xml(&self, template_id: &str) -> Result<String, ServiceError> {
        sqlx::query_scalar::<_, String>("SELECT content FROM template_store WHERE template_id = $1")
            .bind(template_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("template {template_id}")))
    }

    /// List every stored template's metadata descriptor (by `template_id`).
    pub(super) async fn list_templates(&self) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT template_id, concept, root_archetype, created_at \
             FROM template_store ORDER BY template_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(Self::template_json).collect())
    }

    /// The openEHR template descriptor for one row (ITS-REST template list shape).
    fn template_json(row: &sqlx::postgres::PgRow) -> Value {
        let created = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")
            .map(|t| t.to_jiff().to_string())
            .unwrap_or_default();
        json!({
            "template_id": row.try_get::<String, _>("template_id").unwrap_or_default(),
            "concept": row.try_get::<Option<String>, _>("concept").ok().flatten(),
            "archetype_id": row.try_get::<Option<String>, _>("root_archetype").ok().flatten(),
            "created_timestamp": created,
        })
    }
}

/// The known top-level child elements of an OPT 1.4 `<template>`
/// (`OPERATIONAL_TEMPLATE`) — the wire names of
/// [`openehr_its::opt14::OperationalTemplate`]'s attributes. Any other top-level
/// element is a foreign / "alien" tag.
const OPT_TOP_LEVEL: &[&str] = &[
    "language",
    "is_controlled",
    "description",
    "revision_history",
    "uid",
    "template_id",
    "concept",
    "definition",
    "ontology",
    "component_ontologies",
    "annotations",
    "constraints",
    "view",
];

/// The top-level OPT attributes that legitimately repeat (`Vec`-valued):
/// `component_ontologies` and `annotations`. Every other attribute is
/// single-valued, so a repeated element is malformed.
const OPT_TOP_LEVEL_MULTIPLE: &[&str] = &["component_ontologies", "annotations"];

/// Reject an OPT the tolerant [`openehr_its::opt14`] codec would silently accept:
/// a **foreign** top-level element (CNF `invalid_templates/alien_tags`) or a
/// **duplicated single-valued** top-level element (CNF
/// `invalid_templates/multiple_elements` — `concept`/`definition`/`template_id`
/// twice). Only the direct children of the root `<template>` are inspected, so a
/// legitimate nested reuse of a name (e.g. `<template_id>` inside a term binding)
/// is unaffected. The codec already catches missing mandatory elements; this
/// closes the leniency gap the CNF `upload_opt-invalid_opt` case exercises.
fn validate_opt_structure(xml: &str) -> Result<(), ServiceError> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    let mut depth: i32 = 0;
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut check = |raw: &[u8]| -> Result<(), ServiceError> {
        let name = String::from_utf8_lossy(raw).into_owned();
        if !OPT_TOP_LEVEL.contains(&name.as_str()) {
            return Err(ServiceError::Unprocessable(format!(
                "operational template has an unexpected top-level element <{name}> \
                 (not an OPERATIONAL_TEMPLATE attribute)"
            )));
        }
        let count = seen.entry(name.clone()).or_insert(0);
        *count += 1;
        if *count > 1 && !OPT_TOP_LEVEL_MULTIPLE.contains(&name.as_str()) {
            return Err(ServiceError::Unprocessable(format!(
                "operational template has a duplicate single-valued <{name}> element"
            )));
        }
        Ok(())
    };

    loop {
        match reader.read_event() {
            // A direct child's Start is seen at depth 2 (root <template> is 1).
            Ok(Event::Start(e)) => {
                depth += 1;
                if depth == 2 {
                    check(e.local_name().as_ref())?;
                }
            }
            // A self-closing direct child is seen while the root (depth 1) is open.
            Ok(Event::Empty(e)) => {
                if depth == 1 {
                    check(e.local_name().as_ref())?;
                }
            }
            Ok(Event::End(_)) => depth -= 1,
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ServiceError::Unprocessable(format!(
                    "operational template XML is malformed: {e}"
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_opt_structure;

    fn tmpl(children: &str) -> String {
        format!(
            "<?xml version=\"1.0\"?>\n<template xmlns=\"http://schemas.openehr.org/v1\">{children}</template>"
        )
    }

    #[test]
    fn accepts_a_well_formed_top_level() {
        // Single-valued attributes once each + a repeated multi-valued one.
        let xml = tmpl(
            "<language/><template_id><value>t.v1</value></template_id>\
             <concept>C</concept><definition/>\
             <component_ontologies/><component_ontologies/>",
        );
        assert!(validate_opt_structure(&xml).is_ok());
    }

    #[test]
    fn rejects_a_foreign_top_level_element() {
        // CNF invalid_templates/alien_tags — a <bullfrog> tag at the top level.
        let xml = tmpl("<concept>C</concept><bullfrog>x</bullfrog>");
        let err = validate_opt_structure(&xml).unwrap_err().to_string();
        assert!(err.contains("bullfrog"), "{err}");
    }

    #[test]
    fn rejects_duplicate_single_valued_top_level() {
        // CNF invalid_templates/multiple_elements — concept/definition/template_id twice.
        for name in ["concept", "definition", "template_id"] {
            let xml = tmpl(&format!("<{name}>a</{name}><{name}>b</{name}>"));
            let err = validate_opt_structure(&xml).unwrap_err().to_string();
            assert!(err.contains("duplicate"), "{name}: {err}");
        }
    }

    #[test]
    fn ignores_nested_reuse_of_a_name() {
        // A nested <template_id> deeper in the tree must not count as a duplicate
        // of the single top-level one (valid OPTs carry nested references).
        let xml = tmpl(
            "<template_id><value>t.v1</value></template_id>\
             <definition><attributes><template_id>ref</template_id></attributes></definition>",
        );
        assert!(validate_opt_structure(&xml).is_ok());
    }
}
