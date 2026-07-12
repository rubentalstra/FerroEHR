//! The DEFINITION surface on [`EhrbaseService`]: the SM `I_DEFINITION_*`
//! interfaces ([`DefinitionAdl14Service`] / [`DefinitionAdl2Service`] /
//! [`DefinitionQueryService`]) plus the ITS-REST wire-shaped
//! [`DefinitionAdapter`] extension (the generated ITS-REST
//! `DefinitionApi` is no longer a `Platform` supertrait; the definition
//! dispatcher maps the wire onto these native traits).
//!
//! Storage: stored queries on the `stored_query` table; OPT 1.4 operational
//! templates on the `template_store` table (ingested into the
//! `openehr-its::opt14` model); ADL2 artefacts on the `adl2_artefact` store
//! (SM-2). The ADL2 `get` is served via the SM `DefinitionAdl2Service::get_artefact`
//! seam in the dispatcher (ADL2 is text); the ADL2 `example`/`version` wire ops
//! stay `501` in the dispatcher — they need an example generator / a cADL source
//! parser (none in the tree yet; ADL2 is OPTIONAL for CNF and untested).

use async_trait::async_trait;
use serde_json::Value;

use ehrbase_sm::SmError;
use ehrbase_sm::{
    DefinitionAdapter, DefinitionAdl2Service, DefinitionAdl14Service, DefinitionQueryService, Page,
    QueryDescriptor,
};
use openehr_flat::{DetailLevel, ExampleType};

use crate::service::EhrbaseService;

// ── ITS-REST Definitions adapter-support (wire-shaped) ───────────────────────
//
// The wire-only rich shapes (template summaries, the example COMPOSITION,
// `StoredQuery` descriptors) the ITS-REST `DEFINITION` group returns, which the
// SM `I_DEFINITION_*` interfaces do not express. All native (`serde_json::Value`
// + `SmError`), so `ehrbase-sm` stays protocol-free. The
// `get_opt`/`get_artefact` retrievals stay on the SM traits below.
#[async_trait]
impl DefinitionAdapter for EhrbaseService {
    async fn template_adl14_upload(&self, opt_xml: String) -> Result<Value, SmError> {
        // The OPT 1.4 canonical XML is parsed + stored (opt14); the wire `201`
        // body is the created template summary.
        Ok(self.store_template(&opt_xml).await?)
    }

    async fn template_adl14_get(&self, template_id: String) -> Result<String, SmError> {
        Ok(self.opt_get_by_template_id(&template_id).await?)
    }

    async fn template_adl14_list(&self) -> Result<Vec<Value>, SmError> {
        Ok(self.list_templates().await?)
    }

    async fn template_adl14_example(
        &self,
        template_id: String,
        detail_level: Option<String>,
        kind: Option<String>,
    ) -> Result<Value, SmError> {
        // `type`/`detail_level` are the dev-OAS `example_type`/`example_detail_level`
        // enums (`definition-validation.openapi.yaml`); an out-of-enum value is a
        // `precondition_violation` (→ `400`). All three detail levels are
        // implemented, so no "unsupported level" fallback applies.
        let level =
            DetailLevel::from_query(detail_level.as_deref()).map_err(SmError::precondition)?;
        let kind = ExampleType::from_query(kind.as_deref()).map_err(SmError::precondition)?;
        Ok(self.template_example(&template_id, level, kind).await?)
    }

    async fn template_adl2_upload(&self, source: String) -> Result<String, SmError> {
        // ADL2 operational-template source (text/plain). Store it and return the
        // stored ARCHETYPE_HRID; the dispatcher builds `Location` + the `Prefer`
        // body from it (201_Template_adl2_upload). Invalid source → 422.
        //
        // Duplicate handling diverges by surface: the REST contract declares
        // `409_template_already_exists` on this endpoint
        // (`definition-codegen.openapi.yaml` /definition/template/adl2 POST),
        // while the SM native `upload_artefact` says "replace it" (SM master04
        // `i_definition_adl2.adoc`). This is the REST adapter seam, so an
        // existing HRID is a 409 here; native SM callers keep replace.
        let meta = crate::service::adl2_validation::validate_adl2_source(&source)
            .map_err(|v| SmError::precondition(format!("{}: {}", v.code, v.detail)))?;
        if self.adl2_exists(&meta.hrid).await? {
            // → ServiceError::Conflict semantics: the wire maps this onto 409
            // (`409_template_already_exists`).
            return Err(SmError::new(
                ehrbase_sm::CallStatusType::CompositionAlreadyExists,
                format!("an ADL2 template with id '{}' already exists", meta.hrid),
            ));
        }
        Ok(self.adl2_upload(&source).await?)
    }

    async fn template_adl2_list(&self) -> Result<Vec<Value>, SmError> {
        // The OAS `filter_template_id`/`concept`/`filter_version`/`offset`/`fetch`
        // filters are not yet applied (no cADL metadata extraction); the full
        // template+OPT list is returned. PORT NOTE on `adl2_template_list`.
        Ok(self.adl2_template_list(Page::all()).await?)
    }

    async fn query_list(&self, qualified_query_name: String) -> Result<Vec<Value>, SmError> {
        Ok(self.list_stored_queries(&qualified_query_name).await?)
    }

    async fn query_version_get(
        &self,
        qualified_query_name: String,
        version: String,
    ) -> Result<Value, SmError> {
        Ok(self
            .get_stored_query(&qualified_query_name, Some(&version))
            .await?)
    }

    async fn query_store(
        &self,
        qualified_query_name: String,
        version: Option<String>,
        body: String,
    ) -> Result<(), SmError> {
        // The effective version is recovered by the dispatcher through the list
        // seam for the `Location` header; the store itself is bodyless.
        self.store_query(&qualified_query_name, version.as_deref(), body)
            .await?;
        Ok(())
    }
}

// ── SM Definitions native API (SM-2) ─────────────────────────────────────────
//
// These realize the SM `I_DEFINITION_ADL14` / `I_DEFINITION_QUERY` interfaces
// (`docs/specs/openehr/SM/docs/UML/classes/{i_definition_adl14,i_definition_query}.adoc`).
// They join `Backend` but drive no ITS-REST route — the DEFINITION wire above is
// unchanged (SM-2 is native-API only). The bodies delegate to the `service::definition`
// logic; `ServiceError` → `ApiError` conversion happens at the `?` boundary.

#[async_trait]
impl DefinitionAdl14Service for EhrbaseService {
    async fn has_archetype(&self, an_id: String) -> Result<bool, SmError> {
        Ok(self.archetype_exists(&an_id).await?)
    }

    async fn valid_archetype(&self, adl: String) -> Result<bool, SmError> {
        Ok(Self::valid_archetype_source(&adl))
    }

    async fn upload_archetype(&self, adl: String) -> Result<(), SmError> {
        Ok(self.archetype_upload(&adl).await?)
    }

    async fn get_archetype(&self, an_id: String) -> Result<String, SmError> {
        Ok(self.archetype_get(&an_id).await?)
    }

    async fn list_archetypes(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.archetype_list(page).await?)
    }

    async fn list_matching_archetypes(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        Ok(self.archetype_list_matching(&id_pattern, page).await?)
    }

    async fn delete_archetype(&self, an_id: String) -> Result<(), SmError> {
        Ok(self.archetype_delete(&an_id).await?)
    }

    async fn archetypes_count(&self) -> Result<i64, SmError> {
        Ok(self.archetype_count().await?)
    }

    async fn has_opt(&self, an_opt_id: String) -> Result<bool, SmError> {
        Ok(self.opt_exists(&an_opt_id).await?)
    }

    async fn valid_opt(&self, opt_xml: String) -> Result<bool, SmError> {
        Ok(Self::valid_opt_xml(&opt_xml))
    }

    async fn upload_opt(&self, opt_xml: String) -> Result<(), SmError> {
        // Delegate to the existing OPT ingestion: parse + structural validation
        // (→ 422 `invalid_template` on failure) and the 409-on-duplicate rule.
        self.store_template(&opt_xml).await?;
        Ok(())
    }

    async fn get_opt(&self, an_opt_id: String) -> Result<String, SmError> {
        Ok(self.opt_get(&an_opt_id).await?)
    }

    async fn list_opts(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.opt_list(page).await?)
    }

    async fn list_matching_opts(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        Ok(self.opt_list_matching(&id_pattern, page).await?)
    }

    async fn delete_opt(&self, an_opt_id: String) -> Result<(), SmError> {
        Ok(self.opt_delete(&an_opt_id).await?)
    }

    async fn opts_count(&self) -> Result<i64, SmError> {
        Ok(self.opt_count().await?)
    }
}

#[async_trait]
impl DefinitionAdl2Service for EhrbaseService {
    async fn has_artefact(&self, an_id: String) -> Result<bool, SmError> {
        Ok(self.adl2_exists(&an_id).await?)
    }

    async fn valid_artefact(&self, adl2: String) -> Result<bool, SmError> {
        Ok(Self::valid_adl2_source(&adl2))
    }

    async fn upload_artefact(&self, adl2: String) -> Result<(), SmError> {
        // Replace-if-exists (same HRID); invalid source → 422 invalid_artefact.
        self.adl2_upload(&adl2).await?;
        Ok(())
    }

    async fn get_artefact(&self, an_id: String) -> Result<String, SmError> {
        Ok(self.adl2_get(&an_id).await?)
    }

    async fn list_artefacts(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list(page).await?)
    }

    async fn list_archetypes(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list_by_kind("archetype", page).await?)
    }

    async fn list_templates(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list_by_kind("template", page).await?)
    }

    async fn list_opts(&self, page: Page) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list_by_kind("operational_template", page).await?)
    }

    async fn list_matching_artefacts(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        Ok(self.adl2_list_matching(&id_pattern, page).await?)
    }

    async fn delete_artefact(&self, an_id: String) -> Result<(), SmError> {
        Ok(self.adl2_delete(&an_id).await?)
    }

    async fn artefacts_count(&self) -> Result<i64, SmError> {
        Ok(self.adl2_count().await?)
    }

    async fn archetypes_count(&self) -> Result<i64, SmError> {
        Ok(self.adl2_count_by_kind("archetype").await?)
    }

    async fn templates_count(&self) -> Result<i64, SmError> {
        Ok(self.adl2_count_by_kind("template").await?)
    }

    async fn opts_count(&self) -> Result<i64, SmError> {
        Ok(self.adl2_count_by_kind("operational_template").await?)
    }
}

#[async_trait]
impl DefinitionQueryService for EhrbaseService {
    async fn has_query(&self, a_query_name: String) -> Result<bool, SmError> {
        Ok(self.query_exists(&a_query_name).await?)
    }

    async fn valid_query(&self, a_query_text: String, a_type: String) -> Result<bool, SmError> {
        Ok(Self::valid_query_source(&a_query_text, &a_type))
    }

    async fn store_query(
        &self,
        a_query_text: String,
        a_type: String,
        a_query_name: Option<String>,
    ) -> Result<QueryDescriptor, SmError> {
        Ok(self
            .query_store_sm(a_query_text, &a_type, a_query_name)
            .await?)
    }

    // `store_query_set` keeps the trait default (`NotImplemented`) — the SM
    // entry is an explicit TODO with no defined semantics.

    async fn list_queries(&self, page: Page) -> Result<Vec<QueryDescriptor>, SmError> {
        Ok(self.query_list(page).await?)
    }

    async fn list_matching_queries(
        &self,
        id_pattern: String,
        artefact_id_pattern: Option<String>,
        page: Page,
    ) -> Result<Vec<QueryDescriptor>, SmError> {
        Ok(self
            .query_list_matching(&id_pattern, artefact_id_pattern.as_deref(), page)
            .await?)
    }

    async fn delete_query(&self, a_query_name: String) -> Result<(), SmError> {
        Ok(self.query_delete(&a_query_name).await?)
    }

    async fn queries_count(&self) -> Result<i64, SmError> {
        Ok(self.query_count().await?)
    }
}
