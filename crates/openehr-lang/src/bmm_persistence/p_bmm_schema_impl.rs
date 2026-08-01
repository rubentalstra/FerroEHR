//! Hand-written spec functions of `P_BMM_SCHEMA` — the derived schema id it
//! inherits from `BMM_SCHEMA_CORE`.
//!
//! Spec: `LANG/docs/UML/classes/org.openehr.lang.bmm_persistence.p_bmm_schema.adoc`
//! §Inherit (`BMM_SCHEMA_CORE`) and
//! `…org.openehr.lang.bmm.bmm_schema_core.adoc` §Functions (`schema_id`:
//! "Derived name of schema, based on model publisher, model name, model
//! release"), with the rendering pinned by
//! `LANG/docs/bmm_persistence/master04-syntax.adoc` §Header Items
//! ("schema_id computed as `<rm_publisher>_<schema_name>_<rm_release>`").

use crate::bmm_persistence::p_bmm_schema::PBmmSchema;

/// Renders a schema id from its three parts, lower-cased.
///
/// `LANG/docs/bmm_persistence/master04-syntax.adoc` §Header Items states the
/// composition verbatim ("schema_id computed as
/// `<rm_publisher>_<schema_name>_<rm_release>`"); the lower-casing is pinned by
/// the v3 counterpart `org.openehr.lang.bmm3.bmm_model.adoc` §Functions
/// (`model_id`: "Identifier of this model, lower-case, formed from:
/// `<rm_publisher>_<model_name>_<rm_release>`. E.g. `"openehr_ehr_1.0.4"`"),
/// the same adjudication
/// [`crate::bmm::core::bmm_model::BmmModel::schema_id`] records.
pub(crate) fn compose_schema_id(rm_publisher: &str, schema_name: &str, rm_release: &str) -> String {
    format!("{rm_publisher}_{schema_name}_{rm_release}").to_lowercase()
}

impl PBmmSchema {
    /// `BMM_SCHEMA_CORE.schema_id`: "Derived name of schema, based on model
    /// publisher, model name, model release"
    /// (`org.openehr.lang.bmm.bmm_schema_core.adoc` §Functions).
    ///
    /// Rendered `<rm_publisher>_<schema_name>_<rm_release>`, lower-cased — the
    /// form `master04-syntax.adoc` §Header Items states and the form the
    /// `_includes_` blocks of the vendored `.bmm` schemas reference.
    #[must_use]
    pub fn schema_id(&self) -> String {
        compose_schema_id(&self.rm_publisher, &self.schema_name, &self.rm_release)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::bmm_persistence::p_bmm_schema::PBmmSchema;

    /// A header-only schema with the three identifying parts set.
    fn schema(rm_publisher: &str, schema_name: &str, rm_release: &str) -> PBmmSchema {
        PBmmSchema {
            packages: BTreeMap::new(),
            rm_publisher: rm_publisher.to_owned(),
            rm_release: rm_release.to_owned(),
            schema_name: schema_name.to_owned(),
            schema_revision: String::new(),
            schema_lifecycle_state: String::new(),
            schema_author: String::new(),
            schema_description: String::new(),
            schema_contributors: Vec::new(),
            archetype_parent_class: None,
            archetype_data_value_parent_class: None,
            archetype_rm_closure_packages: Vec::new(),
            archetype_visualise_descendants_of: None,
            bmm_version: "2.4".to_owned(),
            includes: None,
            primitive_types: Vec::new(),
            class_definitions: Vec::new(),
        }
    }

    #[test]
    fn schema_id_is_the_lower_cased_publisher_name_release() {
        assert_eq!(
            schema("openehr", "primitive_types", "1.0.2").schema_id(),
            "openehr_primitive_types_1.0.2"
        );
        // The CIMI schemas write the parts in upper case and reference each
        // other by the lower-cased id (`tests/vendor/bmm/cimi/`).
        assert_eq!(
            schema("CIMI", "RM_CORE", "0.0.2").schema_id(),
            "cimi_rm_core_0.0.2"
        );
    }
}
