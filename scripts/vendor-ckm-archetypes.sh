#!/usr/bin/env bash
# Vendor the official openEHR CKM archetype library — the ADL 1.4 half of the
# two-dialect archetype corpus.
#
# Source: the public openEHR Clinical Knowledge Manager REST API
# (https://ckm.openehr.org/ckm/rest/v1). Each file is CKM's own export,
# vendored verbatim with provenance.
#
# WHICH DIALECT CKM SERVES (verified 2026-08-01 — do not re-guess):
#   * `GET /archetypes/{cid}/adl` -> ADL **1.4** text (`adl_version=1.4` in the
#     archetype header). This is the ONLY ADL CKM publishes.
#   * `GET /archetypes/{cid}/xml` -> the AM 1.4 ARCHETYPE XML of the same
#     archetype (opt-in here via --with-xml; roughly +40% bytes).
#   * There is NO ADL 2 export: `/adl2`, `/adl14`, `/adl2.4`, `/opt2` and
#     `/source` all 404, and `?format=ADL2` / `?version=2` are silently
#     ignored (byte-identical 1.4 response). The ADL 2.4 corpus therefore
#     comes from a DIFFERENT official source —
#     `scripts/vendor-adl2-archetypes.sh` (openEHR/adl-archetypes).
#     Never present a CKM export as ADL 2, and never fill the ADL 2 side by
#     running our own 1.4->2 converter over CKM output: that would test the
#     converter against itself.
#
# CKM REST PAGINATION GOTCHA: the list endpoints page with `?page=N&size=M`.
# `limit`, `pageSize`, `maxResults`, `offset`, `count` and `rows` are all
# silently IGNORED and you get a 20-row first page — which reads exactly like
# "CKM only publishes 20 archetypes". Always page with page/size and assert
# the count grew.
#
# Usage:
#   scripts/vendor-ckm-archetypes.sh                # ADL 1.4 texts
#   scripts/vendor-ckm-archetypes.sh --with-xml     # + the AM 1.4 XML twin
#   CKM_JOBS=8 scripts/vendor-ckm-archetypes.sh     # parallel (default 4)
set -Eeuo pipefail

CKM="https://ckm.openehr.org/ckm/rest/v1"
OUT="tools/cnf-runner/artifacts/corpus/archetypes/ckm"
JOBS="${CKM_JOBS:-4}"

# ── re-entrant single fetch (the xargs worker; not a user-facing mode) ────
if [[ "${1:-}" == "--fetch-one" ]]; then
  cid=$2
  dest=$3
  fmt=$4 # adl | xml
  for attempt in 1 2 3; do
    if curl -fsS --max-time 240 "$CKM/archetypes/$cid/$fmt" -o "$dest"; then
      if grep -qi "archetype" <<< "$(head -c 2048 "$dest")"; then
        echo "OK   $cid $dest"
        exit 0
      fi
      rm -f "$dest"
      echo "BAD  $cid $dest (response is not an archetype)"
      exit 0
    fi
    sleep $((attempt * 2))
  done
  rm -f "$dest"
  echo "FAIL $cid $dest"
  exit 0
fi

WITH_XML=0
[[ "${1:-}" == "--with-xml" ]] && WITH_XML=1

ADL_DIR="$OUT/adl14"
XML_DIR="$OUT/xml"
mkdir -p "$ADL_DIR"
[[ $WITH_XML == 1 ]] && mkdir -p "$XML_DIR"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
STAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

echo "==> listing the full CKM archetype library (page/size pagination)"
curl -fsS "$CKM/archetypes?page=0&size=10000" -H "Accept: application/json" \
  -o "$WORK/archetypes.json"

python3 - "$WORK/archetypes.json" "$WORK/jobs_adl.txt" "$WORK/jobs_xml.txt" \
  "$WORK/rows.tsv" "$ADL_DIR" "$XML_DIR" <<'PY'
import collections
import json
import sys

src, jobs_adl, jobs_xml, rows_path, adl_dir, xml_dir = sys.argv[1:7]
archetypes = json.load(open(src))
if len(archetypes) <= 20:
    raise SystemExit(
        f"::error::the list endpoint returned only {len(archetypes)} rows — "
        "CKM ignored the pagination parameters (use ?page=N&size=M)"
    )

# the archetype HRID is the natural, stable file name (openEHR-EHR-*.v1) —
# unlike templates, CKM archetypes carry it in resourceMainId
seen = collections.Counter()
adl, xml, rows = [], [], []
for a in sorted(archetypes, key=lambda x: x["resourceMainId"]):
    hrid = a["resourceMainId"]
    seen[hrid] += 1
    name = hrid if seen[hrid] == 1 else f"{hrid}__{seen[hrid]}"
    adl.append(f"{a['cid']} {adl_dir}/{name}.adl")
    xml.append(f"{a['cid']} {xml_dir}/{name}.xml")
    rows.append(
        "\t".join(
            (
                a["cid"],
                name,
                a["resourceMainDisplayName"].replace("|", "/"),
                a["status"],
                a["modificationTime"],
                str(a.get("revisionLatest") or ""),
            )
        )
    )

open(jobs_adl, "w").write("\n".join(adl) + "\n")
open(jobs_xml, "w").write("\n".join(xml) + "\n")
open(rows_path, "w").write("\n".join(rows) + "\n")
print(f"==> {len(rows)} archetypes published by CKM")
PY

echo "==> fetching ADL 1.4 texts (jobs=$JOBS)"
find "$ADL_DIR" -name '*.adl' -delete
xargs -P "$JOBS" -n 2 bash -c 'bash "$0" --fetch-one "$1" "$2" adl' "$0" \
  < "$WORK/jobs_adl.txt" | tee "$WORK/adl.log"

if [[ $WITH_XML == 1 ]]; then
  echo "==> fetching AM 1.4 archetype XML (jobs=$JOBS)"
  find "$XML_DIR" -name '*.xml' -delete
  xargs -P "$JOBS" -n 2 bash -c 'bash "$0" --fetch-one "$1" "$2" xml' "$0" \
    < "$WORK/jobs_xml.txt" | tee "$WORK/xml.log"
else
  : > "$WORK/xml.log"
fi

python3 - "$WORK/rows.tsv" "$WORK/adl.log" "$WORK/xml.log" \
  "$OUT/PROVENANCE.md" "$STAMP" "$CKM" "$WITH_XML" <<'PY'
import collections
import sys

rows_path, adl_log, xml_log, prov_path, stamp, ckm, with_xml = sys.argv[1:8]


def outcomes(path):
    out = {}
    for line in open(path):
        parts = line.split()
        if len(parts) >= 2 and parts[0] in {"OK", "BAD", "FAIL"}:
            out[parts[1]] = parts[0]
    return out


adl = outcomes(adl_log)
xml = outcomes(xml_log)
rows = [line.rstrip("\n").split("\t") for line in open(rows_path) if line.strip()]
ok = [r for r in rows if adl.get(r[0]) == "OK"]
bad = [r for r in rows if adl.get(r[0]) in {"BAD", "FAIL"}]
classes = collections.Counter(r[1].split(".")[0] for r in ok)
statuses = collections.Counter(r[3] for r in ok)

with open(prov_path, "w") as fh:
    w = fh.write
    w("# CKM archetype library (ADL 1.4) — provenance\n\n")
    w(f"Every archetype the official openEHR CKM (`{ckm}`) publishes, exported\n")
    w("by CKM itself and vendored verbatim by\n")
    w(f"`scripts/vendor-ckm-archetypes.sh` on {stamp}.\n\n")
    w("## Dialect\n\n")
    w("`adl14/` holds CKM's `GET /archetypes/{cid}/adl` response — **ADL 1.4**\n")
    w("text (`adl_version=1.4`). CKM publishes NO ADL 2 export (`/adl2`,\n")
    w("`/adl14`, `/opt2` 404; `?format=ADL2` is ignored and returns the same\n")
    w("1.4 bytes), so the **ADL 2.4 half of the corpus comes from\n")
    w("`scripts/vendor-adl2-archetypes.sh`** (openEHR/adl-archetypes). A CKM\n")
    w("export is never labelled ADL 2, and the ADL 2 side is never produced by\n")
    w("running our own 1.4->2 converter over these files — that would test the\n")
    w("converter against itself.\n\n")
    if with_xml == "1":
        w(f"`xml/` holds the AM 1.4 ARCHETYPE XML twin of the same {len(xml)} exports\n")
        w("(`GET /archetypes/{cid}/xml`), for the XML codec's read path.\n\n")
    else:
        w("The AM 1.4 ARCHETYPE XML twin (`GET /archetypes/{cid}/xml`) is NOT\n")
        w("vendored here; re-run with `--with-xml` to add it.\n\n")
    w("## Exercised, with adjudicated refusals\n\n")
    w("The pack is parsed 100% by\n")
    w("`crates/openehr-adl/tests/it/ckm_archetype_packs.rs`\n")
    w("(`ckm_adl14_pack_parses`). A file the conformant reader REFUSES is listed\n")
    w("in that gate's `ADJUDICATED_CKM_ADL14` table with the syntax code the\n")
    w("refusal must carry plus the spec ground for it, and the gate asserts the\n")
    w("refusal HAPPENS and carries that code — a refusal is a negative test, not\n")
    w("a skip. Never delete a refused file: that drops the negative case\n")
    w("(`.claude/rules/vendored-corpora.md`, `.claude/rules/testing.md`).\n\n")
    w("## Licensing\n\n")
    w("CKM publishes no repository-level license; each archetype carries its\n")
    w("own `description` > `licence` metadata (predominantly CC-BY-SA 3.0\n")
    w("where stated — see the individual file). Vendored verbatim, so the\n")
    w("authorship and licence metadata ride along in every file; root\n")
    w("reference copy: `LICENSE-CC-BY-SA-3.0`.\n\n")
    w("## Inventory\n\n")
    w(f"- published by CKM: **{len(rows)}**\n")
    w(f"- vendored: **{len(ok)}**\n")
    w(f"- unreachable: **{len(bad)}**\n\n")
    w("| RM class | count |\n|---|---|\n")
    for cls, n in sorted(classes.items(), key=lambda kv: (-kv[1], kv[0])):
        w(f"| {cls} | {n} |\n")
    w("\n| status | count |\n|---|---|\n")
    for st, n in sorted(statuses.items(), key=lambda kv: (-kv[1], kv[0])):
        w(f"| {st} | {n} |\n")
    w("\n")
    if bad:
        w("## Unreachable (recorded, not skipped)\n\n")
        w("CKM answers 404 for resources held in a private incubator; they are\n")
        w("only exportable by a signed-in account with access.\n\n")
        w("| cid | archetype | status |\n|---|---|---|\n")
        for cid, name, _disp, status, *_ in bad:
            w(f"| {cid} | {name} | {status} |\n")
        w("\n")
    w("## Vendored\n\n")
    w("| cid | archetype | display name | status | modified | revision |\n")
    w("|---|---|---|---|---|---|\n")
    for cid, name, disp, status, modified, revision in ok:
        w(f"| {cid} | `{name}` | {disp} | {status} | {modified} | {revision} |\n")

print(f"==> {len(ok)} ADL 1.4 archetypes vendored, {len(bad)} unreachable → {prov_path}")
if bad:
    print("    unreachable: " + ", ".join(r[0] for r in bad))
PY
