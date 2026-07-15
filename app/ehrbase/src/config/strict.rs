//! Strict-validation helpers (§5.3): the reserved-namespace env sweep, the
//! did-you-mean suggester, and its Damerau-Levenshtein metric. No openEHR spec
//! governs configuration — our own design.

use std::collections::HashMap;

use super::alias::{self, ALLOWLIST, DIES, SECTIONS};
use super::loader::ConfigError;

/// Sweep the reserved `EHRBASE_` namespace: every such variable must be an
/// allowlisted non-config name, a known-section key, or a registered alias —
/// anything else is a boot error (with a did-you-mean), which is what makes a
/// set-but-never-read variable (the historical C-3/C-5 defect class) impossible.
/// Deeper key typos inside a known section are caught at deserialize by
/// `deny_unknown_fields`.
#[must_use]
pub fn strict_env(env: &HashMap<String, String>) -> Vec<ConfigError> {
    let mut errors = Vec::new();
    for key in env.keys() {
        if !key.starts_with("EHRBASE_") || ALLOWLIST.contains(&key.as_str()) {
            continue;
        }
        if let Some((_, msg)) = DIES.iter().find(|(d, _)| *d == key) {
            errors.push(ConfigError::dies(key, msg));
            continue;
        }
        // A legacy alias is handled (warned + remapped) by the assembler.
        if alias::resolve_alias(key).is_some() {
            continue;
        }
        // New-form variable: the leading section must be one of the known
        // eighteen. `EHRBASE_SERVER__BIND` → section `server`.
        let tail = key.strip_prefix("EHRBASE_").unwrap_or_default();
        let section = tail
            .split("__")
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if section.is_empty() || !SECTIONS.contains(&section.as_str()) {
            errors.push(ConfigError::unknown_env(
                key,
                did_you_mean(&section, SECTIONS),
            ));
        }
    }
    // Deterministic order for reproducible error blocks + tests.
    errors.sort_by_key(ToString::to_string);
    errors
}

/// The closest candidate within edit distance 2 (ties broken lexicographically),
/// or `None` when nothing is close enough.
#[must_use]
pub fn did_you_mean(unknown: &str, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .map(|c| (*c, damerau_levenshtein(unknown, c)))
        .filter(|(_, d)| *d <= 2)
        .min_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(b.0)))
        .map(|(c, _)| c.to_owned())
}

/// Optimal-string-alignment (restricted Damerau-Levenshtein) distance: single
/// insert/delete/substitute + adjacent transposition. ~byte-level; the config
/// keys are ASCII.
#[must_use]
pub fn damerau_levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<u8> = a.bytes().collect();
    let b: Vec<u8> = b.bytes().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev2 = vec![0usize; m + 1];
    let mut prev = (0..=m).collect::<Vec<_>>();
    let mut cur = vec![0usize; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut val = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                val = val.min(prev2[j - 2] + 1);
            }
            cur[j] = val;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_basics() {
        assert_eq!(damerau_levenshtein("signing", "signing"), 0);
        assert_eq!(damerau_levenshtein("signin", "signing"), 1);
        assert_eq!(damerau_levenshtein("signign", "signing"), 1); // transposition
        assert_eq!(damerau_levenshtein("telemetry", "db"), 8);
    }

    #[test]
    fn suggestion_finds_close_section() {
        assert_eq!(did_you_mean("signin", SECTIONS).as_deref(), Some("signing"));
        assert_eq!(did_you_mean("aut", SECTIONS).as_deref(), Some("auth"));
        assert_eq!(did_you_mean("completely_bogus", SECTIONS), None);
    }

    #[test]
    fn sweep_flags_unknown_and_dies_and_passes_known() {
        let mut env = HashMap::new();
        env.insert("EHRBASE_SERVER__BIND".to_owned(), "0.0.0.0:9".to_owned());
        env.insert("EHRBASE_SIGNIN__ENABLED".to_owned(), "true".to_owned());
        env.insert("EHRBASE_SIGNING_CONFIG".to_owned(), "/x.toml".to_owned());
        env.insert("EHRBASE_DB_MAX_CONNECTIONS".to_owned(), "5".to_owned()); // legacy alias, ok
        env.insert("EHRBASE_CONFIG".to_owned(), "/e.toml".to_owned()); // allowlisted
        let errs = strict_env(&env);
        let joined = errs
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("EHRBASE_SIGNIN__ENABLED"), "{joined}");
        assert!(joined.contains("signing"), "did-you-mean: {joined}");
        assert!(joined.contains("EHRBASE_SIGNING_CONFIG"), "{joined}");
        assert_eq!(errs.len(), 2, "only the typo + the dies var: {joined}");
    }
}
