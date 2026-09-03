// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `SUBJECT_DATA_SET` and `DATA_SET_RESULT` (`subject_data_set.adoc`,
//! `data_set_result.adoc`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::sample::VariableSample;
use super::variable::SubjectVariable;

/// `SUBJECT_DATA_SET` — "Data set relating to a subject as used within an
/// application" (`subject_data_set.adoc`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectDataSet {
    /// `id [1]` — unique identifier of this data set for the subject (usually
    /// an application semantic label, e.g. a guideline id).
    pub id: String,
    /// `subject_id [1]` — identifier of the data subject.
    pub subject_id: String,
    /// `creating_app_id [0..1]` — identifier of the creating/registering
    /// application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creating_app_id: Option<String>,
    /// `using_app_ids: List<String> [0..1]` — applications using this data
    /// set. "May be used to track applications, and dump the data set when
    /// empty" — maintained by the service: the creating app is always a user;
    /// `remove_application_data_set`/`remove_application` retract an app, and
    /// a data set whose user list empties is dropped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub using_app_ids: Vec<String>,
    /// `variables: Hash<String, SUBJECT_VARIABLE> [1]` — the variable set
    /// keyed by *local* name within the data set, which may differ from the
    /// variable's canonical `name` (e.g. `dob` → `date_of_birth`; master10
    /// §Subject Variable Naming).
    pub variables: BTreeMap<String, SubjectVariable>,
}

/// `DATA_SET_RESULT` — "Data set result consisting of full set of variable
/// values extracted from data retrieve frame sources" (`data_set_result.adoc`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataSetResult {
    /// `name [1]` — unique name of this data set.
    pub name: String,
    /// `subject_id [1]` — identifier of the data subject.
    pub subject_id: String,
    /// `variables: List<VARIABLE_SAMPLE> [0..1]` — samples of the variables
    /// in this data set.
    #[serde(default)]
    pub variables: Vec<VariableSample>,
}
