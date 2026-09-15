---
name: opt14-format-is-the-xsd
description: Adjudication 2026-09-15 (#3395/#3401) — the OPT 1.4 XML format is defined by the ITS-XML schema alone; where the AOM 1.4 abstract model and OpenehrProfile.xsd disagree (ordinal symbol value), the schema wins for document validity; Archie/EHRbase leniency is never an argument
metadata:
  type: feedback
---

On #3395 (Archetype Designer omits `<value>` on ordinal symbols) I first wrote "our bug" from the AOM 1.4 model (`ORDINAL.symbol: CODE_PHRASE`) plus EHRbase's lenient parser. The owner pushed back: "are you really sure this is a defect on our side? the archie implementation is too lenient". Corrected adjudication: no docs text defines the OPT 1.4 XML format; `OpenehrProfile.xsd` is its only released definition, it serialises `C_DV_ORDINAL.list` as RM `DV_ORDINAL` (mandatory `DV_CODED_TEXT.value`), so a conformant document carries the element; CKM's empty `<value/>` is valid, AD's omission is not. FerroEHR keeps refusing; the model/schema mismatch is the upstream report #3401; the remedy goes into the error message.

**Why:** strictness is a hard rule (accept exactly what the released format admits); the ITS-REST "docs text wins over the OAS" ruling applies to a format the docs text DOES define, which is not the case for OPT 1.4 XML.

**How to apply:** for OPT 1.4 documents the XSD is the oracle; a disagreement with the AOM text is filed upstream, never resolved by accepting; never cite another implementation's behaviour as a reason to accept or refuse. Related: [[console-wire-spec-check]], [[rewrite-not-inherited-code]].
