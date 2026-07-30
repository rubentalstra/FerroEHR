//! The outer ADL2 artefact parser (phase A2).
//!
//! Transcribed from the vendored `adl2.g4` (at
//! `crates/openehr-adl/vendor/grammar/`) plus the spec-text section extensions
//! the pinned grammar lacks. It produces a [`SourceArtefact`]: the artefact
//! kind, identification meta + HRID, the specialise parent reference, each
//! ODIN section parsed via `openehr_lang::odin`, and the cADL `definition` /
//! `rules` bodies captured as **raw spans** (cADL parsing is a separate pass,
//! `crate::cadl`).
//!
//! Section boundaries follow the grammar's `'\n'`-anchoring of the section
//! keywords (`adl_keywords.g4`): a section header is a keyword at column 0
//! (its preceding byte is a newline), so an identical word appearing indented
//! inside an ODIN section or the definition is never mistaken for a header.
//! Because a multi-line `STRING` is one lexer token, a section keyword inside a
//! quoted value can never read as a header either.

use openehr_am::am24::aom2::archetype::archetype_hrid::ArchetypeHrid;
use openehr_base::prelude::VersionStatus;

use crate::error::{SyntaxError, SyntaxErrorCode};
use crate::lexer::{Spanned, Token};

/// The artefact kind (first keyword of an ADL2 source).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtefactKind {
    /// `archetype`.
    Archetype,
    /// `template`.
    Template,
    /// `template_overlay`.
    TemplateOverlay,
    /// `operational_template` (also `operational_archetype`, per the
    /// `master07.04` keyword inconsistency).
    OperationalTemplate,
}

/// A raw, unparsed section body: its byte range in the source and its token
/// index range in the whole-file token stream (for a later cADL/rules phase).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSpan {
    /// Byte range of the body in the original source.
    pub bytes: std::ops::Range<usize>,
    /// Token index range of the body in the whole-file token stream.
    pub tokens: std::ops::Range<usize>,
}

/// Identification meta-data (`adl2.g4` `meta_data`; `master07.05`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtefactMeta {
    /// `adl_version=` (mandatory in a well-formed artefact).
    pub adl_version: Option<String>,
    /// `rm_release=` (mandatory in a well-formed artefact).
    pub rm_release: Option<String>,
    /// `uid=` (a GUID or an OID).
    pub uid: Option<String>,
    /// `build_uid=` (a GUID).
    pub build_uid: Option<String>,
    /// `provenance_id=`.
    pub provenance_id: Option<String>,
    /// `controlled` ⇒ `Some(true)`, `uncontrolled` ⇒ `Some(false)`.
    pub controlled: Option<bool>,
    /// The `generated` flag.
    pub generated: bool,
    /// Any other (unknown) meta items, preserved verbatim as `(key, value?)`.
    pub other: Vec<(String, Option<String>)>,
}

/// A parsed ADL2 source artefact (outer structure only; the definition/rules
/// bodies stay raw here, parsed on demand by `crate::cadl` /
/// `openehr_lang::odin`).
#[derive(Debug, Clone, PartialEq)]
pub struct SourceArtefact {
    /// The artefact kind.
    pub kind: ArtefactKind,
    /// Identification meta-data.
    pub meta: ArtefactMeta,
    /// The parsed HRID.
    pub hrid: ArchetypeHrid,
    /// The `specialise`/`specialize` parent reference, if any.
    pub parent_ref: Option<ArchetypeHrid>,
    /// The `language` section (ODIN).
    pub language: Option<openehr_lang::odin::OdinValue>,
    /// The `description` section (ODIN).
    pub description: Option<openehr_lang::odin::OdinValue>,
    /// The `terminology` section (ODIN).
    pub terminology: Option<openehr_lang::odin::OdinValue>,
    /// The `annotations` section (ODIN).
    pub annotations: Option<openehr_lang::odin::OdinValue>,
    /// The `rm_overlay` section (ODIN; `master07.12`).
    pub rm_overlay: Option<openehr_lang::odin::OdinValue>,
    /// The `component_terminologies` section (ODIN; OPT only).
    pub component_terminologies: Option<openehr_lang::odin::OdinValue>,
    /// The ADL **1.4** `revision_history` section (ODIN;
    /// `AM/docs/ADL1.4/master08-adl` §Revision History Section: "The revision
    /// history section of an archetype shows the audit history of changes to
    /// the archetype, and is expressed in dADL syntax. It is optional, and is
    /// included at the end of the archetype").
    ///
    /// NOTE: it has no landing site in the assembled AOM2 model, and that is
    /// deliberate upstream, not a generated-model gap —
    /// `AM/docs/ADL2/master01-preface` §Changes from ADL 1.4: "the
    /// `revision_history` section is removed, since the AOM2 uses the openEHR
    /// Base Types version of the Resource package" (SPECAM-61 in that
    /// specification's amendment record, and the mirroring "Remove
    /// `revision_history` property" entry in the BASE resource amendment
    /// record). So the section is read and preserved *here*, at the 1.4 source
    /// level, where a caller that wants the audit history can reach it, and it
    /// is not carried into the ADL2 artefact that
    /// [`crate::adl14::convert`] produces.
    pub revision_history: Option<openehr_lang::odin::OdinValue>,
    /// The `definition` (cADL) body as a raw span.
    pub definition: Option<RawSpan>,
    /// The `rules`/`invariant` body as a raw span.
    pub rules: Option<RawSpan>,
    /// Nested `template_overlay` artefacts (a `template` may carry many).
    pub overlays: Vec<SourceArtefact>,
}

/// Parse an ADL2 source into a [`SourceArtefact`].
///
/// ODIN sections are delegated to `openehr_lang::odin::parse`; the `definition`
/// and `rules` bodies are captured as [`RawSpan`]s. Recoverable errors are
/// collected; the whole error list is returned on any failure.
///
/// # Errors
/// Returns every [`SyntaxError`] found (lexical, identification, ODIN-section,
/// or missing-required-section). ODIN parse failures surface as
/// [`SyntaxErrorCode::Sdinv`] carrying the section name.
pub fn parse_source(src: &str) -> Result<SourceArtefact, Vec<SyntaxError>> {
    let toks = match crate::lexer::lex(src) {
        Ok(t) => t,
        Err(e) => return Err(vec![e]),
    };
    let mut outer = Outer {
        src,
        toks: &toks,
        pos: 0,
        errors: Vec::new(),
    };
    let artefact = outer.parse_artefact(false);
    if outer.pos < outer.toks.len() && outer.errors.is_empty() {
        // Trailing tokens after a complete artefact.
        let span = outer.toks[outer.pos].span.clone();
        outer.push(SyntaxErrorCode::Sunk, "unexpected trailing input", span);
    }
    match artefact {
        Some(a) if outer.errors.is_empty() => Ok(a),
        _ => {
            if outer.errors.is_empty() {
                outer.errors.push(SyntaxError::at(
                    SyntaxErrorCode::Sunk,
                    "empty artefact",
                    0..0,
                    src,
                ));
            }
            Err(outer.errors)
        }
    }
}

/// A section keyword class (column-0 header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionKw {
    Specialise,
    Language,
    Description,
    Definition,
    Rules,
    Terminology,
    Annotations,
    ComponentTerminologies,
    RmOverlay,
    /// The ADL 1.4-only `revision_history` clause
    /// (`AM/docs/ADL1.4/master08-adl` §Revision History Section).
    RevisionHistory,
    /// The obsolete `concept` clause (`master07.09`) — recognised as a section
    /// boundary and otherwise ignored in ADL2.
    Concept,
    /// A new artefact keyword (`template_overlay`, …) — an artefact boundary.
    ArtefactBoundary,
}

fn classify_section(s: &str) -> Option<SectionKw> {
    match s {
        "specialize" | "specialise" => Some(SectionKw::Specialise),
        "language" => Some(SectionKw::Language),
        "description" => Some(SectionKw::Description),
        "definition" => Some(SectionKw::Definition),
        // Deprecated `invariant` maps to the rules section (`master07.09`).
        "rules" | "invariant" => Some(SectionKw::Rules),
        // Deprecated `ontology` maps to the terminology section (`master07.09`).
        "terminology" | "ontology" => Some(SectionKw::Terminology),
        "annotations" => Some(SectionKw::Annotations),
        "component_terminologies" => Some(SectionKw::ComponentTerminologies),
        // ADL 1.4-only (`master08` §Revision History Section); removed in ADL2
        // (`ADL2/master01-preface` §Changes from ADL 1.4). Recognised so a
        // spec-valid 1.4 archetype carrying one parses instead of sinking on
        // "expected a section header".
        "revision_history" => Some(SectionKw::RevisionHistory),
        // Obsolete in ADL2 (concept derives from the HRID); recognised as a
        // boundary and ignored (`master07.09`).
        "concept" => Some(SectionKw::Concept),
        // NOTE: `rm_overlay` is a spec-text section (`ADL2/master07.12`) that
        // the vendored adl-antlr `adl2.g4` predates; recognised here as an ODIN
        // section per the spec text.
        "rm_overlay" => Some(SectionKw::RmOverlay),
        "archetype"
        | "template"
        | "template_overlay"
        | "operational_template"
        | "operational_archetype" => Some(SectionKw::ArtefactBoundary),
        _ => None,
    }
}

fn classify_kind(s: &str) -> Option<ArtefactKind> {
    match s {
        "archetype" => Some(ArtefactKind::Archetype),
        "template" => Some(ArtefactKind::Template),
        "template_overlay" => Some(ArtefactKind::TemplateOverlay),
        // NOTE: `operational_archetype` accepted as `operational_template` per
        // the `ADL2/master07.04` keyword inconsistency.
        "operational_template" | "operational_archetype" => Some(ArtefactKind::OperationalTemplate),
        _ => None,
    }
}

struct Outer<'a> {
    src: &'a str,
    toks: &'a [Spanned],
    pos: usize,
    errors: Vec<SyntaxError>,
}

impl Outer<'_> {
    fn current(&self) -> Option<&Token> {
        self.toks.get(self.pos).map(|s| &s.token)
    }

    fn span_at(&self, idx: usize) -> std::ops::Range<usize> {
        self.toks
            .get(idx)
            .map_or(self.src.len()..self.src.len(), |s| s.span.clone())
    }

    fn push(
        &mut self,
        code: SyntaxErrorCode,
        msg: impl Into<String>,
        span: std::ops::Range<usize>,
    ) {
        self.errors.push(SyntaxError::at(code, msg, span, self.src));
    }

    /// True if the token at `idx` starts a line (its preceding byte is `\n`).
    fn is_line_start(&self, idx: usize) -> bool {
        let Some(sp) = self.toks.get(idx) else {
            return false;
        };
        let start = sp.span.start;
        start == 0 || self.src.as_bytes().get(start - 1) == Some(&b'\n')
    }

    /// The section-keyword class at `idx`, if that token is a column-0 keyword.
    fn section_kw_at(&self, idx: usize) -> Option<SectionKw> {
        if !self.is_line_start(idx) {
            return None;
        }
        if let Some(Token::AlphaLcId(s)) = self.toks.get(idx).map(|s| &s.token) {
            classify_section(s)
        } else {
            None
        }
    }

    fn parse_artefact(&mut self, overlay: bool) -> Option<SourceArtefact> {
        // Artefact kind keyword (accept an optional leading `flat`).
        if matches!(self.current(), Some(Token::AlphaLcId(s)) if s == "flat") {
            self.pos += 1;
        }
        let kind_span = self.span_at(self.pos);
        let kind = match self.current() {
            Some(Token::AlphaLcId(s)) => classify_kind(s),
            _ => None,
        };
        let Some(kind) = kind else {
            self.push(
                SyntaxErrorCode::Sunk,
                "expected an artefact keyword",
                kind_span,
            );
            return None;
        };
        self.pos += 1;

        let mut meta = ArtefactMeta::default();
        if matches!(self.current(), Some(Token::LParen)) {
            self.parse_meta(&mut meta);
        }

        // HRID.
        let hrid_span = self.span_at(self.pos);
        let Some(Token::ArchetypeId(hrid_str)) = self.current().cloned() else {
            self.push(
                SyntaxErrorCode::Sarid,
                "expected an archetype HRID after the artefact keyword",
                hrid_span,
            );
            return None;
        };
        self.pos += 1;
        let hrid = match parse_hrid(&hrid_str) {
            Ok(h) => h,
            Err(msg) => {
                self.push(SyntaxErrorCode::Sarid, msg, hrid_span);
                return None;
            }
        };

        let mut art = SourceArtefact {
            kind,
            meta,
            hrid,
            parent_ref: None,
            language: None,
            description: None,
            terminology: None,
            annotations: None,
            rm_overlay: None,
            component_terminologies: None,
            revision_history: None,
            definition: None,
            rules: None,
            overlays: Vec::new(),
        };

        // Sections.
        loop {
            let Some(kw) = self.section_kw_at(self.pos) else {
                if self.pos >= self.toks.len() {
                    break;
                }
                // An unexpected token where a section header was expected.
                let span = self.span_at(self.pos);
                self.push(SyntaxErrorCode::Sunk, "expected a section header", span);
                break;
            };
            if kw == SectionKw::ArtefactBoundary {
                break;
            }
            let header_idx = self.pos;
            self.pos += 1; // consume the section keyword
            let body = self.section_body_range();
            self.process_section(kw, header_idx, body, &mut art);
        }

        // A `template` (and, defensively, any root) collects trailing
        // `template_overlay` blocks (`adl2.g4` `template`).
        if !overlay {
            while matches!(self.current(), Some(Token::AlphaLcId(s)) if s == "template_overlay")
                && self.is_line_start(self.pos)
            {
                if let Some(ov) = self.parse_artefact(true) {
                    art.overlays.push(ov);
                } else {
                    break;
                }
            }
        }

        self.validate_required(&art, overlay, kind_span);
        Some(art)
    }

    /// The token index range of a section body: from the current position up to
    /// the next column-0 section/artefact keyword (or end of input).
    fn section_body_range(&self) -> std::ops::Range<usize> {
        let start = self.pos;
        let mut end = start;
        while end < self.toks.len() && self.section_kw_at(end).is_none() {
            end += 1;
        }
        start..end
    }

    fn process_section(
        &mut self,
        kw: SectionKw,
        header_idx: usize,
        body: std::ops::Range<usize>,
        art: &mut SourceArtefact,
    ) {
        self.pos = body.end;
        match kw {
            SectionKw::Specialise => {
                if body.len() == 1
                    && let Some(Token::ArchetypeId(s)) = self.toks.get(body.start).map(|t| &t.token)
                {
                    match parse_hrid(s) {
                        Ok(h) => art.parent_ref = Some(h),
                        Err(msg) => {
                            self.push(SyntaxErrorCode::Sasid, msg, self.span_at(body.start));
                        }
                    }
                } else {
                    self.push(
                        SyntaxErrorCode::Sasid,
                        "expected a single parent archetype id in the specialise section",
                        self.span_at(header_idx),
                    );
                }
            }
            SectionKw::Definition => {
                if body.is_empty() {
                    self.push(
                        SyntaxErrorCode::Sadf,
                        "empty definition section",
                        self.span_at(header_idx),
                    );
                } else {
                    art.definition = Some(self.raw_span(body));
                }
            }
            SectionKw::Rules => {
                if !body.is_empty() {
                    art.rules = Some(self.raw_span(body));
                }
            }
            SectionKw::Language => {
                art.language = self.parse_odin(body, header_idx, "language");
            }
            SectionKw::Description => {
                art.description = self.parse_odin(body, header_idx, "description");
            }
            SectionKw::Terminology => {
                art.terminology = self.parse_odin(body, header_idx, "terminology");
            }
            SectionKw::Annotations => {
                art.annotations = self.parse_odin(body, header_idx, "annotations");
            }
            SectionKw::RmOverlay => {
                art.rm_overlay = self.parse_odin(body, header_idx, "rm_overlay");
            }
            SectionKw::ComponentTerminologies => {
                art.component_terminologies =
                    self.parse_odin(body, header_idx, "component_terminologies");
            }
            SectionKw::RevisionHistory => {
                art.revision_history = self.parse_odin(body, header_idx, "revision_history");
            }
            // The obsolete `concept` clause is consumed and ignored in ADL2.
            SectionKw::Concept | SectionKw::ArtefactBoundary => {}
        }
    }

    fn raw_span(&self, tokens: std::ops::Range<usize>) -> RawSpan {
        let bytes = self.span_at(tokens.start).start..self.span_at(tokens.end - 1).end;
        RawSpan { bytes, tokens }
    }

    /// Parse an ODIN section body (delegated to `openehr_lang::odin`), mapping
    /// any failure to [`SyntaxErrorCode::Sdinv`] with the section name.
    fn parse_odin(
        &mut self,
        body: std::ops::Range<usize>,
        header_idx: usize,
        name: &str,
    ) -> Option<openehr_lang::odin::OdinValue> {
        if body.is_empty() {
            self.push(
                SyntaxErrorCode::Sdinv,
                format!("empty {name} section"),
                self.span_at(header_idx),
            );
            return None;
        }
        let byte_span = self.span_at(body.start).start..self.span_at(body.end - 1).end;
        let text = self.src.get(byte_span.clone()).unwrap_or("");
        match openehr_lang::odin::parse(text) {
            Ok(v) => Some(v),
            Err(e) => {
                // Offset the ODIN-local byte span into the whole-file source.
                let span = (byte_span.start + e.span.start)..(byte_span.start + e.span.end);
                self.push(
                    SyntaxErrorCode::Sdinv,
                    format!("invalid ODIN in {name} section: {e}"),
                    span,
                );
                None
            }
        }
    }

    fn parse_meta(&mut self, meta: &mut ArtefactMeta) {
        self.pos += 1; // consume '('
        while !matches!(self.current(), Some(Token::RParen) | None) {
            if matches!(self.current(), Some(Token::SymSemiColon)) {
                self.pos += 1;
                continue;
            }
            let Some(Token::AlphaLcId(key) | Token::AlphaUnderscoreId(key)) = self.current() else {
                // Unexpected meta token: skip to keep the parse going.
                self.pos += 1;
                continue;
            };
            let key = key.clone();
            self.pos += 1;
            let value = if matches!(self.current(), Some(Token::SymEq)) {
                self.pos += 1;
                let v = self.current().and_then(token_text);
                if v.is_some() {
                    self.pos += 1;
                }
                v
            } else {
                None
            };
            match key.as_str() {
                "adl_version" => meta.adl_version = value,
                "rm_release" => meta.rm_release = value,
                "uid" => meta.uid = value,
                "build_uid" => meta.build_uid = value,
                "provenance_id" => meta.provenance_id = value,
                "generated" => meta.generated = true,
                "controlled" => meta.controlled = Some(true),
                "uncontrolled" => meta.controlled = Some(false),
                _ => meta.other.push((key, value)),
            }
        }
        if matches!(self.current(), Some(Token::RParen)) {
            self.pos += 1;
        }
    }

    fn validate_required(
        &mut self,
        art: &SourceArtefact,
        overlay: bool,
        kind_span: std::ops::Range<usize>,
    ) {
        if art.definition.is_none() {
            self.push(
                SyntaxErrorCode::Sadf,
                "missing definition section",
                kind_span.clone(),
            );
        }
        if art.terminology.is_none() {
            self.push(
                SyntaxErrorCode::Saon,
                "missing terminology section",
                kind_span.clone(),
            );
        }
        // template_overlay inherits language/description from its root
        // (`master07.07` / `master10`); every other kind requires a language
        // section.
        if !overlay && art.kind != ArtefactKind::TemplateOverlay && art.language.is_none() {
            self.push(
                SyntaxErrorCode::Salan,
                "missing language section",
                kind_span,
            );
        }
    }
}

/// The text a meta value token carries, if any.
fn token_text(t: &Token) -> Option<String> {
    match t {
        Token::VersionId(s)
        | Token::Guid(s)
        | Token::String(s)
        | Token::Integer(s)
        | Token::Real(s)
        | Token::ArchetypeId(s)
        | Token::AlphaLcId(s)
        | Token::AlphaUcId(s) => Some(s.clone()),
        _ => None,
    }
}

/// Parse an archetype HRID (or a version-partial specialise reference) into an
/// [`ArchetypeHrid`], normalising a partial version per `master07.05`.
///
/// Form: `[ns::]publisher-package-class.concept.vMAJOR[.MINOR[.PATCH]]
/// [-rc|-alpha|-beta[.build]]`.
///
/// # Errors
/// Returns a message describing the first structural problem.
pub fn parse_hrid(s: &str) -> Result<ArchetypeHrid, String> {
    let (namespace, rest) = match s.split_once("::") {
        Some((ns, rest)) => (Some(ns.to_owned()), rest),
        None => (None, s),
    };

    let vpos = rest
        .rfind(".v")
        .filter(|&i| rest[i + 2..].starts_with(|c: char| c.is_ascii_digit()))
        .ok_or_else(|| format!("HRID {s:?} has no `.vN` version segment"))?;
    let left = &rest[..vpos];
    let version = &rest[vpos + 2..];

    let (model_part, concept_id) = left
        .rsplit_once('.')
        .ok_or_else(|| format!("HRID {s:?} has no `.concept` segment"))?;

    let segments: Vec<&str> = model_part.split('-').collect();
    let [publisher, package, class] = segments.as_slice() else {
        return Err(format!(
            "HRID {s:?} model part must be `publisher-package-class`, found {model_part:?}"
        ));
    };
    if publisher.is_empty() || package.is_empty() || class.is_empty() || concept_id.is_empty() {
        return Err(format!("HRID {s:?} has an empty identifier segment"));
    }

    let (release_version, version_status, build_count) = parse_version(version)?;

    Ok(ArchetypeHrid {
        namespace,
        rm_publisher: (*publisher).to_owned(),
        rm_package: (*package).to_owned(),
        rm_class: (*class).to_owned(),
        concept_id: concept_id.to_owned(),
        release_version,
        version_status: VersionStatus::from_wire(version_status),
        build_count,
    })
}

/// Parse the version tail into `(release_version, status, build_count)`,
/// normalising a 1- or 2-part numeric version to 3 parts (`master07.05`;
/// 1.4 `v1` ⇒ `1.0.0`).
fn parse_version(version: &str) -> Result<(String, &'static str, String), String> {
    let (status, numeric, build) = if let Some((numeric, build)) = split_status(version, "-rc") {
        ("rc", numeric, build)
    } else if let Some((numeric, build)) = split_status(version, "-alpha") {
        ("alpha", numeric, build)
    } else if let Some((numeric, build)) = split_status(version, "-beta") {
        ("beta", numeric, build)
    } else {
        ("", version, "")
    };

    let mut parts = numeric.split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    let patch = parts.next().unwrap_or("0");
    if major.is_empty() || !major.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("invalid version {version:?}"));
    }
    let release_version = format!(
        "{}.{}.{}",
        major,
        if minor.is_empty() { "0" } else { minor },
        if patch.is_empty() { "0" } else { patch }
    );
    Ok((release_version, status, build.to_owned()))
}

/// If `version` carries the `marker` pre-release suffix, split into
/// `(numeric, build)` where `build` is the `.N` count after the marker (empty
/// if absent).
fn split_status<'a>(version: &'a str, marker: &str) -> Option<(&'a str, &'a str)> {
    let idx = version.find(marker)?;
    let numeric = &version[..idx];
    let after = &version[idx + marker.len()..];
    let build = after.strip_prefix('.').unwrap_or("");
    Some((numeric, build))
}

#[cfg(test)]
#[allow(clippy::panic)] // test assertions panic by design
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_archetype() {
        let src = "archetype (adl_version=2.0.5; rm_release=1.0.2)\n\
                   \topenehr-TEST_PKG-WHOLE.most_minimal.v2.0.0\n\
                   \nlanguage\n\toriginal_language = <[ISO_639-1::en]>\n\
                   \ndescription\n\tlifecycle_state = <\"published\">\n\
                   \ndefinition\n\tWHOLE[id1]\n\
                   \nterminology\n\tterm_definitions = <\n\t\t[\"en\"] = <\n\t\t\t[\"id1\"] = <\n\t\t\t\ttext = <\"x\">\n\t\t\t\tdescription = <\"x\">\n\t\t\t>\n\t\t>\n\t>\n";
        let a = parse_source(src).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
        assert_eq!(a.kind, ArtefactKind::Archetype);
        assert_eq!(a.meta.adl_version.as_deref(), Some("2.0.5"));
        assert_eq!(a.meta.rm_release.as_deref(), Some("1.0.2"));
        assert_eq!(a.hrid.rm_publisher, "openehr");
        assert_eq!(a.hrid.rm_package, "TEST_PKG");
        assert_eq!(a.hrid.rm_class, "WHOLE");
        assert_eq!(a.hrid.concept_id, "most_minimal");
        assert_eq!(a.hrid.release_version, "2.0.0");
        assert!(a.language.is_some());
        assert!(a.description.is_some());
        assert!(a.terminology.is_some());
        let def = a.definition.expect("definition present");
        assert!(!def.tokens.is_empty());
        // the raw definition span covers `WHOLE[id1]`.
        assert_eq!(&src[def.bytes.clone()], "WHOLE[id1]");
    }

    #[test]
    fn missing_definition_is_sadf() {
        let src = "archetype (adl_version=2.0.5; rm_release=1.0.2)\n\
                   \topenehr-TEST_PKG-WHOLE.x.v1.0.0\n\
                   \nlanguage\n\toriginal_language = <[ISO_639-1::en]>\n\
                   \ndescription\n\tlifecycle_state = <\"published\">\n\
                   \nterminology\n\tterm_definitions = <\n\t\t[\"en\"] = <>\n\t>\n";
        let errs = parse_source(src).expect_err("should fail");
        assert!(
            errs.iter().any(|e| e.code == SyntaxErrorCode::Sadf),
            "{errs:?}"
        );
    }

    #[test]
    fn hrid_forms() {
        let h = parse_hrid("openEHR-EHR-OBSERVATION.blood_pressure.v1.2.3").expect("full");
        assert_eq!(h.namespace, None);
        assert_eq!(h.rm_publisher, "openEHR");
        assert_eq!(h.rm_class, "OBSERVATION");
        assert_eq!(h.release_version, "1.2.3");
        // An empty status token is outside the `VERSION_STATUS` constant set, so
        // `from_wire` preserves it verbatim as `Other` (HRID tolerance).
        assert_eq!(h.version_status, VersionStatus::Other(String::new()));

        // 1.4 single-number version normalises to 3 parts.
        let h = parse_hrid("openehr-TASK_PLANNING-TASK_PLAN.good_include.v0").expect("partial");
        assert_eq!(h.release_version, "0.0.0");

        // namespaced + release-candidate with a build count.
        let h = parse_hrid("uk.gov::openEHR-EHR-CLUSTER.device.v1.0.0-rc.2").expect("ns+rc");
        assert_eq!(h.namespace.as_deref(), Some("uk.gov"));
        // `rc` is not a `VERSION_STATUS` constant (`release_candidate` is), so the
        // out-of-set token is preserved as `Other`.
        assert_eq!(h.version_status, VersionStatus::Other("rc".to_owned()));
        assert_eq!(h.build_count, "2");

        // alpha with no build count.
        let h = parse_hrid("openEHR-EHR-OBSERVATION.x.v0.0.1-alpha").expect("alpha");
        assert_eq!(h.version_status, VersionStatus::Alpha);
        assert_eq!(h.build_count, "");

        assert!(parse_hrid("not-an-hrid").is_err());
    }
}
