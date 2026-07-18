//! Shared UI components used by several screens — the design-system kit.
//! Screens compose these instead of hand-rolling markup: `PageHeader`
//! opens every route, `table_shell` renders every listing, the `field`
//! constants style every control, `StatCard`/`EmptyState` cover metrics
//! and voids, and `toast` reports every mutation outcome.

pub mod brand;
pub mod data_table;
pub mod empty_state;
pub mod field;
pub mod format_view;
pub mod page_header;
pub mod stat_card;
pub mod surface;
pub mod toast;
