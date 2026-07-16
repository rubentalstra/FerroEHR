//! Surface A2 — registration-side validity for uploaded ADL2 sources
//! (`I_DEFINITION_ADL2`).
//!
//! The AOM2 validation catalogue (`docs/specs/openehr/AM/docs/AOM2/`
//! `master08-validation.adoc`) frames its phases as passes of an *archetype
//! compiler*. This product contains no ADL2 compiler: the ADL2 surface is the
//! SM registration registry (`SM/docs/openehr_platform/master04-definition_
//! package.adoc` `I_DEFINITION_ADL2` — store/retrieve/list sources), and the
//! operational constraint surface is OPT 1.4 (the sibling `opt` pass, where the
//! artefact catalogue IS enforced). What a registry can honestly enforce on a
//! *source* without compiling cADL is the section-structural and
//! terminology-side subset below — every rule of the catalogue that is
//! decidable from the header, the ODIN sections (`language`, `terminology`),
//! and a lexical scan of the `definition` section:
//!
//! - **STCNT** — mandatory parts present (`definition`, `terminology`,
//!   `language`) (`AOM2/master08-validation.adoc` Phase 1;
//!   `ADL2/master04.6-cadl_validity_rules.adoc` STCNT).
//! - **VARAV / VARRV** — `adl_version` / `rm_release` header values are valid
//!   dotted-numeric version ids (master08 meta-data checks).
//! - **VARDT** — the definition's root RM type matches the type slot of the
//!   `ARCHETYPE_HRID` (master08 Phase 1).
//! - **VARCN** — the root node id has the form `id1{.1}*` / `at0000{.1}*`,
//!   with extensions iff the artefact specialises (master08 Phase 1;
//!   `AOM2/master03-archetype_package.adoc` VARCN).
//! - **VACSD** — specialisation depth is exactly parent depth + 1, checked
//!   when the parent artefact is resolvable in the registry (master08 Phase 1).
//! - **VOLT** — the original language exists in the terminology (master08).
//! - **VOTM / VTLC** — every translation language has terminology sets, and
//!   all languages define the same code sets (master08;
//!   `AOM2/master07-terminology_package.adoc` VOTM/VTLC).
//! - **VATDF / VACDF / VATID** — every local `[idN]`/`[atN]`/`[acN]` code
//!   used in the definition is defined in `term_definitions` (master08 §Code
//!   Validation; `AOM2/master03-archetype_package.adoc` VATDF/VACDF).
//! - **VTVSID / VTVSMD / VTVSUQ** — value-set ids are defined ac-codes,
//!   members are defined at-codes, members are unique
//!   (`AOM2/master07-terminology_package.adoc`).
//! - **VTTBK** — term-binding keys are defined codes or paths (master07).
//!
//! The rule phases run in the catalogue order above; **the first violation
//! wins** (the phase sequence in [`validate_adl2_source`] is therefore part of
//! the behavioural contract, not an implementation detail).
//!
//! PORT NOTE: the cADL-semantic and specialisation-flattening rules (the
//! RM-conformance `VCxxx` family on parsed constraints, the `VSxxx` redefinition
//! family against a flat parent, the Sxxx cADL syntax codes, tuple/slot
//! constraint semantics) bind the compiler/flattener the product does not
//! contain; no CNF Robot suite or ECC case exercises ADL2 compilation. They
//! are inapplicable at this surface, not silently skipped — data validation
//! happens against OPT 1.4 (the sibling `opt` pass).

mod odin;

use std::collections::HashSet;
use std::fmt::Write as _;

use odin::OdinValue;

/// One registration-validity violation: the AOM2 rule code + a human detail.
#[derive(Debug)]
pub(crate) struct Adl2Violation {
    pub(crate) code: &'static str,
    pub(crate) detail: String,
}

impl Adl2Violation {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// What the structural validation learns about an artefact — the upload path
/// uses it for storage keys and the VACSD parent check.
pub(crate) struct Adl2Meta {
    /// `archetype` / `template` / `template_overlay` / `operational_template`.
    pub(crate) kind: &'static str,
    /// The `ARCHETYPE_HRID` on the line after the header.
    pub(crate) hrid: String,
    /// The parent HRID from the `specialize` section, when present.
    pub(crate) parent_hrid: Option<String>,
    /// Specialisation depth = extension count of the root node id
    /// (`id1` → 0, `id1.1` → 1, …).
    pub(crate) depth: usize,
}

/// Validate one ADL2 source at registration. Returns the artefact metadata or
/// the first violation, running the rule phases in the catalogue order the
/// module doc lists.
///
/// # Errors
///
/// The first [`Adl2Violation`] any phase finds (rule codes in the module doc).
pub(crate) fn validate_adl2_source(src: &str) -> Result<Adl2Meta, Adl2Violation> {
    let text = src.trim_start_matches('\u{feff}');
    let sections = split_sections(text);

    let (kind, hrid) = check_header(&sections)?;
    check_mandatory_sections(&sections)?;
    let definition = sections.definition.as_deref().unwrap_or("");
    let terminology_src = sections.terminology.as_deref().unwrap_or("");
    let language_src = sections.language.as_deref().unwrap_or("");

    let parent_hrid = sections.specialize.clone();
    let depth = check_definition_root(definition, &hrid, parent_hrid.as_deref())?;

    let terminology = OdinValue::parse(terminology_src).ok_or_else(|| {
        Adl2Violation::new(
            "STCNT",
            "the terminology section is not parseable ODIN".to_owned(),
        )
    })?;
    let by_language = term_codes_by_language(&terminology);
    check_language_code_sets(&by_language)?; // VTLC
    check_language_section(language_src, &by_language)?; // VOLT + VOTM

    // The globally defined local codes (any language — VTLC already enforced
    // cross-language equality).
    let defined: HashSet<String> = by_language
        .iter()
        .flat_map(|(_, codes)| codes.iter().cloned())
        .collect();

    check_differential_paths(definition, parent_hrid.is_some())?; // VDIFV
    check_code_specialisation_levels(&defined, depth)?; // VTSD
    check_definition_codes(definition, &defined)?; // C_TERMINOLOGY_CODE + VATDF/VACDF
    check_value_sets(&terminology, &defined)?; // VTVSID / VTVSMD / VTVSUQ
    check_assumed_value_set_membership(&terminology, definition)?; // VATDA
    check_term_bindings(&terminology, &defined)?; // VTTBK

    Ok(Adl2Meta {
        kind,
        hrid,
        parent_hrid,
        depth,
    })
}

/// VACSD: the specialisation depth must be exactly one greater than the
/// parent's (AOM2 master08 Phase 1). Callable once the parent's source has
/// been fetched from the registry; `None` parent-depth (parent not stored /
/// not introspectable) skips the check — registration order is not
/// constrained by the spec.
///
/// # Errors
///
/// An [`Adl2Violation`] with code `VACSD` when
/// `meta.depth != parent_depth + 1`.
pub(crate) fn check_specialisation_depth(
    meta: &Adl2Meta,
    parent_depth: usize,
) -> Result<(), Adl2Violation> {
    if meta.depth != parent_depth + 1 {
        return Err(Adl2Violation::new(
            "VACSD",
            format!(
                "specialisation depth {} is not parent depth {} + 1",
                meta.depth, parent_depth
            ),
        ));
    }
    Ok(())
}

// ─── rule phases (in catalogue order) ───────────────────────────────────────

/// Header phase: the artefact keyword is one of the four AOM2 kinds, the
/// `adl_version`/`rm_release` header attributes are dotted-numeric version
/// ids (VARAV / VARRV — AOM2 master08 meta-data checks), and the header is
/// followed by an `ARCHETYPE_HRID` (STCNT).
fn check_header(sections: &Sections) -> Result<(&'static str, String), Adl2Violation> {
    let Some(header) = sections.header.as_deref() else {
        return Err(Adl2Violation::new(
            "STCNT",
            "the source has no ADL2 artefact header \
             (archetype/template/template_overlay/operational_template)",
        ));
    };
    let keyword = header.split(['(', ' ', '\t']).next().unwrap_or("");
    let kind = match keyword {
        "archetype" => "archetype",
        "template" => "template",
        "template_overlay" => "template_overlay",
        "operational_template" => "operational_template",
        other => {
            return Err(Adl2Violation::new(
                "STCNT",
                format!("'{other}' is not an ADL2 artefact keyword"),
            ));
        }
    };
    // VARAV / VARRV: `(adl_version=2.0.6; rm_release=1.0.2; …)` values must be
    // dotted-numeric version ids (AOM2 master08 meta-data checks).
    if let Some(attrs) = header
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(attrs, _)| attrs)
    {
        for attr in attrs.split(';') {
            let attr = attr.trim();
            let Some((name, value)) = attr.split_once('=') else {
                continue; // flag-style attrs like `generated`
            };
            let (name, value) = (name.trim(), value.trim());
            let code = match name {
                "adl_version" => "VARAV",
                "rm_release" => "VARRV",
                _ => continue,
            };
            if !is_dotted_numeric_version(value) {
                return Err(Adl2Violation::new(
                    code,
                    format!("header {name} '{value}' is not a valid version id"),
                ));
            }
        }
    }
    let Some(hrid) = sections.hrid.clone() else {
        return Err(Adl2Violation::new(
            "STCNT",
            "the artefact header is not followed by an ARCHETYPE_HRID",
        ));
    };
    Ok((kind, hrid))
}

/// STCNT: mandatory parts present (AOM2 master08 Phase 1 "missing mandatory
/// part"; an ADL2 source archetype always carries language, definition and
/// terminology sections).
fn check_mandatory_sections(sections: &Sections) -> Result<(), Adl2Violation> {
    for (name, present) in [
        ("language", sections.language.is_some()),
        ("definition", sections.definition.is_some()),
        ("terminology", sections.terminology.is_some()),
    ] {
        if !present {
            return Err(Adl2Violation::new(
                "STCNT",
                format!("mandatory section '{name}' is missing"),
            ));
        }
    }
    Ok(())
}

/// The definition root line `TYPE[idN] matches {`: parseable (STCNT), the
/// root RM type matches the HRID's type slot (VARDT — composite identifiers
/// compare case-insensitively, BASE `base_types` master05 §Composite
/// Identifiers and Case), and the root node id has the required
/// `id1{.1}*` / `at0000{.1}*` form with extensions iff the artefact
/// specialises (VARCN — AOM2 master03 VARCN; master08 Phase 1). Returns the
/// specialisation depth.
fn check_definition_root(
    definition: &str,
    hrid: &str,
    specialize: Option<&str>,
) -> Result<usize, Adl2Violation> {
    let root_line = definition
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("--"))
        .unwrap_or("");
    let (root_type, root_node_id) = parse_root(root_line).ok_or_else(|| {
        Adl2Violation::new(
            "STCNT",
            format!("the definition does not open with `RM_TYPE[node_id]` (got '{root_line}')"),
        )
    })?;

    // VARDT: root RM type vs the HRID type slot.
    let hrid_core = hrid.rsplit_once("::").map_or(hrid, |(_, r)| r);
    let qualified = hrid_core.split('.').next().unwrap_or("");
    let entity = qualified
        .match_indices('-')
        .nth(1)
        .map_or("", |(i, _)| &qualified[i + 1..]);
    if !entity.eq_ignore_ascii_case(root_type) {
        return Err(Adl2Violation::new(
            "VARDT",
            format!(
                "the definition root RM type '{root_type}' does not match the type slot \
                 '{entity}' of '{hrid_core}'"
            ),
        ));
    }

    // VARCN: root node id form `id1{.1}*` / `at0000{.1}*`; the number of `.1`
    // extensions is the specialisation depth and must be > 0 iff the artefact
    // specialises (AOM2 master03 VARCN; master08 Phase 1).
    let depth = root_node_depth(root_node_id).ok_or_else(|| {
        Adl2Violation::new(
            "VARCN",
            format!("root node id '{root_node_id}' is not of the form id1{{.1}}* / at0000{{.1}}*"),
        )
    })?;
    match (specialize, depth) {
        (None, d) if d > 0 => Err(Adl2Violation::new(
            "VARCN",
            format!(
                "root node id '{root_node_id}' is specialised (depth {d}) but the artefact \
                 has no `specialize` section"
            ),
        )),
        (Some(_), 0) => Err(Adl2Violation::new(
            "VARCN",
            format!(
                "the artefact specialises a parent but its root node id '{root_node_id}' \
                 has specialisation depth 0"
            ),
        )),
        _ => Ok(depth),
    }
}

/// The per-language locally-defined code sets of the `term_definitions`
/// ODIN block.
fn term_codes_by_language(terminology: &OdinValue) -> Vec<(String, HashSet<String>)> {
    terminology
        .attr("term_definitions")
        .map(|td| {
            td.keyed_entries()
                .map(|(lang, codes)| (lang.to_owned(), codes.keys()))
                .collect()
        })
        .unwrap_or_default()
}

/// VTLC: all languages define the same code set (AOM2 master07 VTLC).
fn check_language_code_sets(
    by_language: &[(String, HashSet<String>)],
) -> Result<(), Adl2Violation> {
    if let Some(((ref_lang, ref_codes), rest)) = by_language.split_first() {
        for (lang, codes) in rest {
            if codes != ref_codes {
                let mut diff: Vec<&str> = ref_codes
                    .symmetric_difference(codes)
                    .map(String::as_str)
                    .collect();
                diff.sort_unstable();
                return Err(Adl2Violation::new(
                    "VTLC",
                    format!(
                        "the term code set differs between languages '{ref_lang}' and '{lang}' \
                         (e.g. {diff:?}); all codes must exist in all languages"
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// VOLT: the original language is a terminology language (AOM2 master08);
/// VOTM: every declared translation language must have terminology sets
/// (AOM2 master03 VOTM). Both are decidable only when the `language` section
/// parses and names an `original_language` code.
fn check_language_section(
    language_src: &str,
    by_language: &[(String, HashSet<String>)],
) -> Result<(), Adl2Violation> {
    let Some(language) = OdinValue::parse(language_src) else {
        return Ok(());
    };
    let Some(orig) = language
        .attr("original_language")
        .and_then(OdinValue::code_string)
    else {
        return Ok(());
    };
    let orig_code = orig.rsplit_once("::").map_or(orig.as_str(), |(_, c)| c);
    if !by_language.is_empty() && !by_language.iter().any(|(l, _)| l == orig_code) {
        return Err(Adl2Violation::new(
            "VOLT",
            format!("original language '{orig_code}' has no term_definitions set"),
        ));
    }
    if let Some(translations) = language.attr("translations") {
        for (lang, _) in translations.keyed_entries() {
            if !by_language.iter().any(|(l, _)| l == lang) {
                return Err(Adl2Violation::new(
                    "VOTM",
                    format!("translation language '{lang}' has no term_definitions set"),
                ));
            }
        }
    }
    Ok(())
}

/// VDIFV: a differential path (an attribute line opening with `/`) may only
/// appear in a specialised archetype (AOM2 master04.5 VDIFV).
fn check_differential_paths(definition: &str, specialised: bool) -> Result<(), Adl2Violation> {
    if !specialised
        && let Some(diff) = definition
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with('/') && l.contains("matches"))
    {
        return Err(Adl2Violation::new(
            "VDIFV",
            format!(
                "differential path '{}' in a non-specialised artefact",
                diff.split_whitespace().next().unwrap_or(diff)
            ),
        ));
    }
    Ok(())
}

/// VTSD: the specialisation level of every defined code must be no greater
/// than the artefact's specialisation depth (AOM2 master07 VTSD — flat form:
/// same or less).
fn check_code_specialisation_levels(
    defined: &HashSet<String>,
    depth: usize,
) -> Result<(), Adl2Violation> {
    for code in defined {
        let code_depth = code.chars().filter(|c| *c == '.').count();
        if code_depth > depth {
            return Err(Adl2Violation::new(
                "VTSD",
                format!(
                    "code '{code}' has specialisation level {code_depth}, deeper than the \
                     artefact's depth {depth}"
                ),
            ));
        }
    }
    Ok(())
}

/// VATDF / VACDF / VATID: every local `[idN]`/`[atN]`/`[acN]` code used in
/// the definition must be defined (AOM2 master08 §Code Validation). A
/// terminology-qualified code (`[snomed_ct::…]`) is an external binding
/// (VETDF is advisory: "warn only" where the terminology is inaccessible).
/// The `C_TERMINOLOGY_CODE` structural rule rides along: a constraint is a
/// single ac-code, optionally with ONE assumed at-code (`[acN; atM]`), or a
/// single at-code — nothing else (AOM2 master04.2 §Terminology Constraints,
/// Formal Definition; ADL2 removed the ADL 1.4 inline at-code value list in
/// favour of value sets).
fn check_definition_codes(
    definition: &str,
    defined: &HashSet<String>,
) -> Result<(), Adl2Violation> {
    for (head, parts) in bracket_tokens(definition) {
        if parts.len() > 1
            && !(parts.len() == 2 && head.starts_with("ac") && parts[1].starts_with("at"))
        {
            return Err(Adl2Violation::new(
                "C_TERMINOLOGY_CODE_validity",
                format!(
                    "the terminology-code constraint [{}] is neither a single at/ac-code nor \
                     an `[ac; assumed-at]` pair",
                    parts.join("; ")
                ),
            ));
        }
        for code in parts {
            if !defined.contains(&code) {
                return Err(Adl2Violation::new(
                    if code.starts_with("ac") {
                        "VACDF"
                    } else {
                        "VATDF"
                    },
                    format!(
                        "code '{code}' is used in the definition but not defined in terminology"
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Value sets: VTVSID (id is a defined ac-code), VTVSMD (members are defined
/// at-codes), VTVSUQ (members unique) — AOM2 master07.
fn check_value_sets(
    terminology: &OdinValue,
    defined: &HashSet<String>,
) -> Result<(), Adl2Violation> {
    if let Some(value_sets) = terminology.attr("value_sets") {
        for (vs_id, vs) in value_sets.keyed_entries() {
            if !defined.contains(vs_id) {
                return Err(Adl2Violation::new(
                    "VTVSID",
                    format!("value-set id '{vs_id}' is not defined in term_definitions"),
                ));
            }
            let members = vs
                .attr("members")
                .map(OdinValue::string_items)
                .unwrap_or_default();
            let mut seen = HashSet::new();
            for member in &members {
                if !defined.contains(member.as_str()) {
                    return Err(Adl2Violation::new(
                        "VTVSMD",
                        format!(
                            "value-set '{vs_id}' member '{member}' is not defined in \
                             term_definitions"
                        ),
                    ));
                }
                if !seen.insert(member) {
                    return Err(Adl2Violation::new(
                        "VTVSUQ",
                        format!("value-set '{vs_id}' member '{member}' is duplicated"),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// VATDA: a `C_TERMINOLOGY_CODE` assumed at-code (`[acN; atM]`) must belong to
/// the value set identified by the ac-code (AOM2 master03 VATDA).
fn check_assumed_value_set_membership(
    terminology: &OdinValue,
    definition: &str,
) -> Result<(), Adl2Violation> {
    let Some(value_sets) = terminology.attr("value_sets") else {
        return Ok(());
    };
    for (head, parts) in bracket_tokens(definition) {
        if !head.starts_with("ac") || parts.len() < 2 {
            continue;
        }
        let Some(vs) = value_sets
            .keyed_entries()
            .find(|(k, _)| *k == head)
            .map(|(_, v)| v)
        else {
            continue;
        };
        let members = vs
            .attr("members")
            .map(OdinValue::string_items)
            .unwrap_or_default();
        for assumed in &parts[1..] {
            if assumed.starts_with("at") && !members.contains(&assumed) {
                return Err(Adl2Violation::new(
                    "VATDA",
                    format!("assumed code '{assumed}' is not a member of value set '{head}'"),
                ));
            }
        }
    }
    Ok(())
}

/// VTTBK: term-binding keys are defined codes or paths (AOM2 master07).
fn check_term_bindings(
    terminology: &OdinValue,
    defined: &HashSet<String>,
) -> Result<(), Adl2Violation> {
    if let Some(bindings) = terminology.attr("term_bindings") {
        for (_terminology_id, set) in bindings.keyed_entries() {
            for (key, _) in set.keyed_entries() {
                if !key.starts_with('/') && is_local_code(key) && !defined.contains(key) {
                    return Err(Adl2Violation::new(
                        "VTTBK",
                        format!("term binding key '{key}' is neither a defined code nor a path"),
                    ));
                }
            }
        }
    }
    Ok(())
}

// ─── section splitting ──────────────────────────────────────────────────────

#[derive(Default)]
struct Sections {
    header: Option<String>,
    hrid: Option<String>,
    specialize: Option<String>,
    language: Option<String>,
    definition: Option<String>,
    terminology: Option<String>,
}

/// Split an ADL2 source on its column-0 section keywords. Section bodies are
/// everything up to the next column-0 keyword.
fn split_sections(text: &str) -> Sections {
    const KEYWORDS: &[&str] = &[
        "specialize",
        "specialise",
        "concept",
        "language",
        "description",
        "definition",
        "rules",
        "rm_overlay",
        "terminology",
        "annotations",
        "component_terminologies",
    ];
    let mut out = Sections::default();
    let mut current: Option<(&str, String)> = None;
    let mut lines = text.lines();

    // Header = the first non-empty, non-comment line; HRID = the next.
    for line in lines.by_ref() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("--") {
            continue;
        }
        out.header = Some(t.to_owned());
        break;
    }
    for line in lines.by_ref() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("--") {
            continue;
        }
        out.hrid = Some(t.to_owned());
        break;
    }

    let mut store = |name: &str, body: String| match name {
        "specialize" | "specialise" => {
            out.specialize = body
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with("--"))
                .map(str::to_owned);
        }
        "language" => out.language = Some(body),
        "definition" => out.definition = Some(body),
        "terminology" => out.terminology = Some(body),
        _ => {}
    };

    for line in lines {
        let is_keyword = !line.starts_with([' ', '\t']) && KEYWORDS.contains(&line.trim_end());
        if is_keyword {
            if let Some((name, body)) = current.take() {
                store(name, body);
            }
            current = Some((line.trim_end(), String::new()));
        } else if let Some((_, body)) = &mut current {
            let _ = writeln!(body, "{line}");
        }
    }
    if let Some((name, body)) = current.take() {
        store(name, body);
    }
    out
}

// ─── lexical helpers ────────────────────────────────────────────────────────

/// `TYPE[node_id]` at the start of the definition (whitespace inside the
/// brackets tolerated; a generic type like `DV_INTERVAL<DV_QUANTITY>` keeps
/// its base name).
fn parse_root(line: &str) -> Option<(&str, &str)> {
    let open = line.find('[')?;
    let close = line[open..].find(']')? + open;
    let ty = line[..open].trim();
    let ty = ty.split('<').next().unwrap_or(ty).trim();
    let node_id = line[open + 1..close].trim();
    (!ty.is_empty() && !node_id.is_empty()).then_some((ty, node_id))
}

/// Depth of a root node id of the required `id1{.1}*` / `at0000{.1}*` form
/// (AOM2 master08 Phase 1): the extension count, or `None` if malformed.
fn root_node_depth(node_id: &str) -> Option<usize> {
    let segments: Vec<&str> = node_id.split('.').collect();
    let (first, exts) = segments.split_first()?;
    if *first != "id1" && *first != "at0000" {
        return None;
    }
    exts.iter()
        .all(|e| !e.is_empty() && e.bytes().all(|b| b.is_ascii_digit()))
        .then_some(exts.len())
}

/// `2`, `2.0.6`, `1.0.2` — dotted numeric version id (VARAV/VARRV).
fn is_dotted_numeric_version(v: &str) -> bool {
    !v.is_empty()
        && v.split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Every bracket token in the definition, split into its code parts: a plain
/// `[at5]` yields `("at5", ["at5"])`; the `C_TERMINOLOGY_CODE` constraint form
/// `[ac1; at5]` (ac-code with an assumed at-code, cADL §Terminology
/// Constraints) yields `("ac1", ["ac1", "at5"])`. Non-local (qualified /
/// free-text) parts are dropped.
fn bracket_tokens(definition: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(open_rel) = definition[i..].find('[') {
        let open = i + open_rel;
        let Some(close_rel) = definition[open..].find(']') else {
            break;
        };
        let close = open + close_rel;
        let token = definition[open + 1..close].trim();
        let parts: Vec<String> = token
            .split([';', ','])
            .map(str::trim)
            .filter(|p| is_local_code(p))
            .map(str::to_owned)
            .collect();
        if let Some(first) = parts.first() {
            out.push((first.clone(), parts.clone()));
        }
        i = close + 1;
        if i >= definition.len() {
            break;
        }
    }
    out
}

/// `idN`, `atN`, `acN` with optional dotted numeric extensions — a LOCAL
/// archetype code (a `::`-qualified code is an external binding).
fn is_local_code(token: &str) -> bool {
    if token.contains("::") {
        return false;
    }
    let rest = token
        .strip_prefix("id")
        .or_else(|| token.strip_prefix("at"))
        .or_else(|| token.strip_prefix("ac"));
    rest.is_some_and(|r| {
        !r.is_empty()
            && r.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    })
}

#[cfg(test)]
mod tests;
