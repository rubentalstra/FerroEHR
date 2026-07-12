# The ehrbase-rs Conformance Catalogue (ECC)

Generated per run — do not edit. Numbers are allocated once in
`tools/conformance/inventory/ecc-catalog.tsv` and never reused.

## EHR — EHR service (13 cases, 13 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-EHR-001 | Active | EHR existence check — existing EHR id | passed |
| ECC-EHR-002 | Active | EHR existence check — existing subject id | passed |
| ECC-EHR-003 | Active | EHR existence check — non existing EHR id | passed |
| ECC-EHR-004 | Active | EHR existence check — non existing subject id | passed |
| ECC-EHR-005 | Active | Create EHR — main | passed |
| ECC-EHR-006 | Active | Create EHR — same EHR twice | passed |
| ECC-EHR-007 | Active | Create EHR — two EHRs same patient | passed |
| ECC-EHR-008 | Active | Get EHR — existing EHR by EHR id | passed |
| ECC-EHR-009 | Active | Get EHR — existing EHR by subject id | passed |
| ECC-EHR-010 | Active | Get EHR — get EHR by invalid EHR id | passed |
| ECC-EHR-011 | Active | Get EHR — get EHR by invalid subject id | passed |
| ECC-EHR-012 | Active | Create EHR — reject invalid EHR_STATUS data sets | passed |
| ECC-EHR-013 | Active | Create anonymous (subject-less) EHR | passed |

## STA — EHR_STATUS (10 cases, 10 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-STA-001 | Active | Get EHR_STATUS — get by EHR id | passed |
| ECC-STA-002 | Active | Get EHR_STATUS — bad EHR | passed |
| ECC-STA-003 | Active | Set EHR_STATUS is_queryable — existing EHR | passed |
| ECC-STA-004 | Active | Set EHR_STATUS is_queryable — bad EHR | passed |
| ECC-STA-005 | Active | Set EHR_STATUS is_modifiable — existing EHR | passed |
| ECC-STA-006 | Active | Set EHR_STATUS is_modifiable — bad EHR | passed |
| ECC-STA-007 | Active | Clear EHR_STATUS is_queryable — existing EHR | passed |
| ECC-STA-008 | Active | Clear EHR_STATUS is_queryable — bad EHR | passed |
| ECC-STA-009 | Active | Clear EHR_STATUS is_modifiable — existing EHR | passed |
| ECC-STA-010 | Active | Clear EHR_STATUS is_modifiable — bad EHR | passed |

## COM — COMPOSITION (31 cases, 31 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-COM-001 | Active | Create composition — event | passed/passed |
| ECC-COM-002 | Active | Create composition — persistent | passed/passed |
| ECC-COM-003 | Active | Create composition — same OPT twice | passed |
| ECC-COM-004 | Active | Create composition — invalid event | passed |
| ECC-COM-005 | Active | Create composition — invalid persistent | passed |
| ECC-COM-006 | Active | Create composition — event bad OPT | passed |
| ECC-COM-007 | Active | Create composition — event bad EHR | passed |
| ECC-COM-008 | Active | Get latest composition | passed/passed |
| ECC-COM-009 | Active | Get latest composition — bad composition | passed |
| ECC-COM-010 | Active | Get latest composition — bad EHR | passed |
| ECC-COM-011 | Active | Composition existence check — bad composition | passed |
| ECC-COM-012 | Active | Composition existence check — bad EHR | passed |
| ECC-COM-013 | Active | Get composition at time | passed/passed |
| ECC-COM-014 | Active | Get composition at time — no time arg | passed/passed |
| ECC-COM-015 | Active | Get composition at time — bad composition | passed |
| ECC-COM-016 | Active | Get composition at time — bad EHR | passed |
| ECC-COM-017 | Active | Get composition at multiple times | passed |
| ECC-COM-018 | Active | Get composition version | passed/passed |
| ECC-COM-019 | Active | Get composition version — bad version | passed |
| ECC-COM-020 | Active | Get composition version — bad EHR | passed |
| ECC-COM-021 | Active | Get composition versions | passed |
| ECC-COM-022 | Active | Get versioned composition | passed/passed |
| ECC-COM-023 | Active | Get versioned composition — non existent | passed |
| ECC-COM-024 | Active | Get versioned composition — bad EHR | passed |
| ECC-COM-025 | Active | Update composition — event | passed |
| ECC-COM-026 | Active | Update composition — persistent | passed |
| ECC-COM-027 | Active | Update composition — non existent | passed |
| ECC-COM-028 | Active | Update composition — wrong template | passed |
| ECC-COM-029 | Active | Delete composition — event | passed |
| ECC-COM-030 | Active | Delete composition — persistent | passed |
| ECC-COM-031 | Active | Delete composition — non existent | passed |

## CTB — CONTRIBUTION (change sets) (31 cases, 31 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-CTB-001 | Active | Commit contribution — valid composition | passed |
| ECC-CTB-002 | Active | Commit contribution — invalid composition | passed |
| ECC-CTB-003 | Active | Commit contribution — empty | passed |
| ECC-CTB-004 | Active | Commit contribution — valid invalid compositions | passed |
| ECC-CTB-005 | Active | Commit contribution — non exiting OPT | passed |
| ECC-CTB-006 | Active | Commit contribution — event composition | passed |
| ECC-CTB-007 | Active | Commit contribution — persistent composition | passed |
| ECC-CTB-008 | Active | Commit contribution — delete | passed |
| ECC-CTB-009 | Active | Commit contribution — two commits second invalid | passed |
| ECC-CTB-010 | Active | Commit contribution — two commits second creation | passed |
| ECC-CTB-011 | Active | Commit contribution — minimal EHR status | passed |
| ECC-CTB-012 | Active | Commit contribution — full EHR status | passed |
| ECC-CTB-013 | Active | Commit contribution — EHR status invalid change type | passed |
| ECC-CTB-014 | Active | Commit contribution — invalid EHR status | passed |
| ECC-CTB-015 | Active | Commit contribution — valid directory | passed |
| ECC-CTB-016 | Active | Commit contribution — fail create existing directory | passed |
| ECC-CTB-017 | Active | Commit contribution — fail modify non existing directory | passed |
| ECC-CTB-018 | Active | Commit contribution — update existing directory | passed |
| ECC-CTB-019 | Active | Get contribution — existing | passed |
| ECC-CTB-020 | Active | Get contribution — empty EHR | passed |
| ECC-CTB-021 | Active | Get contribution — bad EHR | passed |
| ECC-CTB-022 | Active | Get contribution — bad contribution | passed |
| ECC-CTB-023 | Active | Contribution existence check — existing | passed |
| ECC-CTB-024 | Active | Contribution existence check — bad contribution | passed |
| ECC-CTB-025 | Active | Contribution existence check — bad EHR | passed |
| ECC-CTB-026 | Active | Contribution existence check — empty EHR | passed |
| ECC-CTB-027 | Active | List contributions — empty | skipped |
| ECC-CTB-028 | Active | List contributions — non existing EHR | skipped |
| ECC-CTB-029 | Active | List contributions — post commit | skipped |
| ECC-CTB-030 | Active | List contributions — EHR containing directory | skipped |
| ECC-CTB-031 | Active | List contributions — EHR containing EHR status | skipped |

## DIR — DIRECTORY (FOLDER) (37 cases, 37 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-DIR-001 | Active | Create directory — empty EHR | passed |
| ECC-DIR-002 | Active | Create directory — EHR with directory | passed |
| ECC-DIR-003 | Active | Create directory — bad EHR | passed |
| ECC-DIR-004 | Active | Get directory — EHR root directory | passed |
| ECC-DIR-005 | Active | Get directory — bad EHR | passed |
| ECC-DIR-006 | Active | Get directory at time — EHR with directory | passed |
| ECC-DIR-007 | Active | Get directory at time — bad EHR | passed |
| ECC-DIR-008 | Active | Update directory — EHR with directory | passed |
| ECC-DIR-009 | Active | Update directory — bad EHR | passed |
| ECC-DIR-010 | Active | Delete directory — EHR with directory | passed |
| ECC-DIR-011 | Active | Delete directory — bad EHR | passed |
| ECC-DIR-012 | Active | Directory existence check — empty EHR | passed |
| ECC-DIR-013 | Active | Directory existence check — EHR with directory | passed |
| ECC-DIR-014 | Active | Directory existence check — bad EHR | passed |
| ECC-DIR-015 | Active | Directory path existence check — EHR root directory | passed |
| ECC-DIR-016 | Active | Directory path existence check — folder structure | passed |
| ECC-DIR-017 | Active | Directory path existence check — empty EHR | passed |
| ECC-DIR-018 | Active | Directory path existence check — bad EHR | passed |
| ECC-DIR-019 | Active | Directory version existence check — empty EHR | passed |
| ECC-DIR-020 | Active | Directory version existence check — directory with two versions | passed |
| ECC-DIR-021 | Active | Directory version existence check — bad EHR | passed |
| ECC-DIR-022 | Active | Get directory — empty EHR | passed |
| ECC-DIR-023 | Active | Get directory — directory with structure | passed |
| ECC-DIR-024 | Active | Get directory at time — EHR with directory empty time | passed |
| ECC-DIR-025 | Active | Get directory at time — EHR with directory versions | passed |
| ECC-DIR-026 | Active | Get directory at time — EHR with directory versions empty time | passed |
| ECC-DIR-027 | Active | Get directory at time — empty EHR | passed |
| ECC-DIR-028 | Active | Get directory at time — empty EHR empty time | passed |
| ECC-DIR-029 | Active | Get directory at time — multiple versions first | passed |
| ECC-DIR-030 | Active | Get directory at version — bad EHR | passed |
| ECC-DIR-031 | Active | Get directory at version — directory with two versions | passed |
| ECC-DIR-032 | Active | Get directory at version — empty EHR | passed |
| ECC-DIR-033 | Active | Get versioned directory — empty EHR | passed |
| ECC-DIR-034 | Active | Get versioned directory — directory with two versions | passed |
| ECC-DIR-035 | Active | Get versioned directory — bad EHR | passed |
| ECC-DIR-036 | Active | Update directory — empty EHR | passed |
| ECC-DIR-037 | Active | Delete directory — empty EHR | passed |

## TPL — Template / OPT provisioning (16 cases, 16 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-TPL-001 | Active | Upload OPT — valid OPT | passed |
| ECC-TPL-002 | Active | Upload OPT — invalid OPT | passed |
| ECC-TPL-003 | Active | List OPTs — retrieve all no OPTs | passed |
| ECC-TPL-004 | Active | Upload OPT — valid OPT twice conflict | passed |
| ECC-TPL-005 | Active | Upload OPT — valid OPT twice no conflict | passed |
| ECC-TPL-006 | Active | Get OPT — retrieve single | passed |
| ECC-TPL-007 | Active | Get OPT — retrieve latest version | passed |
| ECC-TPL-008 | Active | Get OPT — retrieve specific version | passed |
| ECC-TPL-009 | Active | Get OPT — retrieve fail | passed |
| ECC-TPL-010 | Active | List OPTs — retrieve all | passed |
| ECC-TPL-011 | Active | Validate OPT — valid OPT | passed |
| ECC-TPL-012 | Active | Validate OPT — invalid OPT | passed |
| ECC-TPL-013 | Active | Delete OPT — delete non existing | skipped |
| ECC-TPL-014 | Active | Delete OPT — delete existing | skipped |
| ECC-TPL-015 | Active | Delete OPT — delete latest version | skipped |
| ECC-TPL-016 | Active | Delete OPT — delete specific version | skipped |

## SQR — Stored-query provisioning (7 cases, 7 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-SQR-001 | Active | Store stored query — valid | passed |
| ECC-SQR-002 | Active | List stored queries — non empty | passed |
| ECC-SQR-003 | Active | Stored query existence check — xxx | passed |
| ECC-SQR-004 | Active | List stored queries — empty | skipped |
| ECC-SQR-005 | Active | List stored queries — select items | skipped |
| ECC-SQR-006 | Active | Store stored query — bad formalism | passed |
| ECC-SQR-007 | Active | Store stored query — invalid | passed |

## QRY — AQL execution (13 cases, 13 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-QRY-001 | Active | Query service smoke test | passed |
| ECC-QRY-002 | Active | Execute ad-hoc AQL query — empty db | passed |
| ECC-QRY-003 | Active | Execute stored AQL query — empty db | passed |
| ECC-QRY-004 | Active | Execute ad-hoc AQL query — loaded db | passed |
| ECC-QRY-005 | Active | AQL corpus — invalid queries rejected | passed |
| ECC-QRY-006 | Active | AQL corpus — A empty db | passed |
| ECC-QRY-007 | Active | AQL corpus — B empty db | passed |
| ECC-QRY-008 | Active | AQL corpus — C empty db | passed |
| ECC-QRY-009 | Active | AQL corpus — D empty db | passed |
| ECC-QRY-010 | Active | AQL corpus — A loaded db | passed |
| ECC-QRY-011 | Active | AQL corpus — B loaded db | passed |
| ECC-QRY-012 | Active | AQL corpus — C loaded db | passed |
| ECC-QRY-013 | Active | AQL corpus — D loaded db | passed |

## VAL — Content / archetype validation (119 cases, 119 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-VAL-001 | Active | Validate COMPOSITION — content card any context any | passed |
| ECC-VAL-002 | Active | Validate COMPOSITION — content card 1plus context any | passed |
| ECC-VAL-003 | Active | Validate COMPOSITION — content card 3plus context any | passed |
| ECC-VAL-004 | Active | Validate COMPOSITION — content card OPT context any | passed |
| ECC-VAL-005 | Active | Validate COMPOSITION — content card mand context any | passed |
| ECC-VAL-006 | Active | Validate COMPOSITION — content card 3to5 context any | passed |
| ECC-VAL-007 | Active | Validate COMPOSITION — content card any context mand | passed |
| ECC-VAL-008 | Active | Validate COMPOSITION — content card 1plus context mand | passed |
| ECC-VAL-009 | Active | Validate COMPOSITION — content card 3plus context mand | passed |
| ECC-VAL-010 | Active | Validate COMPOSITION — content card OPT context mand | passed |
| ECC-VAL-011 | Active | Validate COMPOSITION — content card mand context mand | passed |
| ECC-VAL-012 | Active | Validate COMPOSITION — content card 3to5 context mand | passed |
| ECC-VAL-013 | Active | Validate OBSERVATION — state ex OPT protocol ex OPT | passed |
| ECC-VAL-014 | Active | Validate OBSERVATION — state ex OPT protocol ex mand | passed |
| ECC-VAL-015 | Active | Validate OBSERVATION — state ex mand protocol ex OPT | passed |
| ECC-VAL-016 | Active | Validate OBSERVATION — state ex mand protocol ex mand | passed |
| ECC-VAL-017 | Active | Validate HISTORY — events card any summary ex OPT | passed |
| ECC-VAL-018 | Active | Validate HISTORY — events card 1plus summary ex OPT | passed |
| ECC-VAL-019 | Active | Validate HISTORY — events card 3plus summary ex OPT | passed |
| ECC-VAL-020 | Active | Validate HISTORY — events card OPT summary ex OPT | passed |
| ECC-VAL-021 | Active | Validate HISTORY — events card mand summary ex OPT | passed |
| ECC-VAL-022 | Active | Validate HISTORY — events card 3to5 summary ex OPT | passed |
| ECC-VAL-023 | Active | Validate HISTORY — events card any summary ex mand | passed |
| ECC-VAL-024 | Active | Validate HISTORY — events card 1plus summary ex mand | passed |
| ECC-VAL-025 | Active | Validate HISTORY — events card 3plus summary ex mand | passed |
| ECC-VAL-026 | Active | Validate HISTORY — events card OPT summary ex mand | passed |
| ECC-VAL-027 | Active | Validate HISTORY — events card mand summary ex mand | passed |
| ECC-VAL-028 | Active | Validate HISTORY — events card 3to5 summary ex mand | passed |
| ECC-VAL-029 | Active | Validate EVENT — state ex OPT | passed |
| ECC-VAL-030 | Active | Validate EVENT — state ex mand | passed |
| ECC-VAL-031 | Active | Validate EVENT — type any | passed |
| ECC-VAL-032 | Active | Validate EVENT — type point event | passed |
| ECC-VAL-033 | Active | Validate EVENT — type interval event | passed |
| ECC-VAL-034 | Active | Validate ITEM_STRUCTURE — type any | passed |
| ECC-VAL-035 | Active | Validate ITEM_STRUCTURE — type item tree | passed |
| ECC-VAL-036 | Active | Validate ITEM_STRUCTURE — type item list | passed |
| ECC-VAL-037 | Active | Validate ITEM_STRUCTURE — type item table | passed |
| ECC-VAL-038 | Active | Validate ITEM_STRUCTURE — type item single | passed |
| ECC-VAL-039 | Active | Validate DV_BOOLEAN — anything allowed | passed |
| ECC-VAL-040 | Active | Validate DV_BOOLEAN — only true allowed | passed |
| ECC-VAL-041 | Active | Validate DV_BOOLEAN — only false allowed | passed |
| ECC-VAL-042 | Active | Validate DV_IDENTIFIER — all pattern | passed |
| ECC-VAL-043 | Active | Validate DV_IDENTIFIER — all list | passed |
| ECC-VAL-044 | Active | Validate DV_TEXT — open | passed |
| ECC-VAL-045 | Active | Validate DV_TEXT — list | passed |
| ECC-VAL-046 | Active | Validate DV_CODED_TEXT — open | passed |
| ECC-VAL-047 | Active | Validate DV_CODED_TEXT — local codes | passed |
| ECC-VAL-048 | Active | Validate DV_CODED_TEXT — ext term | passed |
| ECC-VAL-049 | Active | Validate DV_ORDINAL — open | passed |
| ECC-VAL-050 | Active | Validate DV_ORDINAL — constraint | passed |
| ECC-VAL-051 | Active | Validate DV_SCALE — open | passed |
| ECC-VAL-052 | Active | Validate DV_SCALE — constraint | passed |
| ECC-VAL-053 | Active | Validate DV_COUNT — open | passed |
| ECC-VAL-054 | Active | Validate DV_COUNT — range | passed |
| ECC-VAL-055 | Active | Validate DV_COUNT — list | passed |
| ECC-VAL-056 | Active | Validate DV_QUANTITY — open | passed |
| ECC-VAL-057 | Active | Validate DV_QUANTITY — property | passed |
| ECC-VAL-058 | Active | Validate DV_QUANTITY — property units | passed |
| ECC-VAL-059 | Active | Validate DV_QUANTITY — property units mag | passed |
| ECC-VAL-060 | Active | Validate DV_PROPORTION — open | passed |
| ECC-VAL-061 | Active | Validate DV_PROPORTION — ratio | passed |
| ECC-VAL-062 | Active | Validate DV_PROPORTION — unitary | passed |
| ECC-VAL-063 | Active | Validate DV_PROPORTION — percent | passed |
| ECC-VAL-064 | Active | Validate DV_PROPORTION — fraction | passed |
| ECC-VAL-065 | Active | Validate DV_PROPORTION — integer fraction | passed |
| ECC-VAL-066 | Active | Validate DV_PROPORTION — any fraction | passed |
| ECC-VAL-067 | Active | Validate DV_PROPORTION — ratio range | passed |
| ECC-VAL-068 | Active | Validate DV_INTERVAL<DV_COUNT> — open | passed |
| ECC-VAL-069 | Active | Validate DV_INTERVAL<DV_COUNT> — lower upper | passed |
| ECC-VAL-070 | Active | Validate DV_INTERVAL<DV_COUNT> — lower upper list | passed |
| ECC-VAL-071 | Active | Validate DV_INTERVAL<DV_QUANTITY> — open | passed |
| ECC-VAL-072 | Active | Validate DV_INTERVAL<DV_QUANTITY> — upper lower | passed |
| ECC-VAL-073 | Active | Validate DV_INTERVAL<DV_DATE_TIME> — open | passed |
| ECC-VAL-074 | Active | Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint | passed |
| ECC-VAL-075 | Active | Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range | passed |
| ECC-VAL-076 | Active | Validate DV_INTERVAL<DV_DATE> — open | passed |
| ECC-VAL-077 | Active | Validate DV_INTERVAL<DV_DATE> — lower upper constraint | passed |
| ECC-VAL-078 | Active | Validate DV_INTERVAL<DV_DATE> — lower upper range | passed |
| ECC-VAL-079 | Active | Validate DV_INTERVAL<DV_TIME> — open | passed |
| ECC-VAL-080 | Active | Validate DV_INTERVAL<DV_TIME> — lower upper constraint | passed |
| ECC-VAL-081 | Active | Validate DV_INTERVAL<DV_TIME> — lower upper range | passed |
| ECC-VAL-082 | Active | Validate DV_INTERVAL<DV_DURATION> — open | passed |
| ECC-VAL-083 | Active | Validate DV_INTERVAL<DV_DURATION> — constraint | passed |
| ECC-VAL-084 | Active | Validate DV_INTERVAL<DV_DURATION> — range | passed |
| ECC-VAL-085 | Active | Validate DV_INTERVAL<DV_ORDINAL> — open | passed |
| ECC-VAL-086 | Active | Validate DV_INTERVAL<DV_ORDINAL> — constraint | passed |
| ECC-VAL-087 | Active | Validate DV_INTERVAL<DV_SCALE> — open | passed |
| ECC-VAL-088 | Active | Validate DV_INTERVAL<DV_SCALE> — constraint | passed |
| ECC-VAL-089 | Active | Validate DV_INTERVAL<DV_PROPORTION> — open | passed |
| ECC-VAL-090 | Active | Validate DV_INTERVAL<DV_PROPORTION> — ratio | passed |
| ECC-VAL-091 | Active | Validate DV_INTERVAL<DV_PROPORTION> — unitary | passed |
| ECC-VAL-092 | Active | Validate DV_INTERVAL<DV_PROPORTION> — percentage | passed |
| ECC-VAL-093 | Active | Validate DV_INTERVAL<DV_PROPORTION> — fraction | passed |
| ECC-VAL-094 | Active | Validate DV_INTERVAL<DV_PROPORTION> — integer fraction | passed |
| ECC-VAL-095 | Active | Validate DV_INTERVAL<DV_PROPORTION> — ratio range | passed |
| ECC-VAL-096 | Active | Validate DV_DURATION — open | passed |
| ECC-VAL-097 | Active | Validate DV_DURATION — fields | passed |
| ECC-VAL-098 | Active | Validate DV_DURATION — range | passed |
| ECC-VAL-099 | Active | Validate DV_DURATION — fields range | passed |
| ECC-VAL-100 | Active | Validate DV_TIME — open | passed |
| ECC-VAL-101 | Active | Validate DV_TIME — constraint | passed |
| ECC-VAL-102 | Active | Validate DV_TIME — range | passed |
| ECC-VAL-103 | Active | Validate DV_DATE — open | passed |
| ECC-VAL-104 | Active | Validate DV_DATE — constraint | passed |
| ECC-VAL-105 | Active | Validate DV_DATE — range | passed |
| ECC-VAL-106 | Active | Validate DV_DATE_TIME — open | passed |
| ECC-VAL-107 | Active | Validate DV_DATE_TIME — constraint | passed |
| ECC-VAL-108 | Active | Validate DV_DATE_TIME — range | passed |
| ECC-VAL-109 | Active | Validate DV_PARSABLE — open | passed |
| ECC-VAL-110 | Active | Validate DV_PARSABLE — value formalism | passed |
| ECC-VAL-111 | Active | Validate DV_MULTIMEDIA — open | passed |
| ECC-VAL-112 | Active | Validate DV_MULTIMEDIA — media type | passed |
| ECC-VAL-113 | Active | Validate DV_URI — open | passed |
| ECC-VAL-114 | Active | Validate DV_URI — pattern | passed |
| ECC-VAL-115 | Active | Validate DV_URI — list | passed |
| ECC-VAL-116 | Active | Validate DV_EHR_URI — open | passed |
| ECC-VAL-117 | Active | Validate DV_EHR_URI — pattern | passed |
| ECC-VAL-118 | Active | Validate DV_EHR_URI — list | passed |
| ECC-VAL-119 | Active | Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected) | passed |

## DEM — Demographic service (24 cases, 24 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-DEM-001 | Active | Demographic person create | passed |
| ECC-DEM-002 | Active | Demographic person get | passed |
| ECC-DEM-003 | Active | Demographic person get by version | passed |
| ECC-DEM-004 | Active | Demographic person update | passed |
| ECC-DEM-005 | Active | Demographic person delete | passed |
| ECC-DEM-006 | Active | Demographic person get deleted | passed |
| ECC-DEM-007 | Active | Demographic person get absent | passed |
| ECC-DEM-008 | Active | Demographic person update bad if match | passed |
| ECC-DEM-009 | Active | Demographic agent create | passed |
| ECC-DEM-010 | Active | Demographic agent get | passed |
| ECC-DEM-011 | Active | Demographic agent delete | passed |
| ECC-DEM-012 | Active | Demographic group create | passed |
| ECC-DEM-013 | Active | Demographic group get | passed |
| ECC-DEM-014 | Active | Demographic group delete | passed |
| ECC-DEM-015 | Active | Demographic organisation create | passed |
| ECC-DEM-016 | Active | Demographic organisation get | passed |
| ECC-DEM-017 | Active | Demographic organisation delete | passed |
| ECC-DEM-018 | Active | Demographic role create | passed |
| ECC-DEM-019 | Active | Demographic role get | passed |
| ECC-DEM-020 | Active | Demographic role delete | passed |
| ECC-DEM-021 | Active | Demographic create bad body | passed |
| ECC-DEM-022 | Active | Demographic versioned party get | passed |
| ECC-DEM-023 | Active | Demographic versioned party revision history | passed |
| ECC-DEM-024 | Active | Demographic person tags | passed |

## ADM — Admin service (6 cases, 6 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-ADM-001 | Active | Admin EHR delete | passed |
| ECC-ADM-002 | Active | Admin EHR delete absent | passed |
| ECC-ADM-003 | Active | Admin EHR delete idempotent | passed |
| ECC-ADM-004 | Active | Admin EHR delete all | passed |
| ECC-ADM-005 | Active | Admin EHR delete all partial | passed |
| ECC-ADM-006 | Active | Admin EHR delete all empty | passed |

## SEC — Security / authorization (2 cases, 2 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-SEC-001 | Active | Unauthenticated request to a protected route is refused (401) | passed |
| ECC-SEC-002 | Active | Regular credential on an ADMIN-only route is forbidden (403) | passed |

## SIG — Version signing (5 cases, 5 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-SIG-001 | Active | Version signing — digest present | passed/passed |
| ECC-SIG-002 | Active | Version signing — digest recomputes | passed |
| ECC-SIG-003 | Active | Version signing — all kinds | passed |
| ECC-SIG-004 | Active | Version signing — client verbatim | passed |
| ECC-SIG-005 | Active | Version signing — pgp verifies | skipped |

## MSG — Messaging (10 cases, 10 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-MSG-001 | Active | EHR Extract — export whole EHR (export_ehrs) | skipped |
| ECC-MSG-002 | Active | EHR Extract — spec-driven export (export_ehr_extracts) | skipped |
| ECC-MSG-003 | Active | EHR Extract — export of unknown EHR fails | skipped |
| ECC-MSG-004 | Active | EHR Extract — import whole-EHR clone reusing source id (import_ehr) | skipped |
| ECC-MSG-005 | Active | EHR Extract — import whole EHR into a caller-fixed id (import_ehr) | skipped |
| ECC-MSG-006 | Active | EHR Extract — import into a duplicate target id fails | skipped |
| ECC-MSG-007 | Active | EHR Extract — import extract into an existing EHR (import_ehr_extract) | skipped |
| ECC-MSG-008 | Active | TDD — import a TDD as a committed COMPOSITION (import_tdd) | skipped |
| ECC-MSG-009 | Active | TDD — import rejects malformed / non-TDD / unknown EHR / unknown template | skipped |
| ECC-MSG-010 | Active | TDD — batch import commits all, fail-fast on error (import_tdds) | skipped |

## TS — Terminology-server integration (9 cases, 9 active)

| ECC id | Status | Title | Last run |
|---|---|---|---|
| ECC-TS-001 | Active | TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET | passed |
| ECC-TS-002 | Active | TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes | passed |
| ECC-TS-003 | Active | TERMINOLOGY expand (bundle) — explicit code merged with the expansion (matches list) | passed |
| ECC-TS-004 | Active | TERMINOLOGY expand — unknown value set rejected (400) | passed |
| ECC-TS-005 | Active | TERMINOLOGY expand — unknown service_api rejected (400) | passed |
| ECC-TS-006 | Active | TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured | skipped |
| ECC-TS-007 | Active | TERMINOLOGY expand (FHIR) — terminology-server timeout is a server fault (500) | skipped |
| ECC-TS-008 | Active | TERMINOLOGY expand (FHIR) — terminology-server 5xx is a server fault (500) | skipped |
| ECC-TS-009 | Active | TERMINOLOGY expand (FHIR) — malformed terminology response is a server fault (500) | skipped |

