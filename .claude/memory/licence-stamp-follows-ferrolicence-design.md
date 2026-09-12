---
name: licence-stamp-follows-ferrolicence-design
description: FerroEHR's identifier stamp (app/ferroehr/src/licence/stamp.rs) is a COPY of the FerroLICENCE design; any change is verified with `ferrolicence attest` over freshly minted ids, and the known-answer tests pin the design, not the copy
metadata:
  type: project
---

The licence stamp in `app/ferroehr/src/licence/stamp.rs` must compute exactly
what `FerroLICENCE/docs/design.md` §7 specifies and what
`ferrolicence attest` recomputes: `bytes[14..16] = HMAC-SHA256(key,
timestamp_ms.to_be_bytes())[0..2]` with `key` = the RAW material
(`"ferroehr/licence/v1/" ‖ licence_id` or the public `"ferroehr/unlicensed/v1"`).
v4.2.0 shipped a drift (SHA-256 of the material as the key, and a
`no-licence` constant) that its own unit test pinned, so no 4.2.0 identifier
attests (#3282, fixed 2026-09-12).

**Why:** the two repositories carry two copies of the same algorithm and a
unit test that only checks the copy against itself cannot see the drift.

**How to apply:** after touching the stamp or the embedded token, run a
probe that mints three ids under the embedded licence and feed them to
`/Users/rubentalstra/RustroverProjects/FerroLICENCE/target/release/ferrolicence attest --repo <FerroLICENCE>`;
expect `IDENTIFIED` for the register's licence. The known-answer tests in
`stamp.rs` (`121e`, `4951`, `9e8a`) are computed independently in Python and
must not be "updated to match" the code. See [[veredictum-other-session]]
for the rule that sibling repositories are never edited from here.
