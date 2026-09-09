# Privacy & data minimisation

`[privacy]` is what the clinical side refuses to hold. Precedence, the
environment-name grammar, and file discovery are on the
[Configuration reference](configuration.md) index.

<!-- toc -->

## What this section is for

FerroEHR keeps clinical content and demographic parties in separate database
schemas reached by separate roles. That separation is only worth something if
the clinical side does not carry the identity anyway — in the subject
reference, in a party proxy's name, or in free text. This section is the three
rules that keep it out.

No openEHR specification governs any of them. The Reference Model leaves
`EHR_STATUS.subject.external_ref` open and only advises against identifying
content on a party proxy (`PARTY_IDENTIFIED`: "Should not be used to include
patient identifying information"). GDPR
[Art. 4(5) and Art. 25(2)](https://eur-lex.europa.eu/eli/reg/2016/679/oj) are
what make it a duty; this section is where a deployment states its side of it.

All three rules run on the write path, before anything is stored. A refusal is
`422 Unprocessable Entity` with one `validationErrors[]` entry per finding,
each naming the RM path and the rule that matched. **A refusal never echoes the
offending value** — it would otherwise travel into the response body, the
access log and the traces, which is the leak the rule exists to prevent.

```toml
[privacy]
subject_namespaces = []
allow_identified_parties_in_ehr = false

[privacy.identifier_scan]
mode = "strict"
rules = ["fi-hetu", "gb-nhs-number", "nl-bsn", "no-fodselsnummer", "se-personnummer"]
patterns = []
```

## `[privacy]`

| Key | Type | Default | Description |
|---|---|---|---|
| `subject_namespaces` | list of strings | `[]` | The pseudonymisation domains this deployment issues subject pseudonyms in. |
| `allow_identified_parties_in_ehr` | bool | `false` | Accept `PARTY_IDENTIFIED` / `PARTY_RELATED` carrying `name` or `identifiers` in clinical content. |

### The subject reference

`EHR_STATUS.subject` is a `PARTY_SELF` whose optional `external_ref` points at
a demographic service. Once `subject_namespaces` names at least one namespace,
that reference must name one of them and carry a UUID as its `id.value`:

```toml
[privacy]
subject_namespaces = ["urn:ferroehr:pseudonym"]
```

```bash
FERROEHR__PRIVACY__SUBJECT_NAMESPACES=urn:ferroehr:pseudonym,mpi.example
```

**Empty is the default, and it leaves the rule out of force.** A pseudonym
namespace is a deployment fact — which service mints the tokens — and there is
no name a server could invent for an operator. Declaring one is the same act as
turning the rule on. An EHR with no subject reference at all is unaffected
either way: that is the default `EHR_STATUS` the openEHR REST API describes,
and it carries nothing to minimise.

Both spellings of a UUID's case are accepted; the braced, URN and unhyphenated
forms are not, because the promoted subject column is compared as text and four
spellings of one pseudonym would be four subjects.

### Identified parties

A `PARTY_IDENTIFIED` or `PARTY_RELATED` inside clinical content may carry
`external_ref` but not `name` and not `identifiers`. This covers the
composer, participations, the health care facility and feeder-audit party
slots. `PARTY_IDENTIFIED`'s own validity rule is satisfied by `external_ref`
alone, so every party proxy stays expressible — it points into the demographic
domain instead of restating the identity.

The commit's own `AUDIT_DETAILS.committer` is not clinical content and is never
touched by this rule.

A deployment that needs the names on the clinical side sets:

```toml
[privacy]
allow_identified_parties_in_ehr = true
```

which is announced at boot with a warning naming what it permits.

## `[privacy.identifier_scan]`

Every string leaf of every clinical write is checked against the active
identifier rules.

| Key | Type | Default | Description |
|---|---|---|---|
| `mode` | `strict` \| `warn` | `strict` | `strict` refuses the write; `warn` accepts it and records a warning naming the RM path and the rule. |
| `rules` | list of rule keys | every rule the build ships | The national personal-identifier rules to scan for. An unknown key is a boot error listing the shipped keys. |
| `patterns` | list of regular expressions | `[]` | Extra patterns for the kinds no build can ship a rule for. |

### The shipped rules

Each rule transcribes the checksum its own issuing register publishes. A kind
whose current algorithm could not be established from its own register is not
shipped, because a rule that guesses is worse than an absent one: it tells an
operator their data was scanned.

| Key | Jurisdiction | Identifier | Published by |
|---|---|---|---|
| `fi-hetu` | FI | henkilötunnus | [Digital and Population Data Services Agency](https://dvv.fi/en/personal-identity-code) |
| `gb-nhs-number` | GB | NHS Number | [NHS Data Model and Dictionary](https://www.datadictionary.nhs.uk/attributes/nhs_number.html) |
| `nl-bsn` | NL | burgerservicenummer | [Rijksdienst voor Identiteitsgegevens, Logisch Ontwerp BSN](https://www.rvig.nl/logisch-ontwerp-bsn) |
| `no-fodselsnummer` | NO | fødselsnummer | [Skatteetaten](https://skatteetaten.github.io/folkeregisteret-api-dokumentasjon/nytt-fodselsnummer-fra-2032/) |
| `se-personnummer` | SE | personnummer | [Skatteverket](https://www4.skatteverket.se/rattsligvagledning/edition/2020.2/330245.html) |

Two are deliberately absent. The Danish CPR-nummer has been
[issued without its modulus-11 control since 2007](https://cpr.dk/cpr-systemet/personnumre-uden-kontrolciffer-modulus-11-kontrol),
and those numbers are fully valid, so a checksum rule would pass most recent
ones through while reporting Denmark as covered. The Belgian
rijksregisternummer's modulo-97 check is widely reproduced but no definition
published by the Rijksregister itself could be retrieved.

### Why every rule is active by default

Clinical data crosses borders: a Dutch hospital receives referrals carrying a
Norwegian fødselsnummer. An operator who has not configured the scanner is
exactly the operator who has not yet worked out which identifiers their content
carries, so the default covers all of them.

The two costs are not symmetric. A false positive is a loud `422` naming the RM
path and the rule, which an operator answers by narrowing `rules` or moving to
`warn`. A false negative is a national identifier stored on the clinical side
indefinitely.

A deployment that knows its jurisdictions narrows the list:

```toml
[privacy.identifier_scan]
rules = ["no-fodselsnummer"]
```

```bash
FERROEHR__PRIVACY__IDENTIFIER_SCAN__RULES=no-fodselsnummer
```

The boot log states which rules are active, so a jurisdiction that is not
covered is visible rather than assumed.

### False positives, and what narrows them

Every checksum accepts some fraction of random digit runs, and clinical content
is full of numbers. Three things narrow it:

- **A digit run must be delimited** — bounded by something other than a letter,
  a digit or an underscore. Without that, every hex digest and long numeric
  identifier produces hits at the checksum's own rate. It also means a UUID can
  never match any rule: its groups are 8, 4, 4, 4 and 12 characters, so no
  delimited run of 9, 10 or 11 digits occurs in one, and the opaque subject
  pseudonym the boundary mandates passes unconditionally.
- **`CODE_PHRASE.code_string` is skipped.** A terminology code system's
  identifiers are digit runs by construction, so this one slot is where the
  collision is systematic rather than incidental. Measured over this
  repository's vendored corpora: of the 30 distinct nine-digit values that
  satisfy the Dutch elfproef, 27 are SNOMED CT concept identifiers, all of them
  in a code slot. The carve-out names an RM slot, not a country.
- **Each rule narrows itself with whatever structure it has.** Two independent
  control digits (`no-fodselsnummer`, 1 in 120), an embedded date
  (`se-personnummer`, 1 in 139 with the Luhn check), a non-numeric token shape
  (`fi-hetu`, 1 in 31 among tokens of that shape). `nl-bsn` at 1 in 11 is the
  loosest shipped rule, which is why the code-slot carve-out matters most to
  it. Every figure here is measured, over 100 000 random tokens of each rule's
  own shape, by a test that fails if a rule drifts from the rate it publishes.

A deployment measuring its own content runs `warn` first, reads the recorded
findings, then moves to `strict`.

### Local patterns

Medical-record numbers, payer references and postcode forms have no single
issuing register to transcribe, so a deployment declares them itself as Rust
regular expressions:

```toml
[privacy.identifier_scan]
patterns = [
  "\\bMRN-[0-9]{6}\\b",
  "(^|[^0-9A-Za-z])[1-9][0-9]{3} ?[A-Z]{2}[^0-9A-Za-z]{1,3}[0-9]{1,4}([^0-9A-Za-z]|$)",
]
```

The second refuses a Dutch postcode paired with a house number — the pair is
what identifies a household; a postcode alone does not. A pattern that does not
compile is a boot error naming it.

## Interaction with the audit trail

The ATNA Patient-Number participant is filled from the promoted subject column,
so constraining the subject reference to an opaque pseudonym is also what keeps
the audit trail and its forwarding sinks free of the identity. The contribution
outbox carries no subject at all. Both are covered by a CI test that fails when
any subject identifier other than the opaque UUID appears in an outbox payload,
an ATNA message or a trace record.
