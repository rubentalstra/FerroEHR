// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Typed decoding of the FHIR R4B terminology-operation responses.
//!
//! The platform's terminology provider owns the HTTP client, the routing, and
//! the SM error mapping; this module owns the FHIR half: it parses a
//! `Parameters` (`$validate-code` / `$subsumes` / `$lookup`) or a `ValueSet`
//! (`$expand`) response into the typed
//! [`fhir_model`] R4B resources and reduces it to the small view the service
//! consumes.
//!
//! **No openEHR spec governs FHIR resource representation — our own
//! design/extension.** Decoding is strict: a response that is not a valid
//! R4B resource is refused rather than partially read
//! (<https://docs.rs/fhir-model/0.13.0/fhir_model/r4b/resources/>).

use std::collections::BTreeMap;

use fhir_model::r4b::resources::{
    Parameters, ParametersParameterValue, ValueSet, ValueSetExpansionContains,
};

/// Why a FHIR terminology response could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum TerminologyDecodeError {
    /// The body is not a valid R4B resource of the expected type.
    #[error("malformed FHIR response: {0}")]
    Malformed(#[from] serde_json::Error),
    /// The body is a well-formed resource of another type. The concrete R4B
    /// structs carry no `resourceType` discriminator of their own (only the
    /// `Resource` enum does), and `Parameters` has no mandatory member, so
    /// without this check any JSON object would read as an empty `Parameters`.
    #[error("unexpected FHIR resource: expected {expected}, got {found}")]
    WrongResource {
        /// The resource type the operation answers with.
        expected: &'static str,
        /// The `resourceType` the body declares (`(none)` when it declares none).
        found: String,
    },
}

/// The `resourceType` a body declares, checked before the typed decode.
#[derive(serde::Deserialize)]
struct ResourceHeader {
    #[serde(rename = "resourceType")]
    resource_type: Option<String>,
}

/// Refuses a body whose `resourceType` is not `expected`.
fn expect_resource(body: &[u8], expected: &'static str) -> Result<(), TerminologyDecodeError> {
    let header: ResourceHeader = serde_json::from_slice(body)?;
    match header.resource_type {
        Some(found) if found == expected => Ok(()),
        found => Err(TerminologyDecodeError::WrongResource {
            expected,
            found: found.unwrap_or_else(|| "(none)".to_owned()),
        }),
    }
}

/// The named scalar values a `Parameters` response carries, by parameter name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParametersView {
    /// `valueBoolean` parameters (e.g. `$validate-code` `result`).
    pub booleans: BTreeMap<String, bool>,
    /// `valueCode` parameters (e.g. `$subsumes` `outcome`).
    pub codes: BTreeMap<String, String>,
    /// `valueString` parameters (e.g. `$lookup` `display`).
    pub strings: BTreeMap<String, String>,
    /// `ConceptMap/$translate` `match` parameters, in response order.
    pub matches: Vec<TranslateMatch>,
}

/// One `match` of a `ConceptMap/$translate` response: the `equivalence` code
/// plus the target `concept` Coding's members
/// (<https://hl7.org/fhir/R4B/conceptmap-operation-translate.html>).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranslateMatch {
    /// The match's `equivalence` (`equivalent`, `equal`, `wider`, …).
    pub equivalence: Option<String>,
    /// The target concept's code system URL.
    pub system: Option<String>,
    /// The target concept's code.
    pub code: Option<String>,
    /// The target concept's display text.
    pub display: Option<String>,
}

/// One member of a `ValueSet.expansion`, with its nested members.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpansionMember {
    /// The member's code.
    pub code: Option<String>,
    /// The member's display text.
    pub display: Option<String>,
    /// The members nested under this one.
    pub children: Vec<ExpansionMember>,
}

/// Decodes a FHIR `Parameters` response into its named scalar values.
///
/// # Errors
///
/// [`TerminologyDecodeError::WrongResource`] when the body declares another
/// `resourceType`; [`TerminologyDecodeError::Malformed`] when it is not a
/// valid R4B `Parameters` resource.
pub fn decode_parameters(body: &[u8]) -> Result<ParametersView, TerminologyDecodeError> {
    expect_resource(body, "Parameters")?;
    let parameters: Parameters = serde_json::from_slice(body)?;
    let mut view = ParametersView::default();
    for parameter in parameters.parameter.iter().flatten() {
        if parameter.name == "match" {
            view.matches.push(translate_match(parameter));
            continue;
        }
        match &parameter.value {
            Some(ParametersParameterValue::Boolean(value)) => {
                view.booleans.insert(parameter.name.clone(), *value);
            }
            Some(ParametersParameterValue::Code(value)) => {
                view.codes.insert(parameter.name.clone(), value.clone());
            }
            Some(ParametersParameterValue::String(value)) => {
                view.strings.insert(parameter.name.clone(), value.clone());
            }
            _ => {}
        }
    }
    Ok(view)
}

/// Reads one `$translate` `match` parameter's parts (`equivalence` +
/// `concept`).
fn translate_match(parameter: &fhir_model::r4b::resources::ParametersParameter) -> TranslateMatch {
    let mut m = TranslateMatch::default();
    for part in parameter.part.iter().flatten() {
        match (part.name.as_str(), &part.value) {
            ("equivalence", Some(ParametersParameterValue::Code(value))) => {
                m.equivalence = Some(value.clone());
            }
            ("concept", Some(ParametersParameterValue::Coding(coding))) => {
                m.system.clone_from(&coding.0.system);
                m.code.clone_from(&coding.0.code);
                m.display.clone_from(&coding.0.display);
            }
            _ => {}
        }
    }
    m
}

/// Decodes a FHIR `ValueSet` `$expand` response into its expansion members.
///
/// # Errors
///
/// [`TerminologyDecodeError::WrongResource`] when the body declares another
/// `resourceType`; [`TerminologyDecodeError::Malformed`] when it is not a
/// valid R4B `ValueSet` resource.
pub fn decode_expansion(body: &[u8]) -> Result<Vec<ExpansionMember>, TerminologyDecodeError> {
    expect_resource(body, "ValueSet")?;
    let value_set: ValueSet = serde_json::from_slice(body)?;
    Ok(value_set
        .expansion
        .as_ref()
        .map(|expansion| expansion.contains.iter().flatten().map(member).collect())
        .unwrap_or_default())
}

/// One expansion member and its nested members.
fn member(contains: &ValueSetExpansionContains) -> ExpansionMember {
    ExpansionMember {
        code: contains.code.clone(),
        display: contains.display.clone(),
        children: contains.contains.iter().flatten().map(member).collect(),
    }
}
