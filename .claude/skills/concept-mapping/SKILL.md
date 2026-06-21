---
name: concept-mapping
description: Use when an Elohim question is framed in conventional-computing terms — a Linux service/syscall, an OS or hardware primitive, a network protocol, a cloud/hyperscaler service, a k8s primitive — and you need the elohim-native analog AND where the concern is placed. Triggers include "what's the elohim equivalent of X", "where does X live here", "I know Linux/hardware — map my mental model".
---

# Concept Mapping

## Overview

Given a concern framed in terms a developer already knows (Linux / hardware / protocol / cloud), name **(a)** its elohim-native analog and **(b)** where it is *placed* — which seam, and which library/pillar/crate/manifest implements it. The analogy and the placement are **one move**: *"you know this as X → in elohim it's Y, at seam Z, in W."* Then flag where the analogy **breaks** (the inversion). The Rosetta Stone for learning Elohim by analogy.

## When to Use

- "What's the elohim equivalent of `systemd` / `cron` / KMS / `mmap` / TCP / a k8s Deployment?"
- "I know Linux/hardware — where does this concern go in elohim?"
- As `app-port`'s analogy + placement step over a prior-art app's concepts.

Not for: a concept that's already elohim-native (no translation needed); a pure code lookup (just read it).

## The Map

Route through the concern-routing atlas: `genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md`.
- **§8 hyperscaler crosswalk** — the cloud-service → elohim-seam mapping (the analog).
- **§3 seam catalog** — the seam's Home (the concrete library/pillar/crate/manifest = the placement).
- **§7 planes** — control / data / projection (which plane the concept lives on).
- **§8 inversion** — what has no equivalent *either way* (where the analogy must break).

## Method

1. **Decompose, then classify.** A conventional noun often bundles several concerns — "encrypted backups" = encryption + durability + scheduling; "nginx + TLS" = reverse-proxy + cert. **Split the concept into atomic sub-concerns FIRST**, and map EACH to its own seam — one row per sub-concern, not per noun. Then classify each by its job (compute / storage / scheduling / secrecy / networking / auth / …).
2. **Find the analog** — for a *cloud/hyperscaler* concept, use the atlas **§8 crosswalk**; for an *OS / Linux / protocol* primitive (which §8 does NOT cover — it's a hyperscaler crosswalk), use the **§3 seam catalog** + the Quick-Reference table below. Either yields the elohim seam + primitive. Name the **participation track** (T1–T4, sourced from §3 / Figure B — *not* §8) **only if the concept is track-resident**; if it lives on a **plane** (confidentiality §3.13, temporal §3.14, resource-governance §3.15, or the control/data/projection planes §7), name the **plane**, not a track.
3. **Place it** — the seam's §3 **Home** names the concrete library/pillar/crate/manifest. *Don't stop at the seam; name the placement.* **If no §3 Home exists** (e.g. a TLS cert / CA — operator-infra, not a protocol seam), say so explicitly: *"out-of-seam / operator-infra."* That is a **placement gap**, distinct from an inversion (next).
4. **Flag the break** — state where the analogy misleads, and name the **inversion**: the social/governance/trust/recovery plane has **no** conventional equivalent (the protocol's point), and some conventional things (a custodial KMS, a CA-issued cert, a central control plane) have **no** elohim equivalent **by design**. Keep "no §3 Home" (a placement gap, step 3) distinct from "no equivalent by design" (an inversion).

## Quick Reference — common mappings

| You know it as | In elohim | Seam |
|---|---|---|
| `systemd` always-on service | always-on node / hub role | hub cluster ops (§3.12) |
| `cron` / Step Functions / a scheduler | the temporal plane | temporal (§3.14) |
| KMS / secrets / encryption-at-rest | the confidentiality plane | confidentiality (§3.13) |
| IAM / auth | agent keys + attestation + capability grant | SDK + `delegates-compute` (§3.5) |
| throttling / quotas / autoscaling | resource governance (elohim-operator) | resource-governance (§3.15) |
| object storage / S3 | blob + quilt RS(N,K) | peer-hoster / custody (§3.10, §3.1) |
| DNS / CDN | doorway (names→CIDs, cache) | doorway projection (§3.9) |
| a plugin / extension | bridge (compile-time) or mod/plugin (runtime) | bridge (§3.6) / mod-plugin (§3.4) |
| "add an app" | a domain app-manifest | SDK + app-manifest (§3.5/§3.7) |
| a container image / package | the OS/packaging seam | OS/packaging (§3.2) |
| reverse proxy / TLS edge | doorway projection (the cert itself = operator-infra, out-of-seam) | doorway (§3.9) |

Derive any others live from the atlas — do **not** maintain a separate analogy table (it would drift; the atlas is the source).

## Composition

- `app-port` calls this to translate a prior-art app's concepts into elohim seams + placements.
- Reads the **durable** atlas (analogies + placements don't go stale — unlike build-state, which lives in the dated assessments).

## Common Mistakes

- **Treating a conventional noun as atomic** — split compound concepts first (one row per sub-concern), or you'll emit one seam per noun and miss half the map.
- **Mapping to a seam but not naming the placement** — finish the move: the concrete library/crate/manifest.
- **Forcing a participation track onto a plane-resident concept** (confidentiality, temporal, resource-governance) — name the plane instead.
- **Conflating a placement gap with an inversion** — "no §3 Home" (TLS/CA = operator-infra) is a gap; "no equivalent by design" is the inversion. Say which.
- **Treating the analogy as exact** — always state where it breaks.
- Reaching for build-state ("is it built?") — that's `atlas-grounding` + the dated assessments; this skill maps *concept → place*, not *what's wired*.
