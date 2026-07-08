# SM-platform design set — the openEHR Service Model, end to end

The complete design that aligns ehrbase-rs with the **openEHR SM component**
(Service Model): the abstract Platform Service Model, the Simplified
Information Model (SIM-B), and the Serial Data Formats (SDF). Source oracle:
`docs/specs/openehr/SM/` (pinned commit in its `PROVENANCE.md`); every
statement in these docs carries a file + section citation there.

Owner rulings baked in (2026-07-08):
- **Full coverage — nothing deferred.** All ten SM platform services are in
  scope, including `EHR_EXTRACT` (Message service), TDD, Subject Proxy,
  Terminology surface, EHR Index, and the full Admin set.
- SM governs internal decomposition + call semantics; **ITS-REST 1.0.3 + CNF
  remain the wire oracle** (SM is TRIAL/DEVELOPMENT). Conflicts resolve to
  the wire spec with a `// PORT NOTE:` + citation.

## Reading order

| Doc | Content |
|---|---|
| [01-platform-foundations.md](01-platform-foundations.md) | Spec identity/status, the ten services, global conventions (CQS, transactionality, error model, paging, naming), common package (`I_STATUS`, `CALL_STATUS*`, `UPDATE_VERSION`/`UPDATE_AUDIT`, `I_VALIDITY_CHECKER`), Definitions service, spec defects |
| [02-ehr-service.md](02-ehr-service.md) | `I_EHR_SERVICE` / `I_EHR` / `I_EHR_STATUS` / `I_EHR_DIRECTORY` / `I_EHR_COMPOSITION` / `I_EHR_CONTRIBUTION`, `EHR_SUMMARY`, `UV_*`, versioning/audit/contribution semantics |
| [03-demographic-ehr-index-query.md](03-demographic-ehr-index-query.md) | Demographic (`I_PARTY`, `I_PARTY_RELATIONSHIP`), EHR Index (`I_EHR_INDEX`, `RESOURCE_STATUS`), Query (`I_QUERY_SERVICE`, execute specs, `RESULT_SET` family) |
| [04-message-subject-proxy-terminology-admin.md](04-message-subject-proxy-terminology-admin.md) | Message (`I_EHR_EXTRACT_SERVICE`, `I_TDD_SERVICE`), Subject Proxy (full variable/data-set/binding/sample model), Terminology (`I_TERMINOLOGY_SERVICE` + extract classes), Admin (`I_ADMIN_*`), System Log (IHE ATNA) |
| [05-simplified-im.md](05-simplified-im.md) | SIM-B: simplification principles, every `S_*` class, `APP_CONTEXT` (the `ctx/` vocabulary), transformation rules, documented lossiness |
| [06-serial-data-formats.md](06-serial-data-formats.md) | SDF: normative leaf-value/interval encodings, EhrScape variants, and the load-bearing absences (no path syntax, no MIME, `TBD` parser) |
| [07-gap-analysis.md](07-gap-analysis.md) | Every SM component vs the current service layer, per-call |
| [08-target-architecture.md](08-target-architecture.md) | The design: `ehrbase-sm` native-API crate, trait-per-SM-interface, shared types, new component designs (EHR Index, Terminology, Message/Extract/TDD, Subject Proxy, Admin), unified error table, wire exposure |
| [09-roadmap.md](09-roadmap.md) | Build order SM-1…SM-6 interleaved with P17–P20, verification gates |

Decision record: `docs/ADRs/ADR-010-sm-aligned-service-architecture.md`.
Execution tracking: `docs/plans/sm-phase-*.md` (created per phase, SM-1
first).

## The one-paragraph summary

The SM Platform Service Model is the openEHR-official decomposition of a CDR
into named components with formally specified interface calls
(pre/post-conditions, transactional semantics, an error model, and the
`UPDATE_VERSION`/`UPDATE_AUDIT` commit envelope). ehrbase-rs already realizes
its EHR core faithfully but implicitly; this design makes the realization
explicit (an `ehrbase-sm` trait layer as the SM "native API", with
`ehrbase-rest` as one protocol adapter) and completes the platform: the
remaining Definitions calls, PARTY_RELATIONSHIP, EHR Index, the Terminology
surface, the Message service (EHR_EXTRACT + TDD), the full Admin set
(statistics, archive, dump/load), and the Subject Proxy service — with the
SIM-B/SDF specs anchoring the FLAT/`ctx` work in P17.
