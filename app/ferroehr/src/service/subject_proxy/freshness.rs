// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Currency / freshness semantics (SM master10 §Samples: `effective_time` "is
//! comparable to `currency` in order to determine the freshness of the data";
//! `subject_variable.adoc` `currency`: "Required currency of this data item.
//! If not set, most recent available is valid").
//!
//! ISO-8601 durations with nominal parts (months/years) have no total order
//! in isolation; every comparison here is **anchored at the evaluation
//! instant** (`now - duration`), which is the only reading that makes
//! "freshness" decidable. No openEHR spec defines the anchoring — our own
//! documented realization.

use jiff::{Span, Timestamp, Zoned};

/// Parse a `SUBJECT_VARIABLE.currency` ISO-8601 duration (e.g. `PT2m`, `P1D`).
pub(super) fn parse_currency(s: &str) -> Result<Span, String> {
    s.parse::<Span>()
        .map_err(|e| format!("invalid ISO-8601 duration {s:?}: {e}"))
}

/// The freshness threshold `now - currency`, anchored at the evaluation
/// instant (nominal months/years resolve against the current civil date).
fn threshold(currency: &Span) -> Result<Timestamp, String> {
    Zoned::now()
        .checked_sub(*currency)
        .map(|z| z.timestamp())
        .map_err(|e| format!("currency out of range: {e}"))
}

/// Whether a sample stamped `sample_time` (ISO-8601; `effective_time` falling
/// back to `retrieve_time`) still satisfies `currency`.
pub(super) fn is_fresh(sample_time: &str, currency: &Span) -> bool {
    let Ok(at) = sample_time.parse::<Timestamp>() else {
        return false; // unparseable stamp: treat as stale (fail-closed)
    };
    threshold(currency).is_ok_and(|cutoff| at >= cutoff)
}

/// The tighter (lower) of two currencies, anchored at now — the
/// `register_application_data_set` rule: "reducing the currency of existing
/// subject variables, if the currency is lower in the corresponding data set
/// variable" (`i_subject_proxy_service.adoc`). `None` means "most recent
/// available is valid" — the loosest possible currency, so any concrete
/// candidate is lower.
pub(super) fn tighter_currency(existing: Option<&str>, candidate: Option<&str>) -> Option<String> {
    let Some(cand) = candidate else {
        return existing.map(str::to_owned);
    };
    let Some(exist) = existing else {
        return Some(cand.to_owned());
    };
    let (Ok(e), Ok(c)) = (parse_currency(exist), parse_currency(cand)) else {
        return Some(exist.to_owned()); // unparseable candidate: keep existing
    };
    match (threshold(&e), threshold(&c)) {
        // A LATER threshold = a shorter look-back window = lower currency.
        (Ok(te), Ok(tc)) if tc > te => Some(cand.to_owned()),
        _ => Some(exist.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_within_window_stale_outside() {
        let currency = parse_currency("PT10m").expect("parse");
        let now = Timestamp::now();
        assert!(is_fresh(&now.to_string(), &currency));
        let old = now - Span::new().hours(1);
        assert!(!is_fresh(&old.to_string(), &currency));
        assert!(!is_fresh("not-a-time", &currency), "unparseable = stale");
    }

    #[test]
    fn nominal_durations_parse_and_anchor() {
        // Nominal months anchor at the evaluation instant.
        let currency = parse_currency("P1M").expect("parse nominal month");
        assert!(is_fresh(&Timestamp::now().to_string(), &currency));
    }

    #[test]
    fn tighter_currency_takes_the_lower() {
        // Unset existing = loosest: any candidate tightens.
        assert_eq!(
            tighter_currency(None, Some("PT2m")),
            Some("PT2m".to_owned())
        );
        // Candidate unset: no change.
        assert_eq!(
            tighter_currency(Some("PT2m"), None),
            Some("PT2m".to_owned())
        );
        // PT2m is lower than P1D.
        assert_eq!(
            tighter_currency(Some("P1D"), Some("PT2m")),
            Some("PT2m".to_owned())
        );
        // P1D is not lower than PT2m.
        assert_eq!(
            tighter_currency(Some("PT2m"), Some("P1D")),
            Some("PT2m".to_owned())
        );
    }
}
