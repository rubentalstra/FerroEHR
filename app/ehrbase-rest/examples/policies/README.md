# Example Cedar policies

Illustrative embedded-Cedar policies for the ehrbase-rs ABAC layer
(`docs/design/access-control.md` §5.6). Point `abac.cedar.policy_dir` at a
copy of this directory (or your own) to try the embedded engine.

`consent.cedar` reproduces the *shape* of EHRbase v1's external consent checks
(`has_consent_patient` + `has_consent_template`) against the shipped schema:

- **Principal** `User { organization?, patient?, roles, scopes }` — the caller.
- **Resource** (`Ehr`, `EhrStatus`, `Composition`, `Contribution`, `Query`,
  `Directory`) — carries the per-combination candidate `patient?` / `template?`.
- **Action** `"<kind>.<mode>"` — e.g. `composition.create`, `query.execute`.

## Semantics to know

- Cedar is **deny-by-default**; a request is permitted only if some `permit`
  matches and no `forbid` matches (`forbid` overrides `permit`).
- Multiple `permit` policies are **OR-combined**. EHRbase v1 required *all*
  configured policies to pass (logical AND), so the AND is expressed as a
  **single** `permit` whose `when` clause carries both checks — do the same when
  extending.
- Multi-valued attributes (contribution/query template sets, query patient sets)
  are fanned out by the engine as a cartesian product with **all-must-permit**;
  an empty set (empty query result) permits vacuously. This matches the remote
  PDP exactly (the differential test).

Replace the allow-list / consent bodies with your deployment's rule (or a set
backed by your own Cedar entity store).
