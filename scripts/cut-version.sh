#!/usr/bin/env bash
# Cut a frozen documentation version onto the docs-dist orphan branch.
# docs/design/docs-website.md §2c — "generate once, never rebuild".
#
# Usage: scripts/cut-version.sh vX.Y.Z
#
# Builds the book once with site-url=/ehrbase-rs/docs/vX.Y.Z/, rsyncs it into a
# docs-dist worktree under docs/vX.Y.Z/, prepends the version to versions.json,
# re-points the `latest` alias, then commits + pushes docs-dist. Refuses if the
# version already exists (frozen trees are never rebuilt).
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
SITE_BASE="${SITE_BASE:-/ehrbase-rs}"

VER="${1:-}"
if [[ -z "$VER" ]]; then
  echo "usage: scripts/cut-version.sh vX.Y.Z" >&2
  exit 2
fi
if [[ ! "$VER" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]]; then
  echo "::error::'$VER' is not a vX.Y.Z tag." >&2
  exit 2
fi

WT="$ROOT/docs-dist"
log() { printf '\033[1;33m[cut-version]\033[0m %s\n' "$*"; }

# 1. Check out (or create) the docs-dist orphan branch as a worktree.
git worktree remove --force "$WT" >/dev/null 2>&1 || true
git fetch origin docs-dist >/dev/null 2>&1 || true
if git show-ref --verify --quiet refs/remotes/origin/docs-dist; then
  log "checking out existing docs-dist"
  git worktree add --force -B docs-dist "$WT" origin/docs-dist >/dev/null
else
  log "creating fresh docs-dist orphan branch"
  git worktree add --force --detach "$WT" HEAD >/dev/null
  git -C "$WT" checkout --orphan docs-dist >/dev/null 2>&1
  git -C "$WT" reset --hard >/dev/null 2>&1 || true
  git -C "$WT" rm -rf . >/dev/null 2>&1 || true
  printf '%s\n' '{ "latest": null, "versions": [] }' > "$WT/versions.json"
fi

# 2. Guard: never rebuild an existing frozen version.
if [[ -d "$WT/docs/$VER" ]] || python3 - "$WT/versions.json" "$VER" <<'PY'
import json, sys
try:
    m = json.load(open(sys.argv[1]))
except Exception:
    sys.exit(1)
sys.exit(0 if any(v.get("id") == sys.argv[2] for v in m.get("versions", [])) else 1)
PY
then
  echo "::error::version $VER already exists in docs-dist — frozen versions are never rebuilt." >&2
  exit 1
fi

# 3. Build the frozen book once, straight into docs-dist/docs/vX.Y.Z/.
log "building frozen book -> docs/$VER/"
rm -rf "$WT/docs/$VER"
mkdir -p "$WT/docs/$VER"
MDBOOK_OUTPUT__HTML__SITE_URL="$SITE_BASE/docs/$VER/" mdbook build "$ROOT/website/book" -d "$WT/docs/$VER" >/dev/null

# 4. Update versions.json: prepend the version, re-point `latest`.
log "updating versions.json"
python3 - "$WT/versions.json" "$VER" "$SITE_BASE" <<'PY'
import datetime, json, sys
path, ver, base = sys.argv[1], sys.argv[2], sys.argv[3]
try:
    m = json.load(open(path))
except Exception:
    m = {"latest": None, "versions": []}
today = datetime.date.today().isoformat()
vers = [v for v in m.get("versions", []) if v.get("id") not in (ver, "latest", "dev")]

dev = {"id": "dev", "label": "dev (develop)", "path": f"{base}/docs/dev/", "released": None, "prerelease": True}
latest = {"id": "latest", "label": f"latest ({ver})", "path": f"{base}/docs/latest/", "released": today, "aliasOf": ver}
frozen = {"id": ver, "label": ver, "path": f"{base}/docs/{ver}/", "released": today}

m["latest"] = ver
m["versions"] = [dev, latest, frozen] + vers
json.dump(m, open(path, "w"), indent=2)
open(path, "a").write("\n")
PY

# 5. Commit + push docs-dist. CI runners have no git identity configured —
# without this the commit dies with `fatal: empty ident name` (v3.0.1 cut).
if ! git -C "$WT" config user.email >/dev/null 2>&1; then
  git -C "$WT" config user.name "github-actions[bot]"
  git -C "$WT" config user.email "41898282+github-actions[bot]@users.noreply.github.com"
fi
log "committing + pushing docs-dist"
git -C "$WT" add -A
if git -C "$WT" diff --cached --quiet; then
  log "nothing to commit"
else
  git -C "$WT" commit -m "docs: cut frozen documentation $VER" >/dev/null
  git -C "$WT" push origin docs-dist
fi

log "done — /docs/$VER/ frozen; latest -> $VER"
