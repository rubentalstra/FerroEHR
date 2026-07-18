//! The card surface classes: the design-system replacement for widget-kit
//! cards in static chrome. One look — token colors, hairline border, the
//! single soft shadow level.

/// The card surface (no padding — content decides).
pub const CARD: &str = "rounded-card border border-edge bg-raised shadow-card";

/// The card surface with the standard padding.
pub const CARD_PAD: &str = "rounded-card border border-edge bg-raised shadow-card p-4";

/// A sunken well (code panes, read-only documents).
pub const WELL: &str = "rounded-card border border-edge bg-sunken p-3";

/// The standard section heading inside a card.
pub const CARD_TITLE: &str = "text-sm font-semibold text-ink mb-3";
