# Spec-conformance-reviewer memory index

- [CDR extension surfaces — recurring checks](cdr-extension-surfaces.md) — what to verify on "our own extension" REST surfaces (flag discipline, error mapping, ABAC mode, call chains re-derived from code)
- [Error status mapping chain](error-status-mapping-chain.md) — ServiceError→SmError→HTTP table location + FK-violation classification path
- [CNF schedule → catalogue conversion review](cnf-schedule-conversion-review.md) — case-model completeness pitfalls: master08 multi-version-per-commit (unrepresentable), master09 directory temporal/tree, master06 1.a/1.b, verified_by ids, SF MIME reject-list; OAS status sets that check out
- [ADL2 temporal mixed pattern+interval](adl2-temporal-mixed-pattern.md) — the `pattern/interval` form is DURATION-ONLY; emitting it for C_DATE/C_TIME/C_DATE_TIME is non-reparseable; watch the OPT-1.4→ADL2 converter carrying both pattern+range
- [CNF Profiles book shape](cnf-profiles-book-shape.md) — Profiles/master03 is NOT a pure capability×tier matrix (CORE=all/OPTIONS=any rule + External Data Format attr + Security&Privacy); no Enterprise family; Guide line 70 excludes performance only
- [ADL 1.4→2 VCOSU re-mint + depth-0 collapse](adl14-convert-vcosu-collapse.md) — verified: VCOSU archetype-wide id uniqueness re-mint, depth-0 collapse satisfies VARCN/VACSD/VASID/VATCD, conversion_details is real, the two XSD-invalid robot OPT fixtures
- [ITS-REST wire recurring defects](its-rest-wire-recurring-defects.md) — confirmed: ITEM_TAG key+target_path identity & VERSION target, named `query_parameters` GET form, split misc:: stored-query keying, creating_system_id dropped, ETag asymmetry, `/tags` not `/item_tag`
- [CNF content datatype adjudications](cnf-content-datatype-adjudications.md) — verified BASE Interval 4-invariant set (AMB-43), AOM1.4-vs-XSD temporal serialization gap (AMB-42), DV ordering, and the C_PRIMITIVE_OBJECT-has-no-attributes OPT pitfall + realizability-gate no-masking proof
