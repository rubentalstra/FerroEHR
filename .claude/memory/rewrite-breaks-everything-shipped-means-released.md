---
name: rewrite-breaks-everything-shipped-means-released
description: Owner ruling 2026-09-15 (emphatic) — the storage rewrite is ONLY breaking changes: anything on main since the last release (v4.3.0) is rewrite material and is fixed in place; no compatibility shims, placeholder roles, rename-by-addition or deprecation paths; "shipped" = present at the latest release tag
metadata:
  type: feedback
---

When a #3343 worker kept `ferroehr_ehr`/`ferroehr_demographic` as empty NOLOGIN roles because the (unreleased) generation-2 grant files named them, the owner: "we are in a greenfield so everything will break anyway ... if there is something that needs to be changed properly then break it so it's from now on correct" and "ONLY breaking changes, that is what we are doing with this whole rewrite ... we are fully redesigning everything". 

**Why:** no organisation runs FerroEHR in production; carrying legacy into the new design is the thing the rewrite exists to remove.

**How to apply:** during the rewrite cycle (v4.3.1), migration files that exist on main but not at the latest release tag are EDITABLE (the immutability guard compares against the release tag, not the merge base); wrong names, columns or grants are fixed in the file that defines them; never add a rename migration, a placeholder, a shim or a "retired after a deprecation release" note. The append-only rule re-arms automatically at the v4.3.1 cut. Related: [[storage-rewrite-is-greenfield]], [[migrations-are-append-only]], [[rewrite-not-inherited-code]].
