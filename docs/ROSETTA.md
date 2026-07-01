# ROSETTA — Living Mapping Registry

This is the living Java↔Rust and openEHR-spec↔Rust mapping registry for the
port. Unlike `docs/PORTING.md` (the general rule set, fixed once written),
this file accumulates one row per concrete symbol as it gets ported or
transcribed, so a later session or subagent can look up "where did `X` go"
without re-deriving it.

Maintained by the `rosetta-curator` agent and the `rosetta-mapping` skill.
Append rows as files are ported or transcribed; do not delete a row once a
symbol has landed, even if the symbol is later renamed — update the row in
place instead. Kind values are free text but should stay consistent within a
table (e.g. `struct`, `enum`, `trait`, `fn`, `module`).

## Java → Rust

| Java symbol | Rust path | kind | notes |
|---|---|---|---|

## openEHR spec → Rust

| Spec construct | Rust path | kind | notes |
|---|---|---|---|
