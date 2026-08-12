// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The `terminology` section (`ADL2/master07.13`): term definitions, term
//! bindings, and value sets. The body printer is shared with the OPT-only
//! `component_terminologies` block, which nests one body per component id.

use openehr_am::v2_4::aom2::terminology::archetype_terminology::ArchetypeTerminology;

use crate::print::Printer;
use crate::print::odin::quoted;

impl Printer {
    // ── terminology (master07.13) ──────────────────────────────────────────
    pub(super) fn terminology_section(&mut self, t: &ArchetypeTerminology) {
        self.blank();
        self.line(0, "terminology");
        self.terminology_body(t, 1);
    }

    pub(super) fn terminology_body(&mut self, t: &ArchetypeTerminology, depth: usize) {
        self.line(depth, "term_definitions = <");
        for (lang, codes) in &t.term_definitions {
            self.line(depth + 1, &format!("[{}] = <", quoted(lang)));
            for (code, term) in codes {
                self.line(depth + 2, &format!("[{}] = <", quoted(code)));
                self.line(depth + 3, &format!("text = <{}>", quoted(&term.text)));
                self.line(
                    depth + 3,
                    &format!("description = <{}>", quoted(&term.description)),
                );
                if let Some(items) = &term.other_items {
                    for (k, v) in items {
                        self.line(depth + 3, &format!("{k} = <{}>", quoted(v)));
                    }
                }
                self.line(depth + 2, ">");
            }
            self.line(depth + 1, ">");
        }
        self.line(depth, ">");
        if let Some(bindings) = &t.term_bindings {
            self.line(depth, "term_bindings = <");
            for (terminology, map) in bindings {
                self.line(depth + 1, &format!("[{}] = <", quoted(terminology)));
                for (key, uri) in map {
                    self.line(depth + 2, &format!("[{}] = <{uri}>", quoted(key)));
                }
                self.line(depth + 1, ">");
            }
            self.line(depth, ">");
        }
        if let Some(value_sets) = &t.value_sets {
            self.line(depth, "value_sets = <");
            for (id, vs) in value_sets {
                self.line(depth + 1, &format!("[{}] = <", quoted(id)));
                self.line(depth + 2, &format!("id = <{}>", quoted(&vs.id)));
                let members = vs
                    .members
                    .iter()
                    .map(|m| quoted(m))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.line(depth + 2, &format!("members = <{members}>"));
                self.line(depth + 1, ">");
            }
            self.line(depth, ">");
        }
    }
}
