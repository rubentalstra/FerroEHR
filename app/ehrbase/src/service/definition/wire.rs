//! The ITS-REST wire-shaped [`DefinitionAdapter`] extension: the rich shapes the
//! `DEFINITION` API group returns (template summaries, the example COMPOSITION,
//! `StoredQuery` descriptors, glob filters) that the SM `I_DEFINITION_*`
//! interfaces do not express. All native (`serde_json::Value` + `SmError`), so
//! `ehrbase-sm` stays protocol-free; the route wiring is the ITS-REST layer's
//! concern. The retrieval/store behaviour rides on the SM logic in the sibling
//! interface files.

use regex::Regex;
use serde_json::Value;

use crate::service::definition::types::TemplateListFilter;
use crate::service::list::Page;
use crate::service::status::{CallStatusType, SmError};
use openehr_flat::{DetailLevel, ExampleType};

use crate::service::EhrbaseService;

impl EhrbaseService {
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn template_adl14_upload(&self, opt_xml: String) -> Result<Value, SmError> {
        // The OPT 1.4 canonical XML is parsed + stored through the templates
        // layer; the wire `201` body is the created template summary.
        Ok(self.store_template(&opt_xml).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn template_adl14_get(&self, template_id: String) -> Result<String, SmError> {
        Ok(self.opt_get_by_template_id(&template_id).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn template_adl14_list(
        &self,
        filter: TemplateListFilter,
        page: Page,
    ) -> Result<Vec<Value>, SmError> {
        // The wire decodes `filter_template_id`/`concept`/`filter_version`
        // (glob, `*` wildcard) + `offset`/`fetch`
        // (`operations/definition_template_adl1.4_list.yaml`). Filter + paginate
        // the stored template descriptors (from the templates layer) here.
        Ok(filter_templates(
            self.list_templates_response().await?,
            &filter,
            page,
        ))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn template_adl14_example(
        &self,
        template_id: String,
        detail_level: Option<String>,
        kind: Option<String>,
    ) -> Result<Value, SmError> {
        // `type`/`detail_level` are the dev-OAS `example_type`/`example_detail_level`
        // enums; an out-of-enum value is a `precondition_violation` (→ `400`).
        let level =
            DetailLevel::from_query(detail_level.as_deref()).map_err(SmError::precondition)?;
        let kind = ExampleType::from_query(kind.as_deref()).map_err(SmError::precondition)?;
        // The example COMPOSITION is built from the WebTemplate by the templates
        // layer.
        Ok(self.template_example(&template_id, level, kind).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn template_adl2_upload(&self, source: String) -> Result<String, SmError> {
        // ADL2 operational-template source (text/plain). Store it and return the
        // stored ARCHETYPE_HRID; the dispatcher builds `Location` + the `Prefer`
        // body from it (`201_Template_adl2_upload`). Invalid source → 422.
        //
        // PORT NOTE (G-05-12): duplicate handling diverges by surface. The REST
        // contract declares `409_template_already_exists` on this endpoint
        // (`definition-codegen.openapi.yaml` /definition/template/adl2 POST),
        // while the SM native `upload_artefact` says "replace it"
        // (`i_definition_adl2.adoc`). This is the REST adapter seam, so an
        // existing HRID is a 409 here; native SM callers keep replace.
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

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn template_adl2_list(
        &self,
        filter: TemplateListFilter,
        page: Page,
    ) -> Result<Vec<Value>, SmError> {
        // The ADL2 twin of `template_adl14_list`. The store yields
        // `{template_id, created_timestamp}` — `concept` is not extracted (no
        // cADL source parser), so the `concept` filter matches nothing when
        // supplied; `template_id`/`version` globs + `offset`/`fetch` are honoured
        // over the full set here.
        Ok(filter_templates(
            self.adl2_template_list(Page::all()).await?,
            &filter,
            page,
        ))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn query_list(&self, qualified_query_name: String) -> Result<Vec<Value>, SmError> {
        Ok(self.list_stored_queries(&qualified_query_name).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn query_version_get(
        &self,
        qualified_query_name: String,
        version: String,
    ) -> Result<Value, SmError> {
        Ok(self
            .get_stored_query(&qualified_query_name, Some(&version))
            .await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn query_store(
        &self,
        qualified_query_name: String,
        version: Option<String>,
        query_type: String,
        body: String,
    ) -> Result<(), SmError> {
        // `query_type` is the query's formalism (`QUERY_DESCRIPTOR.formalism`,
        // default `AQL`, case-insensitive). The build can only validate + store
        // AQL, so a non-AQL formalism is an honest *unsupported-formalism* reject
        // (a distinct `400`, not a blanket "invalid AQL") — G-05-06. AQL bodies
        // fall through to the AQL syntactic check in `store_query`.
        if !is_aql_formalism(&query_type) {
            return Err(SmError::precondition(format!(
                "unsupported query formalism `{query_type}`: only AQL is supported for \
                 stored queries (parameters/query/query_type.yaml)"
            )));
        }
        // The effective version is recovered by the dispatcher through the list
        // seam for the `Location` header; the store itself is bodyless.
        self.store_query_response(&qualified_query_name, version.as_deref(), body)
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
    let offset = usize::try_from(page.offset()).unwrap_or(usize::MAX);
    let skipped = filtered.skip(offset);
    match page.limit() {
        Some(n) => skipped
            .take(usize::try_from(n).unwrap_or(usize::MAX))
            .collect(),
        None => skipped.collect(),
    }
}

/// Compile a glob pattern (`*` wildcard, per `filter_template_id.yaml`) into an
/// anchored regex; all other characters are matched literally. An empty pattern
/// (or a bare `*`) matches everything.
fn glob_to_regex(pattern: &str) -> Regex {
    let mut re = String::with_capacity(pattern.len() + 4);
    re.push('^');
    for ch in pattern.chars() {
        if ch == '*' {
            re.push_str(".*");
        } else {
            re.push_str(&regex::escape(&ch.to_string()));
        }
    }
    re.push('$');
    // The pattern is escaped except for `*` → `.*`, so compilation cannot fail —
    // a build-time invariant, not a runtime condition.
    #[allow(clippy::expect_used)]
    Regex::new(&re).expect("glob-derived regex is always valid")
}

/// True if `query_type` names AQL (case-insensitive, optional `::version` whose
/// major is 1 or absent) — the only formalism the build can validate + store
/// (`parameters/query/query_type.yaml`; `master04` §Query Formalism, which
/// sanctions "any other string value" that we reject, G-05-06).
fn is_aql_formalism(query_type: &str) -> bool {
    let (name, version) = match query_type.split_once("::") {
        Some((n, v)) => (n, Some(v)),
        None => (query_type, None),
    };
    if !name.trim().eq_ignore_ascii_case("aql") {
        return false;
    }
    let major = version
        .and_then(|v| v.trim().split('.').next())
        .filter(|s| !s.is_empty())
        .unwrap_or("1");
    major == "1"
}
