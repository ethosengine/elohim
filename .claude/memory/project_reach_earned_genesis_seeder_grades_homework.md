---
id: project-reach-earned-genesis-seeder-grades-homework
name: project_reach_earned_genesis_seeder_grades_homework
description: "Public surfaces (landing/manifesto/lamad LMS) aren't reach-exempt — genesis+seeder is the bootstrap authoring authority that GRADES them as having earned commons/public reach; gate stays universal"
metadata: 
  node_type: memory
  type: project
  originSessionId: 02a31f41-5f1f-4600-9d3f-7d0c9a341c9c
cites:
  - bootstrap-steward-authority-frame-design | the frame establishing genesis/seeder as bootstrap authoring authority that grades reach, not a sovereign | sha256:6fb209d2628d39bb
---

Reach is **earned at authoring** ([[project_social_reach_nervous_system]]). The reach gate is **universal and always on** for all content — there is no "exclusion list" or "public-surface bypass."

genesis + seeder is the **bootstrap authoring authority**: it is us "grading our own homework." We are opinionated that our own public protocol website (landing + its manifesto) and the lamad LMS have *already earned* `commons`/`public` reach, so we **seed them at that earned grade**. They pass the gate **legitimately because they hold the earned reach**, not because they're exempt.

**Why:** the protocol takes a values-forward stance that reach must be earned ([[project_values_forward_disclosure_accountability]]); a bypass/exemption mechanism would undercut that — the public surfaces must demonstrate the same gate honoring an earned grade that all other content flows through. "Public by design" = "we graded it public," not "we skip the gate."

**How to apply:** when a public surface is wrongly gated (canonical symptom 2026-05-30: `GET /db/content/manifesto` → `403 requiredReach=community` while the landing that summarizes it is `commons`), the fix lane is **seed-data re-grading** (correct the earned-reach value in `genesis/data/**` / seeder), NOT a code gate bypass. Reach test matrices should be framed as content-type × *earned-reach* × viewer-posture, proving the universal gate HONORS the grade in both directions (open AND denied). Watch for vocabulary drift across three coexisting reach vocabularies — projection `commons`; content `public`/`community`; doorway REACH.md `commons/regional-private/local/private` — `community` may be a non-canonical leaked token. See [[feedback_schema_first_ioc]] for the canonical reach enum being schema-authoritative.
