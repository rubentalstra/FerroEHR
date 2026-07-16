# W-14 service rewrite — the design (owner mandate 2026-07-16)

The full rewrite of `app/ehrbase/src/service/` (+ the versioning core it
drives): modern clean Rust, zero trait-era legacy, SM-spec-cited structure,
the service-owned W-14 findings folded in. Method: big-bang per subsystem,
one convergence at the end (no intermediate stubs). Tracker: this file;
findings register: `w14-audit.md` §4/§5.

## Target shape

1. **One method per SM call.** The wrapper/inner `*_response` splits die.
   Each operation is ONE method whose signature is the SM call shape
   (cite the SM chapter §call) and whose return is a **typed result**, not
   the `ServiceResponse { data: Value, meta: Option<ResourceMeta> }`
   envelope:
   - writes → `Committed { version_uid: ObjectVersionId, vo_id, version,
     time_committed, .. }` (what the wire needs for `ETag`/`Location`/
     committal receipts, no JSON body unless representation is requested);
   - reads → the canonical `Value` plus a typed `VersionMeta` where the
     wire sets version headers;
   - the wire seam (tags-on-response, representation read-back) becomes
     explicit typed fields, not an `Option<ResourceMeta>` side channel.
2. **The full `UpdateVersion` envelope is honoured end-to-end (F-43).**
   Every direct write path (composition create/update/delete, EHR_STATUS,
   directory, demographic party/relationship) threads
   `UpdateVersion { data, audit, lifecycle_state, preceding_version_uid,
   signature }` into `versioning::{create,update,delete}`; caller-supplied
   audit attributes merge with the server rules (ITS-REST overview
   §"openehr-version and openehr-audit-details" MUST; RM common master06:
   `time_committed` is server-computed; master04 committer defaulting).
   The intentionally-red `protocol_tail::committal_headers_merge_into_the_commit`
   goes green here.
3. **F-1 — the signing fold.** `versioning/change.rs`: assign
   `time_committed` **app-side** (the Rust process is the server — RM common
   master06 §Contributions), compute the digest/signature before any
   statement, and ALWAYS commit through the one-CTE
   `commit_new_version`/`commit_version_into` fold. The split path
   (`write_contribution` → sign → `insert_vo_version`) is deleted.
4. **F-2 — the placement trio.** `lineage_tip` + `next_ordinal` become one
   statement; the advisory lock stays; evaluate folding
   `close_ordinal_at_now` into the commit CTE (the partial-unique index
   makes ordering load-bearing — verify with a concurrency test).
5. **F-4** — the persistent-duplicate check is ONE `EXISTS` query (no
   per-vo reassembly; PORT NOTE cites CNF `create_composition-same_opt_twice`
   as SEC-undecided).
6. **F-7** — `ehr_summary` is one merged query (ehr ⋈ status-meta ⋈
   ehr_access ref ⋈ folder refs), not 4 serial round-trips.
7. **F-24** — contribution commit: pre-tx `require_kind` reads batched
   (`= ANY($1)`), the in-tx per-version `first_version_root` reads folded
   into one statement; per-version signing now free with F-1.
8. **F-37** — party_relationship + demographic contribution writes return
   typed committed results (no post-write reassembly read-back).
9. **Structure**: `service/mod.rs` slims to the `EhrbaseService` struct +
   builders; `ServiceError` gets its own file; `CommitEnv` moves to
   `versioning` (it is the commit contract); every module doc cites its SM
   chapter; stale trait-era comments ("method-resolution priority",
   adapter-speak) deleted; the mass-inserted generic `# Errors` docs get
   real per-method text as each file is rewritten.
10. **REST adapter converges** on the typed results (header building from
    `Committed`/`VersionMeta` instead of digging `ResourceMeta`), and the
    HTTP suite stays byte-identical on the wire (ECC zero-drift is the
    gate; the B+C baseline receipt is the comparison point).

## Execution order (each lands big-bang, converge once at the end)

- [x] S1 versioning core (2026-07-16): F-1 — ONE folded commit path (app-known
      tx timestamp via the placement read / `tx_now`, app-generated
      contribution id, signature computed before any insert; the split path
      and `insert_vo_version` deleted); F-2 — placement trio merged into one
      statement carrying `now()`; F-24 — contribution target pre-reads
      batched (`object_kinds`, `= ANY`); F-43 plumbing — `WriteEnvelope` on
      every direct helper, `AuditInput::from_update` (ITS-REST committal
      MUST merge), lifecycle 553 relaxes validation, verbatim signature +
      attestations threaded on composition create/update + status update.
- [ ] S2 common types: typed results replace `ServiceResponse`/
      `ResourceMeta`; `service/mod.rs` split; `CommitEnv` relocation.
- [ ] S3 chapter `ehr` — PARTIAL: F-4 done (one indexed EXISTS over the
      promoted `template_id`), F-7 done (`ehr_summary_read`, one statement),
      composition/status envelope threading done. OPEN: the wrapper/inner
      `*_response` merges, directory envelope threading, DELETE-path
      committal headers, real per-method docs, stale-comment scrub.
- [ ] S4 chapters — PARTIAL: F-37 done (relationship writes respond from the
      in-hand body). OPEN: party/relationship envelope threading, the
      wrapper/inner merges and cleanup across demographic, definition,
      query, terminology, admin, message, ehr_index, subject_proxy,
      validity.
- [ ] S5 REST adapter convergence + test convergence.
- [ ] S6 gates: workspace clippy/nextest green, fmt, ECC **zero-drift vs
      the B+C receipt**, register re-anchor pass (`w14-audit.md`
      post-rewrite item), fresh benchmark pair at W-14 close.
