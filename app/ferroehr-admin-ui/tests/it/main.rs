// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The one admin-console e2e integration binary (`testing.md` §Where tests
//! live): every journey battery is a module here; the shared
//! browser/router/DB fixture lives in `common`.

mod common;

mod e2e_adl2;
mod e2e_admin_ops;
mod e2e_audit;
mod e2e_browse;
mod e2e_chart;
mod e2e_composition;
mod e2e_compositions_filters;
mod e2e_contribution_authoring;
mod e2e_demographics;
mod e2e_directory;
mod e2e_docs_shots;
mod e2e_ehr_ops;
mod e2e_ehr_status;
mod e2e_fhir_admin;
mod e2e_login;
mod e2e_operations;
mod e2e_paging;
mod e2e_scopes;
mod e2e_stored_query_runner;
mod e2e_system;
mod e2e_tags;
mod e2e_tenants;
mod e2e_terminology;
