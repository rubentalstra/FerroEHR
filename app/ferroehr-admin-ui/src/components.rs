// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Shared UI components used by several screens — the design-system kit.
//!
//! Screens compose these instead of hand-rolling markup: `PageHeader` opens
//! every route, `table_shell` renders every listing, the `field` constants
//! style every control, `StatCard`/`EmptyState` cover metrics and voids,
//! `results_chart` draws every AQL result set, `activity_chart` draws every
//! events-per-day timeline, `scope_grants` renders every SMART scope string,
//! `item_tags` renders every `ITEM_TAG` collection, and `toast` reports every
//! mutation outcome. `facts`, `notice`, `tab_bar`, `logical_delete` and
//! `version_history` carry the shapes the detail screens are assembled from: a
//! facts line, the inline notices, one tab pill, the logical-delete affordance,
//! and a versioned object's whole History tab. `wire` holds the two flattened
//! CDR shapes the kit itself renders, so no component reaches into a screen for
//! a type.

pub mod activity_chart;
pub mod brand;
pub mod confirm_dialog;
pub mod data_table;
pub mod empty_state;
pub mod facts;
pub mod field;
pub mod format_view;
pub mod item_tags;
pub mod logical_delete;
pub mod notice;
pub mod page_header;
pub mod results_chart;
pub mod scope_grants;
pub mod stat_card;
pub mod surface;
pub mod tab_bar;
pub mod toast;
pub mod version_history;
pub mod wire;
