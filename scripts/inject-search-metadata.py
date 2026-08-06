#!/usr/bin/env python3
"""Inject per-page search metadata into an assembled documentation site.

mdBook emits ONE `<meta name="description">` for a whole book and no canonical
link at all, so without this pass `/docs/dev/`, `/docs/latest/` and every
frozen `/docs/vX.Y.Z/` are indexable copies of the same page sharing one
snippet: search engines split ranking signals across the copies and write the
snippet themselves.

This runs over the BUILT HTML rather than the mdBook theme on purpose. The
Handlebars context carries only the chapter's SOURCE path, so a template
cannot form the absolute canonical URL, and a canonical injected by JavaScript
is not reliably honoured by crawlers.

Per page it writes: a canonical URL pointing at the canonical version, a
description taken from the page's own opening prose, Open Graph and Twitter
card tags, `noindex,follow` on every non-canonical version, and a
`BreadcrumbList` mirroring the URL hierarchy with real labels.

Idempotent: a page already carrying the marker is skipped, so re-running over
an assembled site changes nothing.

Usage: inject-search-metadata.py <site-dir> <origin> <base-path>
"""

from __future__ import annotations

import html
import json
import pathlib
import re
import sys

MARKER = "<!-- ferroehr:seo -->"
# One URL per page wins the index; the other versions defer to it.
CANONICAL_VERSION = "latest"
SITE_NAME = "FerroEHR"
SOCIAL_CARD = "/assets/social-card.png"
# Search engines truncate a description near 160 characters; keep a little
# more so the sentence still reads, and cut on a word boundary.
DESCRIPTION_LIMIT = 300
DESCRIPTION_MINIMUM = 60


def first_paragraph(page_html: str) -> str:
    """Return the page's opening prose, or `""` when it has none."""
    prose = re.sub(r"(?is)<(script|style|nav|aside)\b.*?</\1>", " ", page_html)
    for match in re.finditer(r"(?is)<p\b[^>]*>(.*?)</p>", prose):
        text = re.sub(r"(?s)<[^>]+>", "", match.group(1))
        text = " ".join(html.unescape(text).split())
        if len(text) >= DESCRIPTION_MINIMUM:
            if len(text) > DESCRIPTION_LIMIT:
                text = text[:DESCRIPTION_LIMIT].rsplit(" ", 1)[0] + "…"
            return text
    return ""


def page_title(page_html: str) -> str:
    """Return the page's `<title>` text, collapsed to one line."""
    match = re.search(r"(?is)<title>(.*?)</title>", page_html)
    return " ".join(html.unescape(match.group(1)).split()) if match else ""


def segment_label(segment: str) -> str:
    """A human label for a URL path segment (`beyond-core` → `Beyond Core`)."""
    return segment.replace("-", " ").replace("_", " ").title()


def breadcrumbs(origin: str, base: str, version: str, rel: str, title: str) -> dict:
    """A `BreadcrumbList` mirroring the page's URL hierarchy."""
    items = [
        (SITE_NAME, f"{origin}{base}/"),
        ("Documentation", f"{origin}{base}/docs/{version}/"),
    ]
    trail = f"{origin}{base}/docs/{version}"
    for segment in [s for s in rel.split("/")[:-1] if s]:
        trail = f"{trail}/{segment}"
        items.append((segment_label(segment), f"{trail}/"))
    leaf = title.split(" - ")[0].strip() or "Page"
    items.append((leaf, f"{origin}{base}/docs/{version}/{rel}"))
    return {
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": [
            {"@type": "ListItem", "position": i, "name": name, "item": url}
            for i, (name, url) in enumerate(items, start=1)
        ],
    }


def meta_tags(origin: str, base: str, version: str, rel: str, page_html: str) -> str:
    """The metadata block for one page, marker first."""
    title = page_title(page_html)
    description = first_paragraph(page_html)
    canonical = f"{origin}{base}/docs/{CANONICAL_VERSION}/{rel}"
    tags = [MARKER, f'<link rel="canonical" href="{html.escape(canonical)}">']
    if description:
        escaped = html.escape(description, quote=True)
        tags.append(f'<meta name="description" content="{escaped}">')
        tags.append(f'<meta property="og:description" content="{escaped}">')
    if title:
        tags.append(f'<meta property="og:title" content="{html.escape(title, quote=True)}">')
    tags.append('<meta property="og:type" content="article">')
    tags.append(f'<meta property="og:site_name" content="{SITE_NAME}">')
    tags.append(f'<meta property="og:url" content="{html.escape(canonical)}">')
    tags.append(f'<meta property="og:image" content="{origin}{base}{SOCIAL_CARD}">')
    tags.append('<meta name="twitter:card" content="summary_large_image">')
    tags.append(f'<meta name="twitter:image" content="{origin}{base}{SOCIAL_CARD}">')
    # A non-canonical version must not compete in the index. robots.txt already
    # disallows crawling those trees; this states it in-page for a crawler that
    # arrives by an inbound link instead.
    if version != CANONICAL_VERSION:
        tags.append('<meta name="robots" content="noindex,follow">')
    crumb = json.dumps(
        breadcrumbs(origin, base, version, rel, title), separators=(",", ":")
    )
    tags.append(f'<script type="application/ld+json">{crumb}</script>')
    return "\n  ".join(tags)


def main() -> int:
    if len(sys.argv) != 4:
        print(
            "usage: inject-search-metadata.py <site-dir> <origin> <base-path>",
            file=sys.stderr,
        )
        return 2
    site, origin, base = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    docs = site / "docs"
    if not docs.is_dir():
        print("  no docs tree — skipped")
        return 0
    patched = 0
    for version_dir in sorted(p for p in docs.iterdir() if p.is_dir()):
        version = version_dir.name
        for page in sorted(version_dir.rglob("*.html")):
            rel = page.relative_to(version_dir).as_posix()
            if rel == "404.html" or rel.startswith("theme/"):
                continue
            body = page.read_text(encoding="utf-8", errors="surrogateescape")
            if MARKER in body or "</head>" not in body:
                continue
            block = meta_tags(origin, base, version, rel, body)
            page.write_text(
                body.replace("</head>", f"  {block}\n</head>", 1),
                encoding="utf-8",
                errors="surrogateescape",
            )
            patched += 1
    print(f"  wrote search metadata into {patched} page(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
