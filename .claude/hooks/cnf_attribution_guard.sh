#!/usr/bin/env bash
# .claude/hooks/cnf_attribution_guard.sh
#
# Claude Code PreToolUse hook (matcher: Write|Edit|NotebookEdit). NON-BLOCKING
# reminder fired when a CNF catalogue EXPECTATION artifact is edited
# (tools/cnf-runner/artifacts/{schedule,bindings,vocab}/**). It injects the
# attribution law as additionalContext so the agent self-checks — at the exact
# moment of the edit — that it is correcting the catalogue toward the SPEC, not
# bending an expectation to match observed SUT behaviour.
#
# WHY: the recurring failure mode is "our code is right, so the CNF must be
# wrong" → editing the catalogue to make a red row green. This guard keeps the
# spec-oracle discipline (.claude/rules/cnf-triage.md) present precisely where
# that mistake happens. It never blocks (legitimate spec-cited catalogue fixes
# and new coverage cases are normal); it only reminds.
#
# Reads the tool-call JSON on stdin; prints hookSpecificOutput.additionalContext
# and exits 0. Corpus data and the ambiguity register are deliberately NOT
# guarded (corpus = data; registers/ = the sanctioned spec-silence path).

set -euo pipefail

payload="$(cat)"

if command -v jq >/dev/null 2>&1; then
  path="$(printf '%s' "$payload" | jq -r '.tool_input.file_path // .tool_input.notebook_path // empty' 2>/dev/null || true)"
else
  path="$payload"
fi
[ -n "${path:-}" ] || exit 0

case "$path" in
  */tools/cnf-runner/artifacts/schedule/* | tools/cnf-runner/artifacts/schedule/* | \
  */tools/cnf-runner/artifacts/bindings/* | tools/cnf-runner/artifacts/bindings/* | \
  */tools/cnf-runner/artifacts/vocab/*    | tools/cnf-runner/artifacts/vocab/*) ;;
  *) exit 0 ;;
esac

msg="CNF attribution law (.claude/rules/cnf-triage.md): you are editing a CNF catalogue expectation. The vendored spec (docs/specs/openehr/) is the ONLY oracle and the application is NEVER assumed correct. This edit is valid ONLY if it moves the catalogue toward the SPEC with a first-hand spec citation — it must NOT bend an expected status/header/outcome/value to match what the SUT returned. If the three-way comparison (spec-required vs catalogue-expected vs SUT-observed) shows the SUT is wrong, the fix belongs in app/* or the openehr-codegen emitter, NOT the catalogue. Spec silence -> artifacts/registers/ambiguities.yaml, never a bent expectation. THE CATALOGUE'S OWN COMMENTS ARE NOT GROUND: a case/binding comment may cite the STALLED OAS (e.g. 400_CONTRIBUTION, *_EHR.yaml, Error.yaml, *.openapi.yaml, responses/*, UpdateVersion) — IGNORE it. Open the ITS-REST DOCS TEXT (docs/specs/openehr/ITS-REST/specifications/docs/) and QUOTE the decisive sentence that assigns THIS value. 'The overview permits X generally' is the RATIONALIZATION TELL, not a docs-text assignment — if you cannot quote a docs-text sentence that assigns the value, it is SPEC-SILENT: register it, do NOT add an alt_status/expectation. STOP and open the docs text; do not talk past this."

if command -v jq >/dev/null 2>&1; then
  jq -n --arg m "$msg" '{hookSpecificOutput: {hookEventName: "PreToolUse", additionalContext: $m}}'
else
  printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"%s"}}' "$msg"
fi
exit 0
