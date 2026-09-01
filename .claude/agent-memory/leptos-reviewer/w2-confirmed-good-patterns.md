---
name: w2-confirmed-good-patterns
description: W2 viewer patterns already verified correct — don't re-flag these in future reviews of the crate
metadata:
  type: project
---

Confirmed-correct in the W2 viewer review (branch `claude/admin-ui`);
do not re-litigate:

- **Server-fn auth:** every session/CDR-touching `#[server]` fn calls
  `require_session()` first (`fetch_status`, `fetch_smart_config`,
  `fetch_openapi`); `current_session`/`login_modes`/`login_basic` are the
  deliberately-public exemptions. §0. (`logout` is the one session-touching fn
  without a guard — safe/idempotent, cookie-scoped; low-priority note only.)
- **Section-boundary `.into_any()` erasure** (§1) is followed consistently —
  each shell/system/login section is bound to an erased local. This is
  REQUIRED here (plain `cargo test` has no `erase_components`; monolithic thaw
  trees blow rustc layout recursion). Do not suggest collapsing them.
- **Theme Effect** in `shell.rs` reads localStorage (outside world) and writes
  `is_dark`/`theme` — a legitimate outside-world sync, NOT a forbidden
  signal-writes-signal effect (§2). Fixed `theme_id="ferroehr-viewer"` keeps the
  thaw style selector hydration-deterministic (§8).
- **No `usize` in serialized/server-fn types** (u16 for CDR status); **no
  unwrap/expect in production** (only `#[cfg(test)]`); **zero re-exports**, no
  `use X as Y`; public items documented. All clean.
- thaw form facts: [[thaw-field-random-id]] (a real §8 hazard) and
  [[thaw-input-name-forwarding-ok]] (fine).
