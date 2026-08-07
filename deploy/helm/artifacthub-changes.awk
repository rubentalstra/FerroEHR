# Derive the Artifact Hub per-release annotations from a CHANGELOG.md section.
#
# Invoked by artifacthub-changes.sh; awk rather than an interpreter because there
# is no Python in this repository (owner directive 2026-08-05) and this runs on the
# chart PUBLISHING path, where a hidden interpreter dependency would surface years
# later on a runner image that dropped it. awk is POSIX and present everywhere
# these lanes run.
#
#   awk -v version=3.17.4 -f artifacthub-changes.awk CHANGELOG.md
#
# Emits the `artifacthub.io/changes` block, plus
# `artifacthub.io/containsSecurityUpdates` when the section has a `### Security`
# subsection. Exits 1 with a message on stderr when the section does not exist or
# lists no changes.

function fail(msg) { print msg > "/dev/stderr"; exit_code = 1; exit 1 }

# Markdown down to the plain text Artifact Hub renders: links to their text,
# emphasis and code ticks removed, whitespace collapsed.
function plain(s,   out) {
  # [text](url) -> text
  while (match(s, /\[[^]]*\]\([^)]*\)/)) {
    inner = substr(s, RSTART + 1, RLENGTH - 1)
    sub(/\]\(.*/, "", inner)
    s = substr(s, 1, RSTART - 1) inner substr(s, RSTART + RLENGTH)
  }
  gsub(/\*\*/, "", s)
  gsub(/`/, "", s)
  gsub(/[[:space:]]+/, " ", s)
  sub(/^ /, "", s); sub(/ $/, "", s)
  return s
}

# The entry's own one-line summary. This changelog's house style opens most
# entries with a bolded claim, and that IS the summary its author wrote, so it is
# used verbatim when present. The remainder fall back to a first-sentence split
# that ignores punctuation inside brackets, because the text is full of it
# (`EVENT.offset`, `(POST /admin/…)`).
function summary(entry,   flat, bold, depth, i, ch, nxt) {
  if (substr(entry, 1, 2) == "**") {
    rest = substr(entry, 3)
    idx = index(rest, "**")
    if (idx > 0) return plain(substr(rest, 1, idx - 1))
  }
  flat = plain(entry)
  depth = 0
  for (i = 1; i <= length(flat); i++) {
    ch = substr(flat, i, 1)
    if (ch == "(" || ch == "[") depth++
    else if (ch == ")" || ch == "]") { if (depth > 0) depth-- }
    else if ((ch == "." || ch == "!" || ch == "?") && depth == 0) {
      nxt = substr(flat, i + 1, 1)
      if (nxt == " ") return substr(flat, 1, i)
    }
  }
  return flat
}

function emit(kind, entry,   desc) {
  desc = summary(entry)
  gsub(/\\/, "\\\\", desc)
  gsub(/"/, "\\\"", desc)
  kinds[++n] = kind
  descs[n] = desc
}

function flush() {
  if (current != "") { emit(kind, current); current = "" }
}

BEGIN {
  if (version == "") fail("usage: awk -v version=<version|Unreleased> -f artifacthub-changes.awk CHANGELOG.md")
  want = tolower(version)
  KINDS = " added changed deprecated removed fixed security "
}

# Section headings: `## [x]`. Entering the wanted one turns collection on; the
# next heading turns it off.
/^## \[/ {
  flush()
  name = $0
  sub(/^## \[/, "", name); sub(/\].*/, "", name)
  if (tolower(name) == want) { inside = 1; found = 1 } else if (inside) { inside = 0 }
  seen = seen (seen == "" ? "" : ", ") name
  kind = ""
  next
}

!inside { next }

# Subsection headings map one-to-one onto Artifact Hub's change kinds, because
# those ARE Keep a Changelog's subsection names.
/^### / {
  flush()
  kind = tolower($2)
  if (index(KINDS, " " kind " ") == 0) fail("'### " $2 "' is not an Artifact Hub change kind")
  if (kind == "security") has_security = 1
  next
}

kind == "" { next }

# Entries are hard-wrapped: a bullet continues on any following
# two-space-indented line until a blank line, the next bullet, or the next
# subsection. Reading only the first physical line truncates the summary.
/^- / { flush(); current = substr($0, 3); next }
/^[[:space:]]*$/ { flush(); next }
/^  [^[:space:]]/ {
  if (current != "") {
    line = $0
    sub(/^[[:space:]]+/, "", line)
    current = current " " line
  }
  next
}

END {
  if (exit_code) exit exit_code
  flush()
  if (!found) fail("no '## [" version "]' section in CHANGELOG.md (found: " seen ")")
  if (n == 0) fail("the '## [" version "]' section lists no changes")
  print "  artifacthub.io/changes: |"
  for (i = 1; i <= n; i++) {
    print "    - kind: " kinds[i]
    print "      description: \"" descs[i] "\""
  }
  if (has_security) print "  artifacthub.io/containsSecurityUpdates: \"true\""
}
