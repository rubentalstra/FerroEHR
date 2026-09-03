---
name: license-busl
description: Since 2026-09-03 FerroEHR's own code is under the Business Source License 1.1 (not MIT), Licensor and copyright holder Ruben Talstra; the reason is a possible sale and a ban on resale/hosted offering by others
metadata:
  type: project
---

On 2026-09-03 the owner moved the project's own code from MIT to the Business Source License 1.1 (issue #3068), after a talk with a company interested in buying and reselling FerroEHR under its own name that did not want MIT. Parameters: Licensor Ruben Talstra; Additional Use Grant written for a CDR (tightened the same day at the owner's direction: production use is free for Non-Commercial Purposes only, i.e. research, teaching, personal use, non-profit or public bodies outside the course of a business; any other production use incl. paid health care, a hosted/managed/embedded service for third parties, and for-fee distribution alone or inside another product need a commercial licence); Change Date four years per version; Change License Apache 2.0. Releases through v4.0.17 and crates through 0.0.56 stay MIT as published. Contributions carry an inbound relicensing grant (CONTRIBUTING.md) so the work stays one work under one licensor.

**Why:** the owner wants no resale and no SaaS by others without permission, and a buyer wants exclusivity and a named copyright holder. No OSI licence can express that (OSD criteria 1 and 6); BUSL beat Elastic 2.0 (allows resale of copies), FSL (two-year conversion), PolyForm Shield (permanent, low recognition), AGPL (does not stop SaaS).

**How to apply:** new first-party files carry `SPDX-FileCopyrightText: Ruben Talstra` + `SPDX-License-Identifier: BUSL-1.1` (spec crates `BUSL-1.1 AND Apache-2.0`); never call the project open source, say source-available; `scripts/checks/licensing-declarations.sh` fails on a stale MIT claim. The grant wording is the owner's legal text: propose changes, never edit it on your own. FerroTERM's grant wording differs (it permits "products and services you operate"); do not copy it here. See [[owner-work-style]].
