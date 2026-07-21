---
name: cnf-profiles-book-shape
description: What the CNF Profiles book (profiles/master03) actually contains — not a pure capability×tier matrix; plus the Guide non-functional exclusion nuance
metadata:
  type: project
---

CNF Profiles book (`docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc`)
is NOT reducible to a capability→family→tier matrix alone. It carries, beyond
the Functional capability×tier table:

- a **verdict-combination rule** in prose: CORE/STANDARD require ALL listed
  capabilities; OPTIONS is a catch-all obtained if ANY optional capability
  passes (line 9).
- a **Non-Functional** matrix: Security & Privacy → Signing (STANDARD),
  Anonymous EHRs (CORE/STANDARD).
- an **"Other Non-Functional"** table of a different shape entirely:
  Product Attribute → Values, `External Data Format: XML, JSON` — a
  tech-profile/format axis, not a capability→tier cell.

**Why:** any claim that "the Profiles prose regenerates from the capability
matrix" is overclaimed — the verdict semantics live in the verdict-rules
artifact, and External Data Format lives in the format/tech-profile axis, not
the capability matrix.

**How to apply:** when reviewing CNF-strategy claims about generating Profiles
prose from data, demand the same semantic-equivalence honesty caveat the
schedule-prose regeneration claim carries. Families present in the book:
Definitions, EHR+Persistence, Demographic, Querying, Admin, Messaging,
REST APIs, Security&Privacy — there is NO "Enterprise" family (D/M/X is a
2017-wiki proposal, absent from the current book).

Guide nuance: `guide/master03-overview.adoc` line 70 excludes non-functional
**performance** ("Non-functional conformance (performance, etc) is not
addressed by this guide") — but the Profiles book DOES carry non-functional
*attributes* (Security&Privacy, External Data Format). "Non-functional is
scoped out" is exact for performance, loose as a blanket. See
[[cnf-schedule-conversion-review]].
