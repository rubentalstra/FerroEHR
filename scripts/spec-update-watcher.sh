#!/usr/bin/env bash
# openEHR spec-update watcher (tracker issue #137; design dossier: recorded on tracker issue #137
# — every mechanism live-verified 2026-07-20). Two detection sources, ONE
# GitHub issue per completed upstream spec change:
#
#   A. Jira poll — anonymous search of openehr.atlassian.net for SPEC* issues
#      newly Resolved/Closed inside the window. Uses the enhanced
#      /rest/api/3/search/jql endpoint (token pagination; the classic
#      /rest/api/2/search is HTTP 410 GONE). `status in (Resolved, Closed)`
#      is the completion signal — statusCategory "done" over-captures
#      (In Review is green-category with resolution=null).
#   B. Amendment-record cross-check — the Jira keys present in each upstream
#      amendment record (default branch) but absent from the vendored copy
#      under docs/specs/openehr/ (paths mirror the upstream repos 1:1; pins
#      live in scripts/vendor-spec-docs.sh).
#
# Dedup / watermark: the issue board itself — a key already present in ANY
# issue title (open or closed, searched with `--state all`) is never
# re-filed; a closed issue means "already triaged/implemented". No state
# file, no repo variables.
#
# Failure honesty: any non-2xx (one bounded retry on 429), unexpected JSON
# shape, or missing amendment path exits non-zero — the run goes RED. Only a
# successful poll that genuinely matches nothing is green-with-zero.
#
# Env: DRY_RUN=1 (report, create nothing) · WINDOW_DAYS (default 14 — the scheduled cadence; run manually with a wide window, e.g. 365, to catch any backlog: dedup + the vendored baseline make that safe) ·
#      GH_TOKEN/GITHUB_TOKEN for gh. Requires curl, jq, gh.
set -euo pipefail
cd "$(dirname "$0")/.."

JIRA="https://openehr.atlassian.net"
WINDOW_DAYS="${WINDOW_DAYS:-14}"
DRY_RUN="${DRY_RUN:-0}"
[[ "$WINDOW_DAYS" =~ ^[0-9]{1,4}$ ]] ||
  { echo "spec-update-watcher: WINDOW_DAYS must be a number of days, got '$WINDOW_DAYS'" >&2; exit 1; }
# Field separator for the candidate rows: ASCII unit separator — non-whitespace,
# so empty fields survive `read` (tab-IFS collapses consecutive tabs).
US=$'\x1f'
# The load-bearing SPEC* projects: the ones with a vendored pin + a spec:*
# label, plus SPECPR (cross-cutting problem reports). SPECPROC/CDS/INTG/PUB
# have neither a pin nor a label and are deliberately excluded.
PROJECTS="SPECRM,SPECBASE,SPECAM,SPECLANG,SPECQUERY,SPECITS,SPECTERM,SPECSM,SPECCNF,SPECPR"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

for bin in curl jq gh; do
  command -v "$bin" >/dev/null 2>&1 || { echo "spec-update-watcher: $bin is required" >&2; exit 1; }
done

# ── component maps ───────────────────────────────────────────────────────────
label_for_project() {
  case "$1" in
    SPECRM) echo "spec:RM" ;; SPECBASE) echo "spec:BASE" ;; SPECAM) echo "spec:AM" ;;
    SPECLANG) echo "spec:LANG" ;; SPECQUERY) echo "spec:QUERY" ;; SPECITS) echo "spec:ITS" ;;
    SPECTERM) echo "spec:TERM" ;; SPECSM) echo "spec:SM" ;; SPECCNF) echo "spec:CNF" ;;
    *) echo "" ;; # SPECPR & friends: cross-component, spec-update only
  esac
}
component_for_project() {
  case "$1" in
    SPECRM) echo "RM" ;; SPECBASE) echo "BASE" ;; SPECAM) echo "AM" ;;
    SPECLANG) echo "LANG" ;; SPECQUERY) echo "QUERY" ;; SPECITS) echo "ITS-REST" ;;
    SPECTERM) echo "TERM" ;; SPECSM) echo "SM" ;; SPECCNF) echo "CNF" ;;
    *) echo "" ;;
  esac
}
label_for_component() { # vendored component dir -> label
  case "$1" in
    RM) echo "spec:RM" ;; BASE) echo "spec:BASE" ;; AM) echo "spec:AM" ;;
    LANG) echo "spec:LANG" ;; QUERY) echo "spec:QUERY" ;; TERM) echo "spec:TERM" ;;
    SM) echo "spec:SM" ;; CNF) echo "spec:CNF" ;; ITS-*) echo "spec:ITS" ;;
    *) echo "" ;;
  esac
}
# Our vendored pin, human-readable: the spec VERSION we currently have
# (parsed out of the vendor ref) + the vendored commit. This is the BASELINE
# ("what we already have"), never a ceiling — upstream changes targeting any
# newer version still get an issue.
pin_for_component() {
  local comp="$1" line ref sha ver
  line=$(grep -E "^  \"${comp}\|" scripts/vendor-spec-docs.sh | head -1 | tr -d '"') || true
  [ -n "$line" ] || { echo "unpinned"; return 0; }
  ref=$(echo "$line" | awk -F'|' '{print $3}')
  sha=$(echo "$line" | awk -F'|' '{printf "%.9s", $4}')
  # "master (BASE 1.3.0)" -> 1.3.0 · "Release-1.1.0" -> 1.1.0 · else the ref word
  ver=$(echo "$ref" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
  [ -n "$ver" ] || ver=$(echo "$ref" | awk '{print $1}')
  echo "$ver @ $sha"
}
repo_for_component() {
  local comp="$1" line
  line=$(grep -E "^  \"${comp}\|" scripts/vendor-spec-docs.sh | head -1 | tr -d '"') || true
  [ -n "$line" ] || return 1
  echo "$line" | awk -F'|' '{print $2}'
}

# ── shared: candidate sink (key<TAB>component<TAB>summary<TAB>extra) ─────────
CANDIDATES="$tmp/candidates.tsv"
: > "$CANDIDATES"

# ── A. Jira poll ─────────────────────────────────────────────────────────────
jira_fetch() { # $1 = full URL; body on stdout; one bounded retry on 429
  local url="$1" code attempt
  for attempt in 1 2; do
    code=$(curl -sS -o "$tmp/resp.json" -w '%{http_code}' "$url") ||
      { echo "spec-update-watcher: curl transport failure for $url" >&2; return 1; }
    case "$code" in
      200) cat "$tmp/resp.json"; return 0 ;;
      429) [ "$attempt" = 1 ] && { echo "Jira 429 — one bounded retry in 30s" >&2; sleep 30; continue; } ;;
    esac
    echo "spec-update-watcher: Jira returned HTTP $code for $url" >&2
    head -c 500 "$tmp/resp.json" >&2 || true
    return 1
  done
}

jql="project in (${PROJECTS}) AND status in (Resolved, Closed) AND resolutiondate >= -${WINDOW_DAYS}d ORDER BY resolutiondate DESC"
jql_enc=$(jq -rn --arg s "$jql" '$s|@uri')
fields="summary,status,resolution,resolutiondate,created,issuetype,fixVersions,components,description"
echo "spec-update-watcher: Jira poll — window ${WINDOW_DAYS}d, projects ${PROJECTS}"

: > "$tmp/jira.jsonl"
token=""
while :; do
  url="$JIRA/rest/api/3/search/jql?maxResults=50&fields=$fields&jql=$jql_enc"
  [ -n "$token" ] && url="$url&nextPageToken=$(jq -rn --arg s "$token" '$s|@uri')"
  body=$(jira_fetch "$url")
  echo "$body" | jq -e 'has("issues") and has("isLast")' >/dev/null ||
    { echo "spec-update-watcher: unexpected Jira payload shape (no issues/isLast) — API changed?" >&2; exit 1; }
  echo "$body" | jq -c '.issues[]' >> "$tmp/jira.jsonl"
  [ "$(echo "$body" | jq -r '.isLast')" = "true" ] && break
  token=$(echo "$body" | jq -r '.nextPageToken // empty')
  [ -n "$token" ] || { echo "spec-update-watcher: isLast=false but no nextPageToken" >&2; exit 1; }
done

jira_count=$(wc -l < "$tmp/jira.jsonl" | tr -d ' ')
echo "spec-update-watcher: Jira poll matched $jira_count completed issue(s)"

# Keys already present in the VENDORED amendment records: the change is
# already inside our pinned spec text — no triage needed, skip with a note.
# (Explicit file list — grep --include on explicit args differs across
# GNU/BSD grep flavors.)
{ find docs/specs/openehr -name 'master00-amendment_record.adoc'
  echo docs/specs/openehr/ITS-REST/specifications/docs/overview/Amendment_record.md
} | xargs grep -hoE 'SPEC[A-Z]*-[0-9]+' | sort -u > "$tmp/vendored-keys"

while IFS= read -r issue; do
  key=$(echo "$issue" | jq -r '.key')
  project="${key%%-*}"
  if grep -qxF "$key" "$tmp/vendored-keys"; then
    echo "  $key: already inside the vendored pin (amendment record carries it) — skipped"
    continue
  fi
  # A rejected/duplicate resolution means NOTHING changed in the spec — no
  # triage material. Skip with a note (still deduped later if it ever flips).
  resolution=$(echo "$issue" | jq -r '.fields.resolution.name // "unresolved"')
  case "$resolution" in
    "Won't Do"|"Won't Fix"|"Duplicate"|"Cannot Reproduce"|"Declined"|"Abandoned"|"Not a Bug")
      echo "  $key: resolution '$resolution' — no spec change, skipped"
      continue ;;
  esac
  summary=$(echo "$issue" | jq -r '.fields.summary')
  status=$(echo "$issue" | jq -r '.fields.status.name // "unknown"')
  itype=$(echo "$issue" | jq -r '.fields.issuetype.name // "unknown"')
  created=$(echo "$issue" | jq -r '.fields.created // "unknown" | split("T")[0]')
  resolved=$(echo "$issue" | jq -r '.fields.resolutiondate // "unknown" | split("T")[0]')
  # Fix versions with their release state: "Release-1.3.0 (unreleased)".
  fixv=$(echo "$issue" | jq -r '[.fields.fixVersions[]? | "\(.name) (\(if .released then "released" else "unreleased" end))"] | join(", ") | if . == "" then "—" else . end')
  fixv_plain=$(echo "$issue" | jq -r '[.fields.fixVersions[]?.name] | join(", ") | if . == "" then "—" else . end')
  comps=$(echo "$issue" | jq -r '[.fields.components[]?.name] | join(", ") | if . == "" then "—" else . end')
  # Description arrives as Atlassian Document Format — flatten the text
  # leaves and truncate to a triage-sized excerpt.
  descr=$(echo "$issue" | jq -r '[.fields.description // {} | .. | .text? // empty] | join(" ") | .[0:700]' | tr "$US" ' ' | tr '\n' ' ')
  component=$(component_for_project "$project")
  printf "%s${US}%s${US}%s${US}jira${US}%s${US}%s${US}%s${US}%s${US}%s${US}%s${US}%s${US}%s${US}%s\n" \
    "$key" "$component" "$summary" "$resolved" "$fixv" "$comps" \
    "$status" "$resolution" "$itype" "$created" "$fixv_plain" "$descr" >> "$CANDIDATES"
done < "$tmp/jira.jsonl"

# ── B. Amendment-record cross-check ─────────────────────────────────────────
echo "spec-update-watcher: amendment-record cross-check against the vendored mirror"
amendment_files=$(find docs/specs/openehr -name "master00-amendment_record.adoc" | sort)
amendment_files="$amendment_files
docs/specs/openehr/ITS-REST/specifications/docs/overview/Amendment_record.md"

while IFS= read -r local_path; do
  [ -f "$local_path" ] || { echo "spec-update-watcher: vendored amendment file missing: $local_path" >&2; exit 1; }
  comp=${local_path#docs/specs/openehr/}; comp=${comp%%/*}
  rel=${local_path#docs/specs/openehr/"$comp"/}
  repo=$(repo_for_component "$comp") ||
    { echo "spec-update-watcher: no upstream repo mapping for component $comp" >&2; exit 1; }
  # Default-branch content via the contents API (gh handles auth + rate limits);
  # a 404 here means the upstream moved the file — that must be a RED run.
  if ! gh api "repos/openEHR/$repo/contents/$rel" --jq '.content' 2>"$tmp/gh-err" | base64 -d > "$tmp/upstream" 2>/dev/null; then
    echo "spec-update-watcher: failed to fetch openEHR/$repo/$rel (moved upstream?):" >&2
    cat "$tmp/gh-err" >&2
    exit 1
  fi
  new_keys=$(comm -23 \
    <(grep -oE 'SPEC[A-Z]*-[0-9]+' "$tmp/upstream" | sort -u) \
    <(grep -oE 'SPEC[A-Z]*-[0-9]+' "$local_path" | sort -u) || true)
  if [ -n "$new_keys" ]; then
    while IFS= read -r key; do
      printf "%s${US}%s${US}%s${US}amendment${US}%s${US}%s${US}%s${US}%s${US}%s${US}%s${US}%s${US}%s${US}%s\n" \
        "$key" "$comp" "new amendment-record row in openEHR/$repo/$rel" \
        "see amendment record" "—" "—" \
        "(amendment)" "(amendment)" "amendment row" "—" "—" \
        "New row referencing $key in the upstream amendment record openEHR/$repo/$rel — not present in our vendored copy." >> "$CANDIDATES"
    done <<< "$new_keys"
  fi
done <<< "$amendment_files"

# ── C+D. Dedup by key against the issue board (open AND closed), create ─────
# Sort so the richer Jira-sourced row wins over an amendment duplicate.
sort -t"$US" -k1,1 -k4,4r "$CANDIDATES" | awk -F"$US" '!seen[$1]++' > "$tmp/unique.tsv"

created=0 skipped=0
while IFS="$US" read -r key component summary source resolved fixv comps status resolution itype jcreated fixv_plain descr; do
  [ -n "$key" ] || continue
  existing=$(gh issue list --state all --search "\"$key\" in:title" --json number --jq 'length')
  if [ "$existing" -gt 0 ]; then
    skipped=$((skipped + 1))
    continue
  fi
  # Title leads with WHERE THE CHANGE LANDS upstream (the Jira fix version),
  # not with our pin — our vendored baseline is context and lives in the body.
  case "$fixv_plain" in
    ""|"—") target="version unassigned" ;;
    *) target="$fixv_plain" ;;
  esac
  if [ -n "$component" ]; then
    pin=$(pin_for_component "$component")
    title="[spec-update] $key — $summary ($component → $target)"
  else
    pin="n/a"
    title="[spec-update] $key — $summary (cross-component → $target)"
  fi
  label_args=(--label spec-update)
  lbl=$([ -n "$component" ] && label_for_component "$component" || true)
  [ -n "${lbl:-}" ] && label_args+=(--label "$lbl")

  cat > "$tmp/body.md" <<EOF
Upstream openEHR spec change completed — conformance-impact triage needed.

- **Jira:** [$key]($JIRA/browse/$key) — $itype
- **Upstream state:** $status / resolution: $resolution
- **Created:** $jcreated · **Completed (resolved):** $resolved
- **Lands in upstream version:** $fixv
- **Jira component(s):** $comps
- **Detected via:** $source poll
- **What we currently have vendored (the baseline this is newer than):** ${component:-n/a} ${pin} (\`docs/VERSIONS.md\` / \`scripts/vendor-spec-docs.sh\`)

### Upstream summary

$summary

$([ -n "$descr" ] && printf '<details><summary>Upstream description (excerpt)</summary>\n\n%s\n\n</details>' "$descr")

### Triage checklist

- [ ] Re-vendor the affected spec (\`scripts/vendor-spec-docs.sh\` — bump the pin + SHA)
- [ ] Regenerate if BMM/OAS/XSD changed (\`/regen-codegen\`)
- [ ] Assess behaviour impact — add exactly one \`spec-impact:*\` label
- [ ] Implement where behaviour changes; update \`docs/VERSIONS.md\`

_Opened automatically by \`.github/workflows/spec-update-watcher.yml\`._
EOF

  if [ "$DRY_RUN" = "1" ]; then
    echo "DRY-RUN would create: $title  [${label_args[*]}]"
    sed 's/^/    │ /' "$tmp/body.md"
  else
    gh issue create --title "$title" "${label_args[@]}" --body-file "$tmp/body.md" >/dev/null
    echo "created: $title"
  fi
  created=$((created + 1))
done < "$tmp/unique.tsv"

echo "spec-update-watcher: done — $created new, $skipped already on the board (dedup by key)."
