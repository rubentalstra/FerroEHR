# Spec-conformance-reviewer memory index

- [CDR extension surfaces — recurring checks](cdr-extension-surfaces.md) — what to verify on "our own extension" REST surfaces (flag discipline, error mapping, ABAC mode, endpoint-map SQL trace accuracy)
- [Error status mapping chain](error-status-mapping-chain.md) — ServiceError→SmError→HTTP table location + FK-violation classification path
- [CNF schedule → catalogue conversion review](cnf-schedule-conversion-review.md) — case-model completeness pitfalls: master08 multi-version-per-commit (unrepresentable), master09 directory temporal/tree, master06 1.a/1.b, verified_by ids, SF MIME reject-list; OAS status sets that check out
- [ADL2 temporal mixed pattern+interval](adl2-temporal-mixed-pattern.md) — the `pattern/interval` form is DURATION-ONLY; emitting it for C_DATE/C_TIME/C_DATE_TIME is non-reparseable; watch the OPT-1.4→ADL2 converter carrying both pattern+range
