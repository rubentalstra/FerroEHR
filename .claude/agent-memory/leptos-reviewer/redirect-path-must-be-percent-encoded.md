---
name: redirect-path-must-be-percent-encoded
description: any user-supplied value that reaches a server-side redirect path MUST be urlencoding::encode'd — leptos_axum::redirect does HeaderValue::from_str(path).expect(), a remotely triggerable panic
metadata:
  type: reference
---

`leptos_axum::redirect` builds the `Location` header with
`HeaderValue::from_str(path).expect("Failed to create HeaderValue")`
(`leptos_axum-0.8.10/src/lib.rs:232-236`). Header values reject control
characters and non-ASCII bytes, so an unencoded user value interpolated into a
redirect path (`format!("/ehrs/{id}")`) is a **remotely triggerable panic** →
catch-panic 500, from a plain query string such as `?find=%0Aevil`.

`urlencoding::encode` emits only `[0-9A-Za-z\-._~]` plus `%XX`
(`urlencoding-2.1.3/src/enc.rs:98-103`), so encoding closes it and also keeps
the value inside its intended path segment (`/`, `?`, `#`, `%`). The viewer's
`ehrs::ehr_detail_href` is therefore load-bearing for server safety, not just
link cosmetics — never "simplify" it away, and never `format!` a raw param into
a `<Redirect path=…>`.

Note `.trim()` strips only leading/trailing whitespace — an interior newline
still reaches the header builder.

(Owner rule: all percent-coding goes through `urlencoding`; leptos_router
decodes params on read, `params.rs:29`, so the round-trip is lossless.)

Related: [[redirect-needs-ssrmode-async]]
