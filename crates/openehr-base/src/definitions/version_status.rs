//! `VERSION_STATUS` — status of a versioned artefact.
//!
//! openEHR class: `VERSION_STATUS` (enumeration), package
//! `base.base_types.definitions`.
//!
//! Status of a versioned artefact, as one of a number of possible values:
//! uncontrolled, prerelease, release, build. Each value corresponds to a
//! specific rendering of a semantic-versioning-style `N.M.P` string, as
//! documented per variant below.

/// Closed five-value enumeration, transcribed directly as a Rust `enum`
/// with the spec's exact lower-case/snake_case symbol names preserved via
/// [`VersionStatus::symbol`].
///
/// PORT NOTE: as with `ValidityKind` in this same package, no `serde`
/// derive is added — `openehr-base` has no `serde` dependency yet, matching
/// the sibling `openehr-foundation::primitive_types` cluster. `symbol()`
/// carries the spec's own identifier for a later serde impl at the RM layer
/// to key its rename off of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VersionStatus {
    /// `alpha` — a version which is 'unstable', i.e. contains an unknown
    /// size of change with respect to its base version. Rendered with the
    /// build number as a string in the form `N.M.P-alpha.B`, e.g.
    /// `2.0.1-alpha.154`.
    Alpha,

    /// `beta` — a version which is 'beta', i.e. contains an unknown but
    /// reducing size of change with respect to its base version. Rendered
    /// with the build number as a string in the form `N.M.P-beta.B`, e.g.
    /// `2.0.1-beta.154`.
    Beta,

    /// `release_candidate` — a version which is 'release candidate', i.e.
    /// contains only patch-level changes on the base version. Rendered as a
    /// string as `N.M.P-rc.B`, e.g. `2.0.1-rc.27`.
    ReleaseCandidate,

    /// `released` — a version which is 'released', i.e. is the definitive
    /// base version. Rendered as `N.M.P`, e.g. `2.0.1`.
    Released,

    /// `build` — a version which is a build of the current base release.
    /// Rendered with the build number as a string in the form `N.M.P+B`,
    /// e.g. `2.0.1+33`.
    Build,
}

impl VersionStatus {
    /// The spec's own snake_case symbol name for this enumeration value.
    pub const fn symbol(self) -> &'static str {
        match self {
            VersionStatus::Alpha => "alpha",
            VersionStatus::Beta => "beta",
            VersionStatus::ReleaseCandidate => "release_candidate",
            VersionStatus::Released => "released",
            VersionStatus::Build => "build",
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/version_status.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-definitions_package.adoc §Class Definitions / version_status.adoc §VERSION_STATUS Enumeration
//   confidence: high
//   todos: 0
//   note: closed 5-value enum with a symbol() method carrying the spec's own snake_case name; render-format strings (N.M.P-alpha.B etc.) are documentation only, not implemented as a formatting method since the spec table does not define one as a class function.
// ─────────────────────────────────────────────
