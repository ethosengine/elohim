---
name: elohim-dna-as-sdk-boundary
description: "elohim DNA is the API/SDK contract; lamad is one implementation conforming to it. Bridge calls target the SDK role (elohim), not an implementation (lamad). Contract should be enforced so boundary leaks fail at compile/validate time."
metadata: 
  node_type: memory
  type: project
  originSessionId: ca911629-dfdd-46f5-8bb1-e936364bea8e
---

elohim DNA is the **protocol-core API/SDK boundary** — `Content` entries, attestations, governance-actions, the consolidated coordinator pattern. lamad is **one implementation** of that contract (the LMS pillar). Other implementations exist (imagodei, mishpat, infrastructure pillar manifests) or could exist — each declares "I implement contract X from elohim DNA."

**Why:** The consolidation sprint moved 22+ legacy entry types onto a single `Content` discriminator pattern owned by elohim DNA. The pillars (lamad, imagodei, mishpat, etc.) became consumers of that pattern via manifests + bridge calls, not authors of their own entry types. lamad happens to be the *first* implementation built against the contract, so historical naming (e.g. `happ.yaml` role names) treats lamad as if it were the SDK — but it isn't. lamad is a tenant of the SDK.

**Implications for the bridge pattern:**

- `CallTargetCell::OtherRole("elohim".into())` in imagodei is **semantically correct** — bridges target the SDK, not the lamad implementation.
- If `happ.yaml` declares the elohim-DNA-housing role as `lamad`, that's a pre-existing naming bug from the consolidation — happ.yaml inherited lamad's name when the consolidation landed because the original DNA was called lamad.
- The fix is to either (a) rename the role in happ.yaml to `elohim` (cheap, immediate) or (b) split protocol-core off into a dedicated `elohim` DNA bundle distinct from `lamad`'s LMS-specific pieces (larger refactor, cleaner long-term).

**Implications for compile-time contract enforcement:**

- Replace every hardcoded `"elohim"` / `"lamad"` string in bridge calls with a Rust **constant** (e.g. `pub const ELOHIM_DNA_ROLE: &str = "elohim";`) shared between pillars. Future renames surface every reference at compile time.
- Pillar manifests already declare "I implement contract X from elohim DNA" via `manifest.json`'s `attestations` + `governance-actions` blocks. The validator harness should reject:
  - Pillar code that creates entries directly when the contract says to bridge through elohim
  - Boundary-leak imports (lamad-specific types appearing in imagodei zome)
  - Manifest entries that reference attestation/governance-action kinds not declared by any contract
- p2p-design-gate has the right hook surface; extending it to scan for bridge-target-role anti-patterns is the natural place to add the check.

**How to apply:**

- When writing bridge calls in any pillar zome, target the SDK role via a shared constant; never hardcode an implementation name.
- When validating a pillar manifest, ensure every declared attestation/governance-action kind is also recognized by the elohim DNA's coordinator (i.e. it's a discriminator the consolidated `Content` pattern accepts) — not an entry type the pillar invents.
- When naming a DNA role in `happ.yaml`, name it after the SDK contract, not the first implementation. Adding a new implementation should not require renaming the role.
- Pillar code that needs to access elohim-DNA primitives should go through `CallTargetCell::OtherRole(ELOHIM_DNA_ROLE)`, never `CallTargetCell::OtherRole("lamad")` or any implementation name.

Linked: `project_attestation_consolidation_sprint_state.md`, `project_doorway_full_facilitator_sprint.md` (similar SDK-vs-implementation distinction at the doorway boundary), `feedback_schema_first_ioc.md` (schemas are the contract; code conforms).
