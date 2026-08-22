// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The shared form-control classes: ONE styled definition for text inputs,
//! selects, and textareas (previously hand-duplicated per screen).
//!
//! These are class constants rather than wrapper components because half the
//! console's inputs are deliberately uncontrolled (the login form) or carry
//! bespoke wiring (the builder's per-datatype editors) — the kit standardizes
//! the LOOK, each screen keeps its own behaviour.

/// A single-line text input.
pub const INPUT: &str = "rounded-control border border-edge-strong bg-raised px-3 py-1.5 text-sm text-ink placeholder:text-ink-faint focus:outline-none focus:ring-2 focus:ring-accent focus:border-accent";

/// A `<select>` control.
pub const SELECT: &str = "rounded-control border border-edge-strong bg-raised px-2 py-1.5 text-sm text-ink focus:outline-none focus:ring-2 focus:ring-accent focus:border-accent";

/// A multi-line `<textarea>` (AQL editor, parameter bindings). A disabled one
/// dims exactly like the disabled buttons below — an edit form is inert until
/// the document it edits has been loaded into it.
pub const TEXTAREA: &str = "w-full rounded-control border border-edge-strong bg-raised px-3 py-2 font-mono text-xs text-ink placeholder:text-ink-faint focus:outline-none focus:ring-2 focus:ring-accent focus:border-accent disabled:opacity-50 disabled:pointer-events-none";

/// A form label.
pub const LABEL: &str = "text-sm font-medium text-ink";

/// The primary (solid accent) button.
pub const BTN_PRIMARY: &str = "inline-flex items-center gap-1.5 rounded-control bg-accent px-3 py-1.5 text-sm font-medium text-on-accent hover:bg-accent-hover focus:outline-none focus:ring-2 focus:ring-accent focus:ring-offset-1 disabled:opacity-50 disabled:pointer-events-none";

/// The secondary (outlined) button.
pub const BTN_SECONDARY: &str = "inline-flex items-center gap-1.5 rounded-control border border-edge-strong bg-raised px-3 py-1.5 text-sm font-medium text-ink hover:bg-sunken focus:outline-none focus:ring-2 focus:ring-accent disabled:opacity-50 disabled:pointer-events-none";

/// The quiet/destructive text button (delete affordances, two-step confirms).
pub const BTN_DANGER: &str = "inline-flex items-center gap-1.5 rounded-control border border-danger/40 px-3 py-1.5 text-sm font-medium text-danger hover:bg-danger-subtle focus:outline-none focus:ring-2 focus:ring-danger disabled:opacity-50 disabled:pointer-events-none";
