//! The ehrbase-rs Conformance Catalogue (ECC) — our own case numbering
//! (design §3.1, v3.1 ownership inversion).
//!
//! The catalogue is the framework's **primary identity system**: every
//! registered case gets a stable `ECC-<AREA>-<NNN>` id, allocated once in the
//! committed allocation file `inventory/ecc-catalog.tsv` and never reused.
//! The official CNF ids (schedule/robot/OAS/AQL) are **trace references**
//! carried in metadata — the reference oracle, not the key system, because
//! upstream is frozen/unmaintained.
//!
//! Allocation is deterministic: a coverage-guard test asserts every registry
//! entry has a catalogue line; `REGEN_CATALOG=1` appends missing entries in
//! registry order with the next free number per area. Removing a line is
//! forbidden — a retired case flips its status to `retired`, keeping the
//! number burned.

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::path::Path;

use serde::Serialize;

/// The default catalogue file, resolved relative to this crate.
pub const CATALOG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/inventory/ecc-catalog.tsv");

/// A catalogue area — the category axis of the ECC id (design §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Area {
    /// EHR service operations.
    Ehr,
    /// `EHR_STATUS` operations.
    Sta,
    /// COMPOSITION operations.
    Com,
    /// CONTRIBUTION change sets.
    Ctb,
    /// Directory (FOLDER) operations.
    Dir,
    /// Template / OPT provisioning (DEFINITION ADL 1.4).
    Tpl,
    /// Stored-query provisioning (DEFINITION QUERY).
    Sqr,
    /// AQL execution (QUERY service).
    Qry,
    /// Content / archetype validation (the master15/16/17 ground + fills).
    Val,
    /// ITS-REST operation × status matrix cases.
    Rest,
    /// Demographic service.
    Dem,
    /// Admin service.
    Adm,
    /// Security / authorization.
    Sec,
    /// Version signing.
    Sig,
    /// Messaging (EHR Extract / TDS), when implemented.
    Msg,
    /// Terminology-server integration (AQL TERMINOLOGY family + FHIR-tx).
    Ts,
}

impl Area {
    /// Every area, in catalogue order.
    pub const ALL: [Area; 16] = [
        Area::Ehr,
        Area::Sta,
        Area::Com,
        Area::Ctb,
        Area::Dir,
        Area::Tpl,
        Area::Sqr,
        Area::Qry,
        Area::Val,
        Area::Rest,
        Area::Dem,
        Area::Adm,
        Area::Sec,
        Area::Sig,
        Area::Msg,
        Area::Ts,
    ];

    /// The id segment (`EHR`, `STA`, …).
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Area::Ehr => "EHR",
            Area::Sta => "STA",
            Area::Com => "COM",
            Area::Ctb => "CTB",
            Area::Dir => "DIR",
            Area::Tpl => "TPL",
            Area::Sqr => "SQR",
            Area::Qry => "QRY",
            Area::Val => "VAL",
            Area::Rest => "REST",
            Area::Dem => "DEM",
            Area::Adm => "ADM",
            Area::Sec => "SEC",
            Area::Sig => "SIG",
            Area::Msg => "MSG",
            Area::Ts => "TS",
        }
    }

    /// The human title of the area (the per-category headline in `CATALOG.md`).
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Area::Ehr => "EHR service",
            Area::Sta => "EHR_STATUS",
            Area::Com => "COMPOSITION",
            Area::Ctb => "CONTRIBUTION (change sets)",
            Area::Dir => "DIRECTORY (FOLDER)",
            Area::Tpl => "Template / OPT provisioning",
            Area::Sqr => "Stored-query provisioning",
            Area::Qry => "AQL execution",
            Area::Val => "Content / archetype validation",
            Area::Rest => "ITS-REST operation×status matrix",
            Area::Dem => "Demographic service",
            Area::Adm => "Admin service",
            Area::Sec => "Security / authorization",
            Area::Sig => "Version signing",
            Area::Msg => "Messaging",
            Area::Ts => "Terminology-server integration",
        }
    }

    /// Parse an area tag (`"EHR"` → [`Area::Ehr`]).
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Area> {
        Area::ALL.into_iter().find(|a| a.tag() == tag)
    }
}

/// The lifecycle status of a catalogue line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EccStatus {
    /// The case is registered and runnable.
    Active,
    /// The case was removed from the registry; the number stays burned.
    Retired,
    /// The number is allocated for planned work (transcription backlog).
    Planned,
}

impl EccStatus {
    fn as_str(self) -> &'static str {
        match self {
            EccStatus::Active => "active",
            EccStatus::Retired => "retired",
            EccStatus::Planned => "planned",
        }
    }

    fn parse(s: &str) -> Option<EccStatus> {
        match s {
            "active" => Some(EccStatus::Active),
            "retired" => Some(EccStatus::Retired),
            "planned" => Some(EccStatus::Planned),
            _ => None,
        }
    }
}

/// One catalogue line: an allocated ECC number.
#[derive(Debug, Clone, Serialize)]
pub struct EccEntry {
    /// The full id, e.g. `"ECC-EHR-003"`.
    pub ecc_id: String,
    /// The area.
    pub area: Area,
    /// The allocated number within the area.
    pub number: u32,
    /// Lifecycle status.
    pub status: EccStatus,
    /// The primary reference key this number is bound to — the registry's
    /// registration key (an official CNF id for traced cases, an `own:*` key
    /// for catalogue-original cases). One number per key, forever.
    pub primary_ref: String,
    /// The human title.
    pub title: String,
}

/// A catalogue-format or consistency error.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// The catalogue file could not be read or written.
    #[error("catalog io at {path}: {source}")]
    Io {
        /// The file path.
        path: String,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A malformed line.
    #[error("catalog line {line} is malformed: {reason}")]
    Malformed {
        /// 1-based line number.
        line: usize,
        /// What is wrong.
        reason: String,
    },
    /// A duplicate id or primary reference.
    #[error("catalog duplicate: {what}")]
    Duplicate {
        /// The duplicated key.
        what: String,
    },
}

/// The loaded catalogue: allocation state + lookups.
#[derive(Debug, Default)]
pub struct Catalog {
    entries: Vec<EccEntry>,
    by_ref: BTreeMap<String, usize>,
    next_number: BTreeMap<Area, u32>,
}

impl Catalog {
    /// Load the committed catalogue from [`CATALOG_PATH`]. A missing file is
    /// an empty catalogue (the pre-allocation state).
    ///
    /// # Errors
    /// [`CatalogError`] on unreadable or malformed content.
    pub fn load_default() -> Result<Catalog, CatalogError> {
        Catalog::load(Path::new(CATALOG_PATH))
    }

    /// Load a catalogue from `path`.
    ///
    /// # Errors
    /// [`CatalogError`] on unreadable or malformed content.
    pub fn load(path: &Path) -> Result<Catalog, CatalogError> {
        let mut catalog = Catalog::default();
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(catalog),
            Err(source) => {
                return Err(CatalogError::Io {
                    path: path.display().to_string(),
                    source,
                });
            }
        };
        for (idx, line) in text.lines().enumerate() {
            let line_no = idx + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let [ecc_id, area_tag, status, primary_ref, title] = fields[..] else {
                return Err(CatalogError::Malformed {
                    line: line_no,
                    reason: format!("expected 5 tab-separated fields, got {}", fields.len()),
                });
            };
            let area = Area::from_tag(area_tag).ok_or_else(|| CatalogError::Malformed {
                line: line_no,
                reason: format!("unknown area {area_tag:?}"),
            })?;
            let status = EccStatus::parse(status).ok_or_else(|| CatalogError::Malformed {
                line: line_no,
                reason: format!("unknown status {status:?}"),
            })?;
            let number: u32 = ecc_id
                .strip_prefix(&format!("ECC-{}-", area.tag()))
                .and_then(|n| n.parse().ok())
                .ok_or_else(|| CatalogError::Malformed {
                    line: line_no,
                    reason: format!("id {ecc_id:?} does not match ECC-{}-<NNN>", area.tag()),
                })?;
            catalog.insert(EccEntry {
                ecc_id: ecc_id.to_owned(),
                area,
                number,
                status,
                primary_ref: primary_ref.to_owned(),
                title: title.to_owned(),
            })?;
        }
        Ok(catalog)
    }

    fn insert(&mut self, entry: EccEntry) -> Result<(), CatalogError> {
        if self.by_ref.contains_key(&entry.primary_ref) {
            return Err(CatalogError::Duplicate {
                what: format!("primary_ref {}", entry.primary_ref),
            });
        }
        if self.entries.iter().any(|e| e.ecc_id == entry.ecc_id) {
            return Err(CatalogError::Duplicate {
                what: format!("ecc_id {}", entry.ecc_id),
            });
        }
        let next = self.next_number.entry(entry.area).or_insert(1);
        *next = (*next).max(entry.number + 1);
        self.by_ref
            .insert(entry.primary_ref.clone(), self.entries.len());
        self.entries.push(entry);
        Ok(())
    }

    /// The entry bound to a primary reference key, if allocated.
    #[must_use]
    pub fn by_primary_ref(&self, primary_ref: &str) -> Option<&EccEntry> {
        self.by_ref.get(primary_ref).map(|&i| &self.entries[i])
    }

    /// All entries, in file order.
    #[must_use]
    pub fn entries(&self) -> &[EccEntry] {
        &self.entries
    }

    /// Allocate the next free number in `area` for `primary_ref` and append
    /// the entry (in-memory; call [`Catalog::save`] to persist).
    ///
    /// # Errors
    /// [`CatalogError::Duplicate`] if the reference is already bound.
    pub fn allocate(
        &mut self,
        area: Area,
        primary_ref: &str,
        title: &str,
    ) -> Result<&EccEntry, CatalogError> {
        let number = *self.next_number.entry(area).or_insert(1);
        let entry = EccEntry {
            ecc_id: format!("ECC-{}-{number:03}", area.tag()),
            area,
            number,
            status: EccStatus::Active,
            primary_ref: primary_ref.to_owned(),
            title: title.to_owned(),
        };
        self.insert(entry)?;
        Ok(self
            .entries
            .last()
            .unwrap_or_else(|| unreachable!("just pushed")))
    }

    /// Serialize to the TSV format.
    #[must_use]
    pub fn to_tsv(&self) -> String {
        let mut out = String::from(
            "# The ehrbase-rs Conformance Catalogue (ECC) — allocated case numbers.\n\
             # ecc_id<TAB>area<TAB>status<TAB>primary_ref<TAB>title\n\
             # Numbers are allocated once and NEVER reused; retire, don't delete.\n\
             # Regenerate additions: REGEN_CATALOG=1 cargo test -p conformance --test coverage\n",
        );
        for e in &self.entries {
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}",
                e.ecc_id,
                e.area.tag(),
                e.status.as_str(),
                e.primary_ref,
                e.title
            );
        }
        out
    }

    /// Persist to `path`.
    ///
    /// # Errors
    /// [`CatalogError::Io`] on write failure.
    pub fn save(&self, path: &Path) -> Result<(), CatalogError> {
        std::fs::write(path, self.to_tsv()).map_err(|source| CatalogError::Io {
            path: path.display().to_string(),
            source,
        })
    }
}

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_is_sequential_per_area_and_refs_unique() {
        let mut c = Catalog::default();
        let id1 = c
            .allocate(Area::Ehr, "sched:a", "A")
            .expect("alloc")
            .ecc_id
            .clone();
        let id2 = c
            .allocate(Area::Ehr, "sched:b", "B")
            .expect("alloc")
            .ecc_id
            .clone();
        let id3 = c
            .allocate(Area::Val, "sched:c", "C")
            .expect("alloc")
            .ecc_id
            .clone();
        assert_eq!(id1, "ECC-EHR-001");
        assert_eq!(id2, "ECC-EHR-002");
        assert_eq!(id3, "ECC-VAL-001");
        assert!(c.allocate(Area::Ehr, "sched:a", "dup").is_err());
    }

    #[test]
    fn tsv_round_trips() {
        let mut c = Catalog::default();
        c.allocate(Area::Sig, "own:sign-digest", "Digest present")
            .expect("alloc");
        c.allocate(Area::Qry, "aql:B/102@loaded_db", "AQL B/102")
            .expect("alloc");
        let tsv = c.to_tsv();
        let dir = std::env::temp_dir().join("ecc-catalog-test");
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let path = dir.join("catalog.tsv");
        std::fs::write(&path, &tsv).expect("write");
        let loaded = Catalog::load(&path).expect("load");
        assert_eq!(loaded.entries().len(), 2);
        assert_eq!(
            loaded
                .by_primary_ref("own:sign-digest")
                .map(|e| e.ecc_id.as_str()),
            Some("ECC-SIG-001")
        );
        // Continuing allocation after load resumes after the max number.
        let mut loaded = loaded;
        let next = loaded
            .allocate(Area::Sig, "own:sign-pgp", "PGP")
            .expect("alloc");
        assert_eq!(next.ecc_id, "ECC-SIG-002");
    }
}
