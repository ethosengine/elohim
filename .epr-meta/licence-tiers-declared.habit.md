---
epr-habit-version: 1
id: licence-tiers-declared
invariant: >-
  Every tracked first-party crate and package declares a licence from the three tiers the steward
  ruled on 2026-11-06 — primitives MPL-2.0, services that serve people AGPL-3.0, everything else
  Apache-2.0 under the root LICENSE — and CAL-1.0 appears nowhere but the conductor fork. A
  reusable path with no licence is all-rights-reserved and cannot be relied on, so a missing field
  is a refusal, never a warning. The canon prose under genesis/docs/content/elohim-protocol/ is
  CC BY-SA 4.0 (owned by no one, constitution I.1; the steward its holder of record, bound by the
  irrevocable grant and the covenant's no-other-terms commitment) and carries that LICENSE in its
  own directory.
status: green
active: false
checks:
  - "node genesis/orchestrator/licence-tiers-check.mjs (walks tracked Cargo.toml [package] tables and package.json manifests; refuses a missing field, a licence outside the allowed set, CAL-1.0 outside the fork, a named tier member on the wrong tier, a zome that is not MPL-2.0, or a missing LICENSE/LICENSES text)"
guard: >-
  Never change a tier's licence or move a crate between tiers in a sweep; that is a steward ruling
  recorded in the decision spec first. Never add a licence to upstream code held in the tree
  (conductor fork, sophia, vendor/); their terms come with them. Never weaken the canon's share-alike
  to a gift licence from this habit, and never record a transfer of the canon's copyright to an
  owner: the constitution says it is owned by no one, and a holder of record is not an owner in its
  sense. Never call the holder a trustee unless an instrument makes them one. This file states no
  legal advice.
refs:
  - LICENSING.md
  - genesis/docs/superpowers/specs/2026-11-06-licence-tiers-decision.md
  - genesis/orchestrator/licence-tiers-check.mjs
  - deny.toml
  - COVENANT.md
retire-when: "cargo-deny's licence check and a package-manifest lint run the same refusals in the pre-push gate and CI, including the canon directory's LICENSE, so this script has no reader of its own"
---

DELTA 2026-11-06 (born GREEN on the sweep that created it): steward ruling — primitives MPL-2.0 (6 crates CAL-1.0 → MPL-2.0; 10 DNA zomes and 6 SDK domain type crates given MPL-2.0 from no field), services AGPL-3.0 (`doorway-service` and `steward/node` Apache-2.0 → AGPL-3.0, both already linking AGPL `elohim-storage`), Apache-2.0 for the rest (`elohim-cache-core`, `elohim-peer-fabric`, sweettest, manifest-hygiene given a field; 26 tracked `package.json` manifests given `"license": "Apache-2.0"`; root `LICENSE` + `LICENSES/{Apache-2.0,MPL-2.0,AGPL-3.0}.txt` added from the SPDX texts). `deny.toml` allows AGPL-3.0. The first check run refused 7 manifests the field census had passed because they carried the *wrong* field: `elohim/sdk/epr-ts` CAL-1.0 → MPL-2.0 (the TS twin of `elohim-epr`), five `app/elohim-library/projects/*` and `elohim/sdk` MIT/UNLICENSED → Apache-2.0. Second run: `licence-tiers-check: 85 Cargo.toml packages, 33 package.json manifests, 0 refusal(s)`, exit 0. Every edited Cargo workspace parses (`cargo metadata --no-deps --offline` ok ×24; the node-registry profile warning predates this). Green is the field census only: compatibility of the combined binaries and the canon's own licence are counsel's and the steward body's, not this check's. README blind-reader loop on the Contributing and License passages (rule `readme-blind-reader-review`): 3 rounds, 3 fresh readers. R1: 1 correctness · 6 interpretability · 1 preference (2 interpretability on the licence text, fixed: tier lookup path, "canon prose" undefined; the rest predate this pass and were named to the steward). R2 READY: 1 correctness · 5 interpretability · 3 preference (4 on the licence text, fixed: directory table, forking/quoting of the unlicensed prose, inbound = outbound, dev/main). R3 READY: 0 correctness · 4 interpretability · 2 preference (fixed: deliberate Apache fallback named, first-party MIT OR Apache row split from upstream forks, LICENSING.md and COVENANT.md linked and glossed; deferred to the steward: the scope of quotation from the unlicensed prose, which only its licence can settle). Closed by the author's judgment at round 3: every remaining finding is either outside the licence passages or a licence decision the steward holds.

DELTA 2026-11-07 (GREEN; the deferred finding is ruled): steward ruling on the canon prose — "the elohim commons should own it", not a gift — lands as CC BY-SA 4.0: `genesis/docs/content/elohim-protocol/LICENSE` and `LICENSES/CC-BY-SA-4.0.txt` from the Creative Commons legal code; the check now refuses a missing canon LICENSE. The README's quotation gloss is gone with the carve-out. Refined the same day on the steward's correction: the trust does not end in a council-derived owner — the canon is "owned by no one, including its authors" (constitution I.1), owed to the constitution and held in trust by the elohim and the people who tend it; the steward is the copyright's holder of record because a licence needs a signatory. Every text that had promised a transfer to a steward body now records a commitment instead. Residue: whether another legal person should ever be holder of record, and whether the commitment should get an instrument. README loop continued for the canon paragraph: R4 READY (0 correctness · 4 interpretability · 1 preference, read against the pre-correction text; the two that survived the correction — is the trust legal or moral, and "pillar papers" narrowing the directory rule — fixed: no deed exists, the irrevocable grant is the floor, every document under the path is covered; CC text location and checker coverage stated). R5 REVISE (1 correctness · 4 interpretability · 1 preference): the correctness finding stands — irrevocability covers only what is already shared and does not stop dual-licensing or private additions, and "trustee" without a deed is a costume. Fixed: "trustee of record" → "holder of record" in every text; the gap named; the covenant's no-other-terms commitment recorded as what closes it, labelled a public pledge not an instrument; succession by inheritance stated with the grant surviving; "commercially or not" added; referent and the overloaded "trust" disambiguated. Closed by the author's judgment at round 5 (5 rounds, 5 fresh readers): the remaining question — whether the pledge should get an instrument — is the steward's, recorded in the spec's open items.
