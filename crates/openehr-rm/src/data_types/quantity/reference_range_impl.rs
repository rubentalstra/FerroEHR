//! Hand-written RM class invariant (ADR-003) for `REFERENCE_RANGE`.
//!
//! `Range_is_simple` (archie `ReferenceRange`): each present, bounded limit of
//! the `range` must be a *simple* `DV_ORDERED` — one that itself carries no
//! `normal_range` and no `other_reference_ranges` (so reference ranges do not
//! nest).

use crate::data_types::quantity::dv_ordered::DvOrdered;
use crate::data_types::quantity::reference_range::ReferenceRange;
use crate::validate::{InvariantViolation, Validate};

/// A `DV_ORDERED` is "simple" when it declares neither a normal range nor other
/// reference ranges (archie `DvOrdered.isSimple`).
fn is_simple(o: &DvOrdered) -> bool {
    match o {
        DvOrdered::DvCount(x) => x.normal_range.is_none() && x.other_reference_ranges.is_empty(),
        DvOrdered::DvQuantity(x) => x.normal_range.is_none() && x.other_reference_ranges.is_empty(),
        DvOrdered::DvOrdinal(x) => x.normal_range.is_none() && x.other_reference_ranges.is_empty(),
        DvOrdered::DvScale(x) => x.normal_range.is_none() && x.other_reference_ranges.is_empty(),
        DvOrdered::DvProportion(x) => {
            x.normal_range.is_none() && x.other_reference_ranges.is_empty()
        }
        DvOrdered::DvDate(x) => x.normal_range.is_none() && x.other_reference_ranges.is_empty(),
        DvOrdered::DvDateTime(x) => x.normal_range.is_none() && x.other_reference_ranges.is_empty(),
        DvOrdered::DvDuration(x) => x.normal_range.is_none() && x.other_reference_ranges.is_empty(),
        DvOrdered::DvTime(x) => x.normal_range.is_none() && x.other_reference_ranges.is_empty(),
    }
}

fn limit_ok(unbounded: bool, limit: Option<&DvOrdered>) -> bool {
    unbounded || limit.is_some_and(is_simple)
}

impl Validate for ReferenceRange {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        let r = &self.range;
        if !(limit_ok(r.lower_unbounded, r.lower.as_ref())
            && limit_ok(r.upper_unbounded, r.upper.as_ref()))
        {
            out.push(InvariantViolation::here(
                "Invariant Range_is_simple failed on type REFERENCE_RANGE",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_interval::DvInterval;
    use crate::data_types::quantity::dv_quantity::DvQuantity;
    use crate::data_types::text::dv_text::{DvText, DvTextData};

    fn quantity(magnitude: f64) -> DvQuantity {
        DvQuantity {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            magnitude,
            precision: None,
            units: "kg".to_owned(),
            units_system: None,
            units_display_name: None,
        }
    }

    fn meaning() -> DvText {
        DvText::DvText(DvTextData {
            value: "normal".to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: Vec::new(),
            language: None,
            encoding: None,
        })
    }

    fn range_with(lower: DvQuantity, upper: DvQuantity) -> ReferenceRange {
        ReferenceRange {
            meaning: meaning(),
            range: DvInterval {
                lower: Some(DvOrdered::DvQuantity(lower)),
                upper: Some(DvOrdered::DvQuantity(upper)),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        }
    }

    #[test]
    fn simple_range_valid() {
        assert!(
            range_with(quantity(0.0), quantity(10.0))
                .invariants()
                .is_empty()
        );
    }

    #[test]
    fn nested_reference_range_invalid() {
        let mut top = quantity(10.0);
        // Make the upper limit non-simple by giving it its own normal range.
        top.normal_range = Some(Box::new(DvInterval {
            lower: Some(quantity(0.0)),
            upper: Some(quantity(10.0)),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        }));
        let v = range_with(quantity(0.0), top).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Range_is_simple failed on type REFERENCE_RANGE"),
            "got {v:?}"
        );
    }
}
