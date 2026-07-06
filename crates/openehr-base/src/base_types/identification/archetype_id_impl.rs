//! Hand-written accessor functions (ADR-003) for `ARCHETYPE_ID`.
//!
//! Spec: BASE 1.3.0
//! `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.archetype_id.adoc`.
//! Lexical form:
//! `rm_originator '-' rm_name '-' rm_entity '.' concept_name {'-' specialisation}* '.v' version_id`
//! e.g. `openEHR-EHR-OBSERVATION.blood_pressure-cuff.v1`.

use super::archetype_id::ArchetypeId;
use super::lexical::IdError;
use std::str::FromStr;

/// Decomposition of an archetype-id value into `qualified_rm_entity`,
/// `domain_concept`, and `version_id` slices.
struct Parts<'a> {
    qualified: &'a str,
    domain: &'a str,
    version: &'a str,
}

/// Split an archetype-id string on the trailing `.vN` and the first `.` after
/// the RM qualifier. Returns `None` if either separator is absent.
fn parse(value: &str) -> Option<Parts<'_>> {
    let (head, version) = value.rsplit_once(".v")?;
    let (qualified, domain) = head.split_once('.')?;
    if qualified.is_empty() || domain.is_empty() || version.is_empty() {
        return None;
    }
    Some(Parts {
        qualified,
        domain,
        version,
    })
}

impl ArchetypeId {
    /// Globally-qualified RM entity, e.g. `openEHR-EHR-OBSERVATION` — the part
    /// before the first `.` (BASE `qualified_rm_entity`).
    #[must_use]
    pub fn qualified_rm_entity(&self) -> &str {
        parse(&self.value).map_or("", |p| p.qualified)
    }

    /// Concept name including any specialisations, e.g. `blood_pressure-cuff` —
    /// the part between the first `.` and the trailing `.vN` (BASE
    /// `domain_concept`).
    #[must_use]
    pub fn domain_concept(&self) -> &str {
        parse(&self.value).map_or("", |p| p.domain)
    }

    /// Organisation originating the RM, e.g. `openEHR` (BASE `rm_originator`).
    #[must_use]
    pub fn rm_originator(&self) -> &str {
        self.qualified_rm_entity().split('-').next().unwrap_or("")
    }

    /// Reference-model name, e.g. `EHR` (BASE `rm_name`).
    #[must_use]
    pub fn rm_name(&self) -> &str {
        self.qualified_rm_entity().split('-').nth(1).unwrap_or("")
    }

    /// Ontological RM entity, e.g. `OBSERVATION` (BASE `rm_entity`) — the third
    /// and remaining `-`-delimited segments of the RM qualifier.
    #[must_use]
    pub fn rm_entity(&self) -> &str {
        // rm_originator '-' rm_name '-' rm_entity: take everything after the
        // second '-'.
        let q = self.qualified_rm_entity();
        match q.match_indices('-').nth(1) {
            Some((i, _)) => &q[i + 1..],
            None => "",
        }
    }

    /// Name of the concept specialisation, if any, e.g. `cuff` for
    /// `blood_pressure-cuff` — the last `-`-delimited segment of the domain
    /// concept when it is specialised, else the empty string (BASE
    /// `specialisation`).
    #[must_use]
    pub fn specialisation(&self) -> &str {
        let d = self.domain_concept();
        match d.rsplit_once('-') {
            Some((_, last)) => last,
            None => "",
        }
    }

    /// Major version identifier, e.g. `1` — the part after the trailing `.v`
    /// (BASE `version_id`).
    #[must_use]
    pub fn version_id(&self) -> &str {
        parse(&self.value).map_or("", |p| p.version)
    }
}

impl FromStr for ArchetypeId {
    type Err = IdError;

    /// Parse an `ARCHETYPE_ID`, requiring the RM qualifier, a domain concept,
    /// and a `.vN` version segment.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IdError::Empty);
        }
        let Some(p) = parse(s) else {
            return Err(IdError::Archetype(s.to_owned()));
        };
        // The RM qualifier must have three '-'-delimited segments.
        if p.qualified.split('-').count() < 3 {
            return Err(IdError::Archetype(s.to_owned()));
        }
        Ok(Self {
            value: s.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aid(v: &str) -> ArchetypeId {
        ArchetypeId {
            value: v.to_owned(),
        }
    }

    #[test]
    fn decomposes_specialised() {
        let a = aid("openEHR-EHR-OBSERVATION.blood_pressure-cuff.v1");
        assert_eq!(a.qualified_rm_entity(), "openEHR-EHR-OBSERVATION");
        assert_eq!(a.domain_concept(), "blood_pressure-cuff");
        assert_eq!(a.rm_originator(), "openEHR");
        assert_eq!(a.rm_name(), "EHR");
        assert_eq!(a.rm_entity(), "OBSERVATION");
        assert_eq!(a.specialisation(), "cuff");
        assert_eq!(a.version_id(), "1");
    }

    #[test]
    fn decomposes_unspecialised() {
        let a = aid("openEHR-EHR-SECTION.vital_signs.v2");
        assert_eq!(a.domain_concept(), "vital_signs");
        assert_eq!(a.specialisation(), "");
        assert_eq!(a.version_id(), "2");
        assert_eq!(a.rm_entity(), "SECTION");
    }

    #[test]
    fn from_str_strict() {
        assert!(
            "openEHR-EHR-OBSERVATION.x.v1"
                .parse::<ArchetypeId>()
                .is_ok()
        );
        assert!(matches!(
            "not-an-archetype".parse::<ArchetypeId>(),
            Err(IdError::Archetype(_))
        ));
        assert!(matches!(
            "openEHR-EHR.concept.v1".parse::<ArchetypeId>(),
            Err(IdError::Archetype(_))
        ));
        assert_eq!("".parse::<ArchetypeId>(), Err(IdError::Empty));
    }
}
