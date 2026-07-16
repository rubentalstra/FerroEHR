//! The ITS-REST wire-shaped DEFINITION extension methods: the rich shapes the
//! `DEFINITION` API group returns (template summaries, the example
//! COMPOSITION, `StoredQuery` descriptors, glob filters) that the SM
//! `I_DEFINITION_*` interfaces do not express. All native
//! (`serde_json::Value` and `SmError`), so this layer stays protocol-free;
//! the route wiring is the
//! ITS-REST layer's concern. The retrieval/store behaviour rides on the SM
//! logic in the sibling interface files.

use regex::Regex;
use serde_json::Value;

use openehr_flat::{DetailLevel, ExampleType};

use crate::service::EhrbaseService;
use crate::service::definition::types::TemplateListFilter;
use crate::service::list::Page;
use crate::service::status::{CallStatusType, SmError};

use super::{paginate, query::is_aql_v1};

impl EhrbaseService {
    /// `POST /definition/template/adl1.4` — parse + store an OPT 1.4
    /// canonical-XML template through the templates layer; the wire `201` body
    /// is the created template summary.
    ///
    /// # Errors
    ///
    /// - Unparseable / structurally invalid OPT XML → `invalid_template`
    /// (`422`).
    /// - A template with the same `template_id` already stored → conflict
    /// (`409`).
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
    /// `precondition_violation` (`400`).
    /// - No template with that `template_id` → `template_does_not_exist`
    /// (`404`).
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

    /// `POST /definition/template/adl2` — store ADL2 operational-template
    /// source (text/plain) and return the stored `ARCHETYPE_HRID`; the
    /// dispatcher builds `Location` + the `Prefer` body from it
    /// (`201_Template_adl2_upload`).
    ///
    /// PORT NOTE: duplicate handling diverges by surface. The REST
    /// contract declares `409_template_already_exists` on this endpoint
    /// (`definition-codegen.openapi.yaml` /definition/template/adl2 POST),
    /// while the SM native `upload_artefact` says "replace it"
    /// (`i_definition_adl2.adoc`). This is the REST adapter seam, so an
    /// existing HRID is a 409 here; native SM callers keep replace.
    ///
    /// # Errors
    ///
    /// - Source failing the registration validator →
    /// `precondition_violation` (`400`) on the pre-check, or
    /// `invalid_artefact` (`422`) from the store path.
    /// - An ADL2 artefact with the same HRID already stored → conflict
    /// (`409`).
    /// - A database failure (`exception` → `500`).
    pub async fn template_adl2_upload(&self, source: String) -> Result<String, SmError> {
        let meta = crate::validation::validate_adl2_source(&source)
            .map_err(|v| SmError::precondition(format!("{}: {}", v.code, v.detail)))?;
        if self.adl2_exists(&meta.hrid).await? {
            return Err(SmError::new(
                CallStatusType::CompositionAlreadyExists,
                format!("an ADL2 template with id '{}' already exists", meta.hrid),
            ));
        }
        Ok(self.adl2_upload(&source).await?)
    }

    /// `GET /definition/template/adl2` — the ADL2 twin of
    /// [`template_adl14_list`](Self::template_adl14_list). The store yields
    /// `{template_id, created_timestamp}` — `concept` is not extracted (no
    /// cADL source parser), so the `concept` filter matches nothing when
    /// supplied; `template_id`/`version` globs + `offset`/`fetch` are honoured
    /// over the full set here.
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
    /// → `exception` (`500`).
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
    /// "invalid AQL") — G-05-06. AQL bodies fall through to the store-time AQL
    /// syntactic check. The effective version is recovered by the dispatcher
    /// through the list seam for the `Location` header; the store itself is
    /// bodyless.
    ///
    /// # Errors
    ///
    /// - A non-AQL `query_type` → `precondition_violation` (`400`).
    /// - A body that fails the AQL parse → `precondition_violation` (`400`).
    /// - With an explicit `version`, an already-existing `(name, version)`
    /// pair → conflict (`409`, `409_StoredQuery_version.yaml`).
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
