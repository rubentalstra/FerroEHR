# Concepts

This part explains the ideas you need to use FerroEHR effectively — two
chapters, read in either order:

- **[openEHR primer](openehr-primer.md)** — the standard itself: the Reference
  Model, archetypes and templates, compositions, versioning, and AQL, with no
  prior openEHR knowledge assumed. Read this first if the words "archetype" and
  "composition" are new to you.
- **[System architecture](architecture.md)** — how *this* server is put
  together and where your data actually lives, so the behaviour you see through
  the API makes sense: why the specification layer is generated, how storage and
  versioning work on PostgreSQL, and how an AQL query becomes SQL.

Neither chapter is a prerequisite for [Getting started](../getting-started.md)
— you can commit a composition and run a query without them. Come back when you
want to know *why* the API behaves as it does, or before you make one of the
decisions you then live with: a specification generation, a template design, a
query shape.
