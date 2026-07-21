//! Case-scoped execution state: `requires` handles, captures (scalar and
//! list), and committed audit instants — plus the FIXED temporal resolution
//! rules so two runners query identical instants (interpreter law d:
//! before = t − 1 ms, after = t + 1 ms, between = the midpoint).

use std::collections::BTreeMap;

use crate::ids::CaptureName;
use crate::refgrammar::TimeExpr;

/// A captured value.
#[derive(Debug, Clone, PartialEq)]
pub enum Captured {
    /// A scalar capture (uids, ids, header values).
    Scalar(String),
    /// A list capture (`created.version_uids[]`).
    List(Vec<String>),
    /// A full response body (`ok.body`).
    Body(serde_json::Value),
    /// A committed audit instant, milliseconds since the Unix epoch
    /// (`created.commit_time` — the anchor for temporal at-time cases).
    InstantMs(i64),
}

/// The per-row variable store. Reset around every row under
/// `reset_per_row` (law a); carried across rows under `single_pass`.
#[derive(Debug, Clone, Default)]
pub struct VarStore {
    values: BTreeMap<CaptureName, Captured>,
}

impl VarStore {
    /// Bind or overwrite a capture.
    pub fn set(&mut self, name: CaptureName, value: Captured) {
        self.values.insert(name, value);
    }

    /// Look up a capture.
    #[must_use]
    pub fn get(&self, name: &CaptureName) -> Option<&Captured> {
        self.values.get(name)
    }

    /// A scalar view of a capture (lists and bodies are not scalars).
    #[must_use]
    pub fn scalar(&self, name: &CaptureName) -> Option<&str> {
        match self.values.get(name) {
            Some(Captured::Scalar(s)) => Some(s),
            _ => None,
        }
    }

    /// Resolve a temporal expression against captured instants (law d).
    ///
    /// # Errors
    /// Returns a message when a referenced capture is missing or not an
    /// instant, or the arithmetic overflows.
    pub fn resolve_time(&self, expr: &TimeExpr) -> Result<i64, String> {
        let instant = |name: &CaptureName| -> Result<i64, String> {
            match self.values.get(name) {
                Some(Captured::InstantMs(ms)) => Ok(*ms),
                Some(_) => Err(format!("capture {name} is not a commit instant")),
                None => Err(format!("capture {name} is not bound")),
            }
        };
        match expr {
            TimeExpr::Before(t) => instant(t)?
                .checked_sub(1)
                .ok_or_else(|| "instant arithmetic underflow".to_owned()),
            TimeExpr::After(t) => instant(t)?
                .checked_add(1)
                .ok_or_else(|| "instant arithmetic overflow".to_owned()),
            TimeExpr::Between(t1, t2) => {
                let (a, b) = (instant(t1)?, instant(t2)?);
                // midpoint without overflow: a + (b - a) / 2
                b.checked_sub(a)
                    .and_then(|d| a.checked_add(d / 2))
                    .ok_or_else(|| "instant arithmetic overflow".to_owned())
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    #[test]
    fn temporal_law_d_is_fixed() {
        let mut store = VarStore::default();
        let t1 = CaptureName::parse("t1").unwrap();
        let t2 = CaptureName::parse("t2").unwrap();
        store.set(t1.clone(), Captured::InstantMs(1_000));
        store.set(t2.clone(), Captured::InstantMs(2_001));

        assert_eq!(
            store.resolve_time(&TimeExpr::Before(t1.clone())).unwrap(),
            999
        );
        assert_eq!(
            store.resolve_time(&TimeExpr::After(t1.clone())).unwrap(),
            1_001
        );
        assert_eq!(
            store
                .resolve_time(&TimeExpr::Between(t1.clone(), t2))
                .unwrap(),
            1_500 // midpoint, integer division
        );
        assert!(
            store
                .resolve_time(&TimeExpr::Before(CaptureName::parse("ghost").unwrap()))
                .is_err()
        );
    }
}
