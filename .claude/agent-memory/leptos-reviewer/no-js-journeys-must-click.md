---
name: no-js-journeys-must-click
description: the "authenticated shell is inert <template> fragments without JS" excuse in the e2e file is STALE — authenticated routes are SsrMode::Async, so no-JS journeys can and must click real DOM
metadata:
  type: project
---

The no-JS e2e journeys in `app/ferroehr-viewer/tests/e2e_composition.rs` carry a
comment claiming the authenticated shell arrives as inert `<template>` fragments
without JavaScript (out-of-order streaming), and therefore assert on
`driver.source()` substrings instead of interacting.

**Why that reason is stale:** the comment landed in `76d595153` (2026-07-18
09:06); the authenticated `ParentRoute` became `ssr=SsrMode::Async` in
`7e05f4517` (same day, 12:28) precisely so one complete document is sent. Async
mode emits no `<template>` placeholders — which is also why the no-JS *login*
journey happily `send_keys` + clicks a real submit button.

**How to apply:** when reviewing a progressive-enhancement journey, require it to
drive the real widget with JS disabled (`wait_css(...).send_keys(...)` + click +
`wait_url_contains(...)`). A `source.contains("name=\"find\"")`-style assertion
matches inert markup too, so it passes even when the control is unreachable or
the `on:submit` path is broken — it cannot prove the mandate (leptos-ui.md §10:
E2E is the merge gate). Direct `goto("/x?param=…")` proves only the
shareable-URL case, never the form.

Related: [[leptos-router-form-interception]]
