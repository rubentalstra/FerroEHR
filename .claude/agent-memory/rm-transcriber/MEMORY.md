# rm-transcriber memory index

- [time_types transcription precedent](project_time_types_precedent.md) — the Iso8601_type multiple-inheritance worked example (ADR-001 §2): Temporal as ordinary supertrait vs Time_Definitions as a non-trait constants struct, Iso8601TypeCore embedding, string-value-not-instant modeling, jiff-bridge deferral.
- [serde _type tag pitfall](feedback_serde_type_tag_pitfall.md) — #[serde(rename)] on a struct does NOT inject a _type key; only #[serde(tag = "_type")] on the enclosing enum does. Verify before annotating.
- [unwired lib.rs masks bugs](project_unwired_lib_rs_masks_bugs.md) — openehr-base/openehr-rm lib.rs have no `pub mod` lines yet; cargo check proves nothing about the code inside. A separate branch (claude/phase-04-serialization-json) already has a full P4 wiring pass.
