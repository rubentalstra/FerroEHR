//! The ITS-REST wire-shaped DEFINITION extension methods: the rich shapes the
//! `DEFINITION` API group returns (template summaries, the example
//! COMPOSITION, `StoredQuery` descriptors, glob filters) that the SM
//! `I_DEFINITION_*` interfaces do not express. Native error types only
//! (`SmError`, or `ServiceError` where a structured per-code validation body is
//! carried — the ADL2 upload), so this layer stays protocol-free; the route
//! wiring is the ITS-REST layer's concern. The retrieval/store behaviour rides
//! on the SM logic in the sibling interface files.

use regex::Regex;
use serde_json::Value;

use openehr_its::flat::example::{DetailLevel, ExampleType};

use crate::service::EhrbaseService;
use crate::service::definition::types::TemplateListFilter;
use crate::service::error::ServiceError;
use crate::service::list::Page;
use crate::service::status::SmError;

use super::{paginate, query::is_aql_v1};

impl EhrbaseService {
    /// `POST /definition/template/adl1.4` — parse + store an OPT 1.4
    /// canonical-XML template through the templates layer; the wire `201` body
    /// is the created template summary.
    ///
    /// # Errors
    ///
    /// - Unparseable / structurally invalid OPT XML → `invalid_template`
    ///   (`422`).
    /// - A template with the same `template_id` already stored → conflict
    ///   (`409`).
    /// - A database failure (`exception` → `500`).
    pub async fn template_adl14_upload(&self, opt_xml: String) -> Result<Value, SmError> {
        Ok(self.store_template(&opt_xml).await?)
    }

    /// `GET /definition/template/adl1.4/{template_id}` — the OPT 1.4 canonical
    /// XML addressed by its `template_id` string (the ITS-REST wire address;
    /// the SM keys OPTs by `UUID`). Identity is case-insensitive.
    ///
    /// # Errors
    ///
    /// - No OPT with that `template_id` → `template_does_not_exist` (`404`).
    /// - A database failure (`exception` → `500`).
    pub async fn template_adl14_get(&self, template_id: String) -> Result<String, SmError> {
        Ok(self.opt_get_by_template_id(&template_id).await?)
    }

    /// `GET /definition/template/adl1.4` — the stored template summaries,
    /// filtered by the wire's `filter_template_id`/`concept`/`filter_version`
    /// globs (`*` wildcard) and cursored by `offset`/`fetch`
    /// (`operations/definition_template_adl1.4_list.yaml`).
    ///
    /// # Errors
    ///
    /// A database failure (`exception` → `500`).
    pub async fn template_adl14_list(
        &self,
        filter: TemplateListFilter,
        page: Page,
    ) -> Result<Vec<Value>, SmError> {
        Ok(filter_templates(
            self.template_summaries().await?,
            &filter,
            page,
        ))
    }

    /// `GET /definition/template/adl1.4/{template_id}/example` — an example
    /// COMPOSITION built from the template's `WebTemplate` by the templates
    /// layer. `kind`/`detail_level` are the dev-OAS
    /// `example_type`/`example_detail_level` enums.
    ///
    /// # Errors
    ///
    /// - An out-of-enum `detail_level` or `kind` value →
    ///   `precondition_violation` (`400`).
    /// - No template with that `template_id` → `template_does_not_exist`
    ///   (`404`).
    /// - A database failure (`exception` → `500`).
    pub async fn template_adl14_example(
        &self,
        template_id: String,
        detail_level: Option<String>,
        kind: Option<String>,
    ) -> Result<Value, SmError> {
        let level =
            DetailLevel::from_query(detail_level.as_deref()).map_err(SmError::precondition)?;
        let kind = ExampleType::from_query(kind.as_deref()).map_err(SmError::precondition)?;
        Ok(self.template_example(&template_id, level, kind).await?)
    }

    /// `GET /definition/template/adl2/{template_id}/example` — an example
    /// COMPOSITION built from the ADL2 template's `WebTemplate` (the am24 front
    /// end feeding the shared example generator). `kind`/`detail_level` are the
    /// `example_type`/`example_detail_level` query enums.
    ///
    /// # Errors
    ///
    /// - An out-of-enum `detail_level` or `kind` value → `BadRequest` (`400`).
    /// - No template with that `template_id` → `NotFound` (`404`).
    /// - The stored template cannot be compiled/built → `Unprocessable` (`422`).
    /// - A database failure (`500`).
    pub async fn template_adl2_example(
        &self,
        template_id: String,
        detail_level: Option<String>,
        kind: Option<String>,
    ) -> Result<Value, ServiceError> {
        let level =
            DetailLevel::from_query(detail_level.as_deref()).map_err(ServiceError::BadRequest)?;
        let kind = ExampleType::from_query(kind.as_deref()).map_err(ServiceError::BadRequest)?;
        self.adl2_example(&template_id, level, kind).await
    }

    /// `POST /definition/template/adl2` — validate ADL2 operational-template
    /// source (text/plain) through the `openehr-adl` engine, store it, and
    /// return the stored `ARCHETYPE_HRID`; the dispatcher builds `Location` +
    /// the `Prefer` body from it (`201_Template_adl2_upload`).
    ///
    /// Returns [`ServiceError`] (not `SmError`) so a semantic-validation failure
    /// keeps its structured per-code violations for the ITS-REST `Error` body
    /// (`schemas/others/Error.yaml`), exactly as the composition upload path
    /// does. Duplicate handling diverges by surface: the REST contract declares
    /// `409_template_already_exists.yaml`, while the SM native `upload_artefact`
    /// replaces (`i_definition_adl2.adoc`) — an existing HRID is a `409` here.
    ///
    /// # Errors
    ///
    /// - Unparseable source → `BadRequest` (`400`,
    ///   `definition_template_adl2_upload.yaml` → `responses/400.yaml`).
    /// - AOM2-invalid source → `ValidationFailed` (`422` with the rule-code
    ///   mnemonics).
    /// - An ADL2 artefact with the same HRID already stored → `Conflict`
    ///   (`409`).
    /// - A database failure (`500`).
    pub async fn template_adl2_upload(&self, source: String) -> Result<String, ServiceError> {
        self.adl2_wire_upload(&source).await
    }

    /// `GET /definition/template/adl2/{template_id}` (and the deprecated
    /// `…/{template_id}/{version}`) — the stored ADL2 source, resolved from a
    /// full or partial `template_id` (`+` optional SEMVER `version`). Served as
    /// `text/plain` (`200_Template_adl2_retrieved.yaml` body `oneOf:
    /// [OperationalTemplateV2, string]`, example = ADL2 source): the stored
    /// source is returned verbatim (lossless).
    ///
    /// # Errors
    ///
    /// - No stored template matches → `NotFound` (`404`).
    /// - A database failure (`500`).
    pub async fn template_adl2_source(
        &self,
        template_id: String,
        version: Option<String>,
    ) -> Result<String, ServiceError> {
        let hrid = self.adl2_resolve(&template_id, version.as_deref()).await?;
        self.adl2_get(&hrid).await
    }

    /// `GET /definition/template/adl2/{template_id}` with `Accept:
    /// application/json` — the `OperationalTemplateV2` canonical-JSON projection
    /// of the resolved template (`200_Template_adl2_retrieved.yaml`,
    /// `application/json` → `OperationalTemplateV2`).
    ///
    /// # Errors
    ///
    /// - No stored template matches → `NotFound` (`404`).
    /// - The OPT cannot be compiled → `Unprocessable` (`422`).
    /// - A database failure (`500`).
    pub async fn template_adl2_opt_json(
        &self,
        template_id: String,
        version: Option<String>,
    ) -> Result<String, ServiceError> {
        let hrid = self.adl2_resolve(&template_id, version.as_deref()).await?;
        let source = self.adl2_get(&hrid).await?;
        self.adl2_opt_json(&source).await
    }

    /// `GET /definition/template/adl2` — the ADL2 twin of
    /// [`template_adl14_list`](Self::template_adl14_list). The store yields
    /// `TemplateMetadata` rows (`template_id`, `concept`, `archetype_id`,
    /// `created_timestamp`); `template_id`/`concept`/`version` globs +
    /// `offset`/`fetch` are honoured over the full set here.
    ///
    /// # Errors
    ///
    /// A database failure (`exception` → `500`).
    pub async fn template_adl2_list(
        &self,
        filter: TemplateListFilter,
        page: Page,
    ) -> Result<Vec<Value>, SmError> {
        Ok(filter_templates(
            self.adl2_template_list(Page::all()).await?,
            &filter,
            page,
        ))
    }

    /// `GET /definition/query/{qualified_query_name}` — all stored versions of
    /// every query whose qualified name starts with `qualified_query_name`
    /// (a case-insensitive prefix; empty ⇒ all —
    /// `definition_query_list.yaml`).
    ///
    /// # Errors
    ///
    /// - A row-decode failure on a `NOT NULL` column (a genuine server fault)
    ///   → `exception` (`500`).
    /// - A database failure (`exception` → `500`).
    pub async fn query_list(&self, qualified_query_name: String) -> Result<Vec<Value>, SmError> {
        Ok(self.list_stored_queries(&qualified_query_name).await?)
    }

    /// `GET /definition/query/{qualified_query_name}/{version}` — one stored
    /// query at an exact version or a SEMVER prefix (`{major}` /
    /// `{major}.{minor}` → the highest matching stored version,
    /// `parameters/path/version.yaml`).
    ///
    /// # Errors
    ///
    /// - No stored query matching that name + version → not-found (`404`).
    /// - A row-decode failure (a genuine server fault) → `exception` (`500`).
    /// - A database failure (`exception` → `500`).
    pub async fn query_version_get(
        &self,
        qualified_query_name: String,
        version: String,
    ) -> Result<Value, SmError> {
        Ok(self
            .get_stored_query(&qualified_query_name, Some(&version))
            .await?)
    }

    /// `PUT /definition/query/{qualified_query_name}[/{version}]` — store a
    /// query under its qualified name. `query_type` is the query's formalism
    /// (`QUERY_DESCRIPTOR.formalism`, default `AQL`, case-insensitive). The
    /// build can only validate + store AQL, so a non-AQL formalism is an
    /// honest *unsupported-formalism* reject (a distinct `400`, not a blanket
    /// "invalid AQL"). AQL bodies fall through to the store-time AQL
    /// syntactic check. The effective version is recovered by the dispatcher
    /// through the list seam for the `Location` header; the store itself is
    /// bodyless.
    ///
    /// # Errors
    ///
    /// - A non-AQL `query_type` → `precondition_violation` (`400`).
    /// - A body that fails the AQL parse → `precondition_violation` (`400`).
    /// - With an explicit `version`, an already-existing `(name, version)`
    ///   pair → conflict (`409`, `409_StoredQuery_version.yaml`).
    /// - A database failure (`exception` → `500`).
    pub async fn query_store(
        &self,
        qualified_query_name: String,
        version: Option<String>,
        query_type: String,
        body: String,
    ) -> Result<(), SmError> {
        if !is_aql_v1(&query_type) {
            return Err(SmError::precondition(format!(
                "unsupported query formalism `{query_type}`: only AQL is supported for \
                 stored queries (parameters/query/query_type.yaml)"
            )));
        }
        self.store_query_version(&qualified_query_name, version.as_deref(), body)
            .await?;
        Ok(())
    }
}

/// Filter + paginate a list of template descriptors per the wire query params.
///
/// `template_id`/`version` glob-match the `template_id` field, `concept`
/// glob-matches the `concept` field (a record lacking a filtered field does not
/// match); `page` then applies `offset`/`fetch`
/// (`parameters/query/filter_template_id.yaml` — "supports wildcards `*`";
/// `master02-overview.adoc` §List Handling).
fn filter_templates(list: Vec<Value>, filter: &TemplateListFilter, page: Page) -> Vec<Value> {
    let tid = filter.template_id.as_deref().map(glob_to_regex);
    let concept = filter.concept.as_deref().map(glob_to_regex);
    let version = filter.version.as_deref().map(glob_to_regex);
    let matches = |re: &Option<Regex>, field: Option<&str>| match re {
        None => true,
        Some(re) => field.is_some_and(|v| re.is_match(v)),
    };
    let filtered = list.into_iter().filter(|row| {
        let template_id = row.get("template_id").and_then(Value::as_str);
        let concept_field = row.get("concept").and_then(Value::as_str);
        matches(&tid, template_id)
            && matches(&concept, concept_field)
            && matches(&version, template_id)
    });
    paginate(filtered, page)
}

/// Compile a glob pattern (`*` wildcard, per `filter_template_id.yaml`) into an
/// anchored regex; all other characters are matched literally. A bare `*`
/// matches everything; an empty pattern matches only the empty string.
fn glob_to_regex(pattern: &str) -> Regex {
    let escaped = pattern
        .split('*')
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join(".*");
    // The pattern is escaped except for `*` → `.*`, so compilation cannot fail —
    // a build-time invariant, not a runtime condition.
    #[allow(clippy::expect_used)]
    Regex::new(&format!("^{escaped}$")).expect("glob-derived regex is always valid")
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn glob_matches_literally_with_star_wildcard() {
        // `*` is the only wildcard (filter_template_id.yaml); everything else
        // is literal — regex metacharacters must not leak through.
        assert!(glob_to_regex("*").is_match("anything at all"));
        assert!(glob_to_regex("IPS*").is_match("IPS v1"));
        assert!(!glob_to_regex("IPS*").is_match("not IPS"));
        assert!(glob_to_regex("a.b").is_match("a.b"));
        assert!(!glob_to_regex("a.b").is_match("aXb"));
        assert!(glob_to_regex("").is_match(""));
        assert!(!glob_to_regex("").is_match("x"));
    }
}
