// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

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
/// [`TerminologyDecodeError::Malformed`] when the body is not a valid R4B
/// `Parameters` resource.
pub fn decode_parameters(body: &[u8]) -> Result<ParametersView, TerminologyDecodeError> {
    let parameters: Parameters = serde_json::from_slice(body)?;
    let mut view = ParametersView::default();
    for parameter in parameters.parameter.iter().flatten() {
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

/// Decodes a FHIR `ValueSet` `$expand` response into its expansion members.
///
/// # Errors
///
/// [`TerminologyDecodeError::Malformed`] when the body is not a valid R4B
/// `ValueSet` resource.
pub fn decode_expansion(body: &[u8]) -> Result<Vec<ExpansionMember>, TerminologyDecodeError> {
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
