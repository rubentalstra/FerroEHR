#!/usr/bin/env bash
# Generate the Rust-dependency OpenVEX document.
#
#   security/vex/rust-advisories.toml  (the reasoning)
# + deny.toml [advisories].ignore      (the authoritative id set)
# → security/vex/rust-advisories.openvex.json
#
# The ids come from the advisory GATE, never from the prose file, so the
# published document and the thing that actually fails a build cannot drift
# apart: this script refuses to emit anything if the two sets disagree in
# either direction, or if a lock-file-only entry has been added to the gate's
# ignore list.
#
# Output is byte-deterministic — statements sorted by advisory id, the document
# timestamp taken from the TOML rather than the clock — so the drift check can
# regenerate and diff.
#
# OpenVEX specification: https://github.com/openvex/spec/blob/main/OPENVEX-SPEC.md
#
# Usage: scripts/security/vex-generate.sh [--write | --stdout]
#   --write   (default) overwrite security/vex/rust-advisories.openvex.json
#   --stdout  print the document instead, changing nothing on disk
set -euo pipefail
cd "$(dirname "$0")/../.."

readonly PROSE='security/vex/rust-advisories.toml'
readonly GATE='deny.toml'
readonly OUT='security/vex/rust-advisories.openvex.json'

mode="${1:---write}"
case "$mode" in
  --write | --stdout) ;;
  *)
    echo "usage: $0 [--write | --stdout]" >&2
    exit 2
    ;;
esac

for tool in yq jq; do
  command -v "$tool" >/dev/null || { echo "vex-generate: $tool is required" >&2; exit 1; }
done
for required in "$PROSE" "$GATE"; do
  [ -f "$required" ] || { echo "vex-generate: missing $required" >&2; exit 1; }
done

# The controlled vocabularies, so a typo in the prose file fails here instead of
# producing a document a consumer's parser rejects.
readonly VALID_STATUS='not_affected affected fixed under_investigation'
readonly VALID_JUSTIFICATION='component_not_present vulnerable_code_not_present vulnerable_code_not_in_execute_path vulnerable_code_cannot_be_controlled_by_adversary inline_mitigations_already_exist'

prose_json="$(yq -p toml -o json '.' "$PROSE")"
gate_ids="$(yq -p toml -o json '[.advisories.ignore[].id]' "$GATE" | jq -r '.[]' | sort)"
accepted_ids="$(jq -r '(.accepted // [])[].id' <<<"$prose_json" | sort)"
lockfile_ids="$(jq -r '(.lockfile_only // [])[].id' <<<"$prose_json" | sort)"

fail=0
note() { echo "vex-generate: $*" >&2; fail=1; }

# ── the two-way agreement that makes one list impossible to forget ──────────
while read -r id; do
  [ -n "$id" ] || continue
  note "deny.toml accepts $id but $PROSE has no [[accepted]] entry for it — every accepted advisory needs a published justification"
done < <(comm -23 <(echo "$gate_ids") <(echo "$accepted_ids"))

while read -r id; do
  [ -n "$id" ] || continue
  note "$PROSE claims $id is accepted but deny.toml does not ignore it — a VEX statement for an advisory the gate still fails on is a false claim"
done < <(comm -13 <(echo "$gate_ids") <(echo "$accepted_ids"))

# A lock-file-only entry that has been added to the gate is either a real
# acceptance (move it to [[accepted]]) or the ignore that deny.toml's header
# explicitly refuses to carry.
while read -r id; do
  [ -n "$id" ] || continue
  note "$id is listed as [[lockfile_only]] but deny.toml now ignores it — move it to [[accepted]], or drop the ignore"
done < <(comm -12 <(echo "$gate_ids") <(echo "$lockfile_ids"))

# ── vocabulary + completeness of every statement ────────────────────────────
while read -r entry; do
  [ -n "$entry" ] || continue
  id="$(jq -r '.id // ""' <<<"$entry")"
  for field in crate status justification impact; do
    value="$(jq -r --arg f "$field" '.[$f] // ""' <<<"$entry")"
    [ -n "$value" ] || note "${id:-<no id>}: missing '$field'"
  done
  status="$(jq -r '.status // ""' <<<"$entry")"
  justification="$(jq -r '.justification // ""' <<<"$entry")"
  grep -qw -- "$status" <<<"$VALID_STATUS" \
    || note "$id: status '$status' is not an OpenVEX status ($VALID_STATUS)"
  if [ "$status" = "not_affected" ]; then
    grep -qw -- "$justification" <<<"$VALID_JUSTIFICATION" \
      || note "$id: justification '$justification' is not in the OpenVEX vocabulary ($VALID_JUSTIFICATION)"
  fi
done < <(jq -c '((.accepted // []) + (.lockfile_only // []))[]' <<<"$prose_json")

[ "$fail" -eq 0 ] || exit 1

document="$(jq -S '
  .document as $d
  | (((.accepted // []) + (.lockfile_only // [])) | sort_by(.id)) as $entries
  | {
      "@context": "https://openvex.dev/ns/v0.2.0",
      "@id": $d.id,
      author: $d.author,
      role: $d.role,
      timestamp: $d.timestamp,
      version: $d.version,
      tooling: "scripts/security/vex-generate.sh from security/vex/rust-advisories.toml + deny.toml",
      statements: [
        $entries[] | . as $e | {
          vulnerability: ({ name: $e.id }
            + (if ($e.aliases // []) | length > 0 then { aliases: $e.aliases } else {} end)),
          products: [
            $d.products[] | { "@id": ., subcomponents: [ { "@id": ("pkg:cargo/" + $e.crate) } ] }
          ],
          status: $e.status,
          justification: $e.justification,
          impact_statement: $e.impact,
        }
      ],
    }
' <<<"$prose_json")"

# jq -S sorts object keys alphabetically, which is what makes the output
# byte-stable across jq versions; the statement ORDER is fixed above by id.
if [ "$mode" = "--stdout" ]; then
  printf '%s\n' "$document"
else
  printf '%s\n' "$document" > "$OUT"
  echo "ok: wrote $OUT ($(jq '.statements | length' <<<"$document") statements)"
fi
