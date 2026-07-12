# A1 Spec Audit — Verify + Fix — chapter `rm-integration`

- **Chapter:** RM 1.2.0 integration (GENERIC_ENTRY, FEEDER_AUDIT)
- **Date:** 2026-07-11
- **Scope:** all 20 requirements `rm-integration-R1 … R20`
- **Result (defer-nothing pass):** 1 gap fixed (GENERIC_ENTRY had no
  standalone invariant dispatch); everything else verifies through the
  standing layers.

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1/R2 | verified | `GenericEntry.data: Item` (closed CLUSTER/ELEMENT enum) non-optional — fail-closed deserialize |
| R3/R4 | verified | the generated struct carries exactly `data` + the LOCATABLE inheritance |
| R5 | verified | `GenericEntry` is a `ContentItem` variant — accepted in `COMPOSITION.content` |
| R6 | verified | GENERIC_ENTRY is not a versioned-object `Kind` — it cannot be committed standalone, only inside a COMPOSITION |
| R7 | verified | every LOCATABLE struct carries `feeder_audit: Option<FeederAudit>` — a present value deserializes fail-closed |
| R8–R13 | verified | `FeederAudit` typed fields (`originating_system_audit: FeederAuditDetails` 1..1 monomorphic; `Vec<DvIdentifier>` lists; `original_content: Option<DvEncapsulated>` closed enum) |
| R14/R15 | verified | `system_id: String` non-optional; `feeder_audit_details_impl.rs` `System_id_valid` (dispatched) |
| R16–R19 | verified | typed optional fields (`PartyIdentified`, `PartyProxy` closed enum, `DvDateTime`, `ItemStructure`) |
| R20 | verified-behavioural | AQL queries stored nodes uniformly; GENERIC_ENTRY content participates as data without any silent promotion into designed-archetype semantics (no transformation is implied or performed) — the master02 caveat is an authoring/interop caution, not a wire rule |

## Fixes applied

- `GENERIC_ENTRY` dispatch arm + `generic_entry_impl.rs`
  (`Archetype_node_id_valid`) — the node previously had no standalone
  invariant run (its shape was only enforced via the COMPOSITION parent).

## Deferred

None.

## Uncertain / runtime probes

None remaining.
