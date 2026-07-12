//! The SM Admin service (`master15-admin_service.adoc`): `I_ADMIN_SERVICE` /
//! `I_ADMIN_ARCHIVE` / `I_ADMIN_DUMP_LOAD` plus `EXPORT_SPEC` and
//! `DUMP_LOAD_FAIL_REPORT`.

pub mod service;

pub use service::{
    AdminArchive, AdminDumpLoad, AdminService, CompressionFormat, DumpLoadFailReport, ExportFormat,
    ExportSpec, StatTimeRange,
};
