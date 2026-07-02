<!-- .github/pull_request_template.md -->
## What this changes

<!-- Describe the change itself. Do not include any AI/tool attribution. -->

## Phase

- Phase: <!-- e.g. P03 (RM transcription) -->
- Phase file updated: [ ] `docs/plans/phase-NN-*.md` checkboxes ticked

## Port fidelity (Stage 1)

- [ ] Ported `.rs` mirror the source Java structure (names, order, control flow)
- [ ] Every ported/transcribed file has a `// PORT STATUS` trailer
- [ ] Annotations used where relevant: `TODO(port)` / `PERF(port)` / `PORT NOTE:` / `SAFETY:`
- [ ] No `.java` file was edited unless its Rust counterpart is complete; no Maven build files touched
- [ ] No test was weakened, skipped, or edited to route around a bug

## Checks

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy` (or noted why it cannot pass yet — allowed in P1–P16)
- [ ] `cargo nextest run` (or noted why it cannot pass yet — allowed in P1–P16)
- [ ] `cargo deny check`

<!--
HARD RULE: this PR description, its title, and all commits must contain NO
AI/Claude attribution (no "Co-authored-by: Claude", no "Generated with Claude
Code", no 🤖). Describe only the change.
-->
