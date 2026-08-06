#!/usr/bin/env bash
# Assemble the full Pages site tree into ./_site, exactly as CI does — so
# "works on my machine" == the deployed layout (URL scheme, frozen versions,
# and workflow all match the Docs CI job).
#
#   --dev-only   landing + /api/ + /docs/dev/ + versions.json + 404 + robots
#                (the PR/build-job path — no frozen versions, no deploy)
#   --full       everything above PLUS the frozen /docs/vX.Y.Z/ trees from the
#                docs-dist branch, a fresh /docs/latest/ from the newest tag,
#                and a generated sitemap.xml (the deploy-job path)
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
OUT="$ROOT/_site"
# The site is served from the ferroehr.eu apex (GitHub Pages custom domain),
# so it lives at the domain ROOT: SITE_BASE is empty. Both stay overridable so
# a sub-path build (e.g. a fork's project-pages URL) is still one env var away.
SITE_ORIGIN="${SITE_ORIGIN:-https://ferroehr.eu}"
SITE_BASE="${SITE_BASE:-}"
MODE="${1:---dev-only}"

log() { printf '\033[1;33m[build-site]\033[0m %s\n' "$*"; }

build_book() {
  # $1 = site-url (with trailing slash), $2 = absolute dest dir
  log "book -> $2  (site-url $1)"
  MDBOOK_OUTPUT__HTML__SITE_URL="$1" mdbook build "$ROOT/website/book" -d "$2" >/dev/null
}

# 1. Served OAS is a byte copy of the vendored ITS-REST bundles.
bash "$ROOT/scripts/assemble-oas.sh"

# 1b. Conformance claims are derived from the committed runner artifacts —
#     the book include is generated before mdbook runs, and the landing's
#     data-cnf markers are filled after the copy (step 3). Sources carry no
#     numbers (CI: scripts/checks/check-conformance-numbers.sh).
bash "$ROOT/scripts/render-conformance-stats.sh" includes
bash "$ROOT/scripts/render-comparison.sh"
bash "$ROOT/scripts/render-perf-assets.sh"

# 2. Clean + recreate _site.
rm -rf "$OUT"
mkdir -p "$OUT"

# 3. Landing at the site root (relative-URL HTML, so no base-path rewriting);
#    conformance markers filled from the committed artifacts.
cp -R "$ROOT/website/landing/." "$OUT/"
bash "$ROOT/scripts/render-conformance-stats.sh" fill-html "$OUT/index.html"

# 4. API endpoint reference at /api/ (Swagger UI + vendored dist + served specs).
mkdir -p "$OUT/api"
cp -R "$ROOT/website/api/." "$OUT/api/"

# 5. The dev book at /docs/dev/.
build_book "$SITE_BASE/docs/dev/" "$OUT/docs/dev"

# 6. Version manifest (docs-dist copy wins in --full; else the in-repo dev stub).
VERSIONS_SRC="$ROOT/website/versions.json"
if [[ "$MODE" == "--full" && -f "$ROOT/docs-dist/versions.json" ]]; then
  VERSIONS_SRC="$ROOT/docs-dist/versions.json"
fi
#    Entries in docs-dist were written under whatever base the site used at the
#    time — `/ehrbase-rs` before the rename, `/ferroehr` before the ferroehr.eu
#    cutover — so their `path` values go stale on every move. Re-anchor each one
#    to the CURRENT SITE_BASE by keeping only the `/docs/<id>/` tail, which is
#    invariant. The frozen trees themselves are NOT rebuilt ("generate once"):
#    their internal links are relative, so only this manifest needs re-anchoring.
python3 - "$VERSIONS_SRC" "$OUT/versions.json" "$SITE_BASE" <<'PY'
import json, sys

src, dest, base = sys.argv[1], sys.argv[2], sys.argv[3]
manifest = json.load(open(src))
for entry in manifest.get("versions", []):
    path = entry.get("path")
    if isinstance(path, str):
        marker = path.find("/docs/")
        if marker != -1:
            entry["path"] = base + path[marker:]
with open(dest, "w") as fh:
    json.dump(manifest, fh, indent=2)
    fh.write("\n")
PY

if [[ "$MODE" != "--full" ]]; then
  # The landing page links to /docs/latest/; alias it to the dev book so the
  # dev-only tree is self-consistent and the link gate stays meaningful.
  mkdir -p "$OUT/docs/latest"
  cp -R "$OUT/docs/dev/." "$OUT/docs/latest/"
  log "injecting per-page search metadata"
  cargo run -q -p docs-meta -- "$OUT" "$SITE_ORIGIN" "$SITE_BASE"
  log "dev-only site assembled at $OUT (latest aliased to dev)"
  exit 0
fi

# ── --full: frozen versions + latest + sitemap ──────────────────────────────

# 7. Materialize frozen /docs/vX.Y.Z/ trees from the docs-dist worktree.
if [[ -d "$ROOT/docs-dist/docs" ]]; then
  log "copying frozen versions from docs-dist"
  cp -R "$ROOT/docs-dist/docs/." "$OUT/docs/"
  # Each frozen tree carries mdBook's generated 404.html, whose asset and home
  # links are absolute and were baked with the base path in force when that
  # version was cut (`/ehrbase-rs/...`, then `/ferroehr/...`). Re-anchor those
  # to the CURRENT base in the ASSEMBLED OUTPUT only — docs-dist is untouched,
  # so "generate once, never rebuilt" still holds for the frozen trees.
  log "re-anchoring absolute links in frozen trees"
  python3 - "$OUT/docs" "$SITE_BASE" <<'PY'
import pathlib, re, sys

root, base = pathlib.Path(sys.argv[1]), sys.argv[2]
# A legacy base is exactly one path segment before /docs/; a already-correct
# "/docs/..." has no such segment and is left alone (so this is idempotent).
LEGACY = re.compile(r'(?<=["\'])/[A-Za-z0-9._-]+/docs/')
patched = 0
for html in root.rglob("*.html"):
    text = html.read_text(encoding="utf-8", errors="surrogateescape")
    fixed, n = LEGACY.subn(f"{base}/docs/", text)
    if n:
        html.write_text(fixed, encoding="utf-8", errors="surrogateescape")
        patched += 1
print(f"  re-anchored {patched} file(s)")
PY
else
  log "no docs-dist worktree — skipping frozen versions (dev-only content only)"
fi

# 8. Build /docs/latest/ fresh from the newest tag's sources (deep links work).
LATEST_TAG="$(python3 - "$OUT/versions.json" <<'PY'
import json, sys
try:
    m = json.load(open(sys.argv[1]))
except Exception:
    print(""); sys.exit(0)
lt = m.get("latest")
print(lt if lt and lt != "dev" else "")
PY
)"
if [[ -n "$LATEST_TAG" ]] && git rev-parse -q --verify "refs/tags/$LATEST_TAG" >/dev/null 2>&1; then
  log "building /docs/latest/ from tag $LATEST_TAG"
  WT="$ROOT/.latest-src"
  rm -rf "$WT"
  git worktree add --force --detach "$WT" "$LATEST_TAG" >/dev/null
  MDBOOK_OUTPUT__HTML__SITE_URL="$SITE_BASE/docs/latest/" mdbook build "$WT/website/book" -d "$OUT/docs/latest" >/dev/null
  git worktree remove --force "$WT" >/dev/null 2>&1 || rm -rf "$WT"
else
  log "no resolvable latest tag — aliasing /docs/latest/ to the dev book"
  mkdir -p "$OUT/docs/latest"
  cp -R "$OUT/docs/dev/." "$OUT/docs/latest/"
fi

# Per-page search metadata (canonical + description + Open Graph +
# BreadcrumbList) — see tools/docs-meta for why this runs over the built HTML
# rather than the mdBook theme. Idempotent.
log "injecting per-page search metadata"
cargo run -q -p docs-meta -- "$OUT" "$SITE_ORIGIN" "$SITE_BASE"

# 9. sitemap.xml — landing + /api/ + the pages of /docs/latest/ only (§2d).
log "generating sitemap.xml"
{
  echo '<?xml version="1.0" encoding="UTF-8"?>'
  echo '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">'
  host="${SITE_ORIGIN}${SITE_BASE}"
  echo "  <url><loc>${host}/</loc></url>"
  echo "  <url><loc>${host}/api/</loc></url>"
  if [[ -d "$OUT/docs/latest" ]]; then
    find "$OUT/docs/latest" -name '*.html' | sort | while IFS= read -r f; do
      rel="${f#"$OUT"}"
      echo "  <url><loc>${host}${rel}</loc></url>"
    done
  fi
  echo '</urlset>'
} > "$OUT/sitemap.xml"

# 10. robots.txt — disallow dev + every archived non-latest version (§2d).
log "writing robots.txt disallows"
{
  echo '# Generated by scripts/build-site.sh --full.'
  echo 'User-agent: *'
  echo 'Allow: /'
  echo "Disallow: ${SITE_BASE}/docs/dev/"
  python3 - "$OUT/versions.json" "$SITE_BASE" <<'PY'
import json, sys
try:
    m = json.load(open(sys.argv[1]))
except Exception:
    sys.exit(0)
base = sys.argv[2]
latest = m.get("latest")
for v in m.get("versions", []):
    vid = v.get("id")
    if vid in ("dev", "latest") or vid == latest or v.get("aliasOf"):
        continue
    print(f"Disallow: {base}/docs/{vid}/")
PY
  echo ""
  echo "Sitemap: ${SITE_ORIGIN}${SITE_BASE}/sitemap.xml"
} > "$OUT/robots.txt"

log "full site assembled at $OUT"
