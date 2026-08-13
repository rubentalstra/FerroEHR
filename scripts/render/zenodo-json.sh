#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# Render `.zenodo.json` from `CITATION.cff` (#2210).
#
# Zenodo's own rule makes this file necessary and dangerous in one sentence:
# "If your repository contains both a .zenodo.json and a CITATION.cff file,
# Zenodo will only use the .zenodo.json metadata. The CITATION.cff will be
# COMPLETELY IGNORED for the GitHub release archiving."
# (https://help.zenodo.org/docs/github/describe-software/zenodo-json/)
#
# A hand-written copy would therefore silently disable the file CI already
# guards, and the two would drift with no signal — the deposit saying one thing
# while the citation box says another, under a DOI nobody can correct. So this
# file is GENERATED: CITATION.cff stays the single source for every fact both
# carry, and `citation-guard` runs this script in --check mode.
#
# The output is the InvenioRDM RECORD shape, not the flat legacy deposit shape
# the help page documents. Zenodo runs on InvenioRDM now and the GitHub
# integration accepts this form — verified against a live GitHub-archived
# record whose .zenodo.json is this shape and whose published record carries
# the custom_fields it declares. The legacy shape cannot say what language a
# piece of software is written in.
#
# Usage:
#   scripts/render/zenodo-json.sh            # write .zenodo.json
#   scripts/render/zenodo-json.sh --check    # fail if the committed file is stale
set -euo pipefail
cd "$(dirname "$0")/../.." || exit 1

command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }

CFF="CITATION.cff"
OUT=".zenodo.json"
MODE="${1:-write}"

# CITATION.cff is YAML, but every field read here is a flat quoted scalar, a
# folded block, or a simple list — so sed/awk read it and jq builds the JSON.
# No YAML parser, and no second language embedded in this script.
cff_scalar() {
  local v
  v="$(sed -nE "s/^$1:[[:space:]]*(.*)$/\1/p" "$CFF" | head -1)"
  v="${v%\"}"; v="${v#\"}"
  [ -n "$v" ] || { echo "$CFF has no \`$1\`" >&2; exit 1; }
  printf '%s' "$v"
}

# A folded block scalar (`key: >-`), rejoined onto one line.
cff_block() {
  awk -v key="$1" '
    $0 ~ "^" key ":[[:space:]]*>-[[:space:]]*$" { grab = 1; next }
    grab && /^[[:space:]]+/ { sub(/^[[:space:]]+/, ""); printf "%s%s", sep, $0; sep = " "; next }
    grab { exit }
  ' "$CFF"
}

# A plain list of scalars (`key:` then `  - value` lines), as a JSON array.
cff_list() {
  awk -v key="$1" '
    $0 ~ "^" key ":[[:space:]]*$" { grab = 1; next }
    grab && /^[[:space:]]+- / { sub(/^[[:space:]]+- /, ""); gsub(/^"|"$/, ""); print; next }
    grab { exit }
  ' "$CFF" | jq -R . | jq -s .
}

# `authors` → InvenioRDM `creators`. The ORCID is emitted as the BARE
# identifier: CITATION.cff stores the full https://orcid.org/… URL, and passing
# that through yields a record with no linked ORCID at all.
cff_creators() {
  awk '
    /^authors:[ \t]*$/ { grab = 1; next }
    !grab { next }
    # Dedent ends the block. Tested on the ORIGINAL line: stripping the indent
    # first would make every entry look dedented and end the block immediately.
    /^[^ \t]/ { exit }
    {
      line = $0
      if (sub(/^[ \t]*- /, "", line)) { if (rec != "") print rec; rec = "" }
      else sub(/^[ \t]+/, "", line)
      rec = rec (rec ? "\t" : "") line
    }
    END { if (rec != "") print rec }
  ' "$CFF" | jq -R -s '
    split("\n") | map(select(length > 0)) | map(
      (split("\t") | map(select(length > 0)) | map(
         capture("^(?<k>[^:]+):[[:space:]]*(?<v>.*)$")
         | {(.k): (.v | sub("^\"";"") | sub("\"$";""))}
       ) | add) as $a
      | {
          person_or_org: (
            {
              type: "personal",
              given_name: $a["given-names"],
              family_name: $a["family-names"],
              name: ($a["family-names"] + ", " + $a["given-names"]),
            }
            + (if $a.orcid
               then {identifiers: [{scheme: "orcid",
                                    identifier: ($a.orcid | split("/") | last)}]}
               else {} end)
          ),
        }
      + (if $a.affiliation then {affiliations: [{name: $a.affiliation}]} else {} end)
    )'
}

SPECS='[
  "https://specifications.openehr.org/releases/RM/Release-1.1.0",
  "https://specifications.openehr.org/releases/ITS-REST/Release-1.1.0",
  "https://specifications.openehr.org/releases/QUERY/Release-1.1.0",
  "https://specifications.openehr.org/releases/AM/Release-2.3.0"
]'
CRATES='["openehr-base","openehr-rm","openehr-am","openehr-term",
         "openehr-lang","openehr-query","openehr-its","openehr-adl"]'

rendered="$(
  jq -n \
    --arg title       "$(cff_scalar title)" \
    --arg abstract    "$(cff_block abstract)" \
    --arg version     "$(cff_scalar version)" \
    --arg released    "$(cff_scalar date-released)" \
    --arg license     "$(cff_scalar license | tr '[:upper:]' '[:lower:]')" \
    --arg repo        "$(cff_scalar repository-code)" \
    --arg site        "$(cff_scalar url)" \
    --argjson keywords "$(cff_list keywords)" \
    --argjson creators "$(cff_creators)" \
    --argjson specs    "$SPECS" \
    --argjson crates   "$CRATES" \
'{
  access: { record: "public", files: "public" },

  # The complete set of software custom fields Zenodo actually carries,
  # established by enumerating the custom_fields of the 100 newest software
  # records: code:codeRepository, code:programmingLanguage,
  # code:developmentStatus. code:operatingSystem, code:runtimePlatform,
  # code:softwareRequirements and code:license returned ZERO records and do not
  # exist — nothing here invents them.
  custom_fields: {
    "code:codeRepository": $repo,
    "code:programmingLanguage": [
      { id: "rust", title: { en: "Rust" } },
      { id: "sql",  title: { en: "SQL"  } }
    ],
    "code:developmentStatus": { id: "active", title: { en: "Active" } }
  },

  metadata: {
    resource_type: { id: "software" },
    title: $title,
    description: ("<p>" + $abstract + "</p>"),
    publication_date: $released,
    version: $version,
    creators: $creators,
    languages: [ { id: "eng", title: { en: "English" } } ],
    rights: [ { id: $license } ],
    subjects: ($keywords | map({ subject: . })),
    references: [
      { reference: "openEHR Reference Model (RM) Release 1.1.0. openEHR International. https://specifications.openehr.org/releases/RM/Release-1.1.0" },
      { reference: "openEHR Archetype Query Language (AQL), QUERY Release 1.1.0. openEHR International. https://specifications.openehr.org/releases/QUERY/Release-1.1.0" },
      { reference: "openEHR REST API (ITS-REST) Release 1.1.0. openEHR International. https://specifications.openehr.org/releases/ITS-REST/Release-1.1.0" },
      { reference: "openEHR Archetype Model (AM) Release 2.3.0. openEHR International. https://specifications.openehr.org/releases/AM/Release-2.3.0" }
    ],
    related_identifiers: (
      [ { identifier: $site, scheme: "url",
          relation_type: { id: "isdocumentedby" },
          resource_type: { id: "publication-softwaredocumentation" } } ]
      + ($specs | map({ identifier: ., scheme: "url",
                        relation_type: { id: "isderivedfrom" } }))
      + ($crates | map({ identifier: ("https://crates.io/crates/" + .),
                         scheme: "url",
                         relation_type: { id: "haspart" },
                         resource_type: { id: "software" } }))
    ),
    additional_descriptions: [
      { type: { id: "technical-info" },
        description: "<p>Implements the openEHR specifications at these pinned versions: Reference Model 1.2.0, BASE 1.3.0, Archetype Model 1.4.0 and 2.4.0, Terminology 3.1.0, AQL (QUERY) 1.1.0, ITS-REST 1.1.0 and ITS-XML. The specification layer is generated from the official machine-readable specifications rather than hand-written, and conformance is measured per release by a built-in openEHR CNF conformance runner whose results are committed alongside the source. Requires PostgreSQL 18.</p>" },
      { type: { id: "notes" },
        description: "<p>Licensing: the recorded licence covers this project&rsquo;s own code. Vendored third-party material keeps its upstream terms &mdash; Apache-2.0 for the openEHR machine-readable artifacts and test corpora, CC-BY-SA-3.0 for the openEHR specification text and CKM-derived clinical models &mdash; each recorded in the PROVENANCE.md of the tree that carries it.</p>" }
    ]
  }
}'
)"

if [ "$MODE" = "--check" ]; then
  [ -f "$OUT" ] || { echo "::error::$OUT is missing — run scripts/render/zenodo-json.sh" >&2; exit 1; }
  if ! printf '%s\n' "$rendered" | diff -u "$OUT" - >/dev/null; then
    echo "::error::$OUT is stale versus $CFF — run scripts/render/zenodo-json.sh" >&2
    printf '%s\n' "$rendered" | diff -u "$OUT" - || true
    exit 1
  fi
  echo "zenodo-json: $OUT matches $CFF"
  exit 0
fi

printf '%s\n' "$rendered" > "$OUT"
echo "zenodo-json: wrote $OUT from $CFF"
