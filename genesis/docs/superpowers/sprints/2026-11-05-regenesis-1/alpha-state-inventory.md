---
id: regenesis-1-alpha-state-inventory
---

# Alpha state inventory — Phase 4 preparation

Matthew uses this inventory to decide what must be preserved before alpha receives fresh DNA hashes and keys. Phase 4 requires his attestation before recasting state. The clean break does not preserve old chains through migration. (`genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/plan.md:159–170`)

This is a repository inventory. No network commands, live checks, reseeding, resets, builds, or exports were run. **Unknown does not mean disposable.** In the table, **no** applies only to the named repo-backed definitions or bytes. It does not establish that alpha has no later changes.

Evidence filenames below resolve to the full paths in their row. `plan.md` means the sibling sprint plan.

| what alpha holds | source of truth (repo path or "live only") | reseeded by (command) | unique? (yes/no/unknown) | evidence (file:line) |
|---|---|---|---|---|
| Declared alpha peer personas: Matthew, Jessica and James use `operations/edge/performance`; Adam, Gertrude, Susan and Eve use `remote`. Actual running agents remain unverified. | `genesis/orchestrator/data/deployments.json`; `genesis/manifests/cluster-state.yaml`; `genesis/Jenkinsfile` | `just seed apply alpha conductors`, then `just seed apply alpha humans`, with the intended conductor roster configured | no — persona definitions | `deployments.json:14–174,253–330,404–438`; `cluster-state.yaml:15–18,43–66`; `Jenkinsfile:164–195`; `justfile:533–535` |
| Suspended personas: Pete, Terrance, Frank, Caleb, Daniel, Emma and Nancy. Any retained cells, PVC data or account changes are unverified. | Definitions: `genesis/orchestrator/data/deployments.json`. Retained state: **live only**. | Registration skips suspended humans. Account-package imports can still include their declared presence. | unknown | `deployments.json:182–241,338–393,446–457`; `genesis/seeder/src/seed-humans.ts:520–525`; `genesis/seeder/src/deployment-registry.ts:4–16` |
| Staging and production humans/agents. Environment tag files exist, but they do not establish a current live roster or a backup of alpha. The conductor seeding code targets alpha. | `genesis/orchestrator/environments/staging.env`; `genesis/orchestrator/environments/prod.env`; `genesis/Jenkinsfile`. Actual state: **live only**. | No cross-environment restoration established. Operator to confirm scope. | unknown | `staging.env:1–9`; `prod.env:1–9`; `Jenkinsfile:194–195` |
| Conductor cells and source chains per DNA: `lamad`, `infrastructure`, `imagodei`, `mishpat`, `node_registry`. Each role declares immediate creation. Installed cell counts, keys and non-fixture entries are unknown for every role. | Bundle: `elohim/holochain/dna/elohim/workdir/happ.yaml`. Existing cells and chains: **live only**. | Phase 4 operator recast and hApp installation; identity fixtures via `just seed apply alpha conductors`. These create new state. | unknown | `happ.yaml:24–90`; `genesis/seeder/src/seed-conductor-identities.ts:2–8,35–52`; `plan.md:160,170,245` |
| Repo-backed content bodies, learning paths and blob-pack entries. These are seed inputs, not a census of alpha. | `genesis/data/lamad/content/`; `genesis/data/lamad/paths/`; `genesis/blobs/manifest.json`; `genesis/seeder/src/seed.ts` | `just seed apply alpha content`; blob pack builder: `pnpm --dir genesis/seeder run genesis:pack` | no — repo-backed material | `seed.ts:342–344,1131–1140,1434–1442`; `genesis/seeder/src/genesis-pack.ts:3–20`; `genesis/seeder/package.json:13`; `justfile:520–531` |
| Additional storage blobs, content revisions and historical probe blobs. Their presence and recoverability are unverified. CI deliberately creates a new propagation blob for each build. | **live only** unless matched to repo bytes or another retained artifact | No general replay established. Content seeding reads its declared inputs. | unknown | `genesis/Jenkinsfile:1959–1965`; `genesis/seeder/src/seed.ts:1131–1140`; `genesis/seeder/src/blob-manager.ts:275–313` |
| Doorway accounts and account state. Fixture registration covers declared, non-suspended stages, including hosted Maria, Ronald and Ezra. Extra registrations, changed credentials and existing key bindings are unverified. | Fixture definitions: `genesis/data/humans/`; `genesis/data/account-packages/`. Current accounts: **live only**. | `just seed apply alpha humans`; `just seed apply alpha accounts`. These are separate registration and package-import operations. | unknown | `genesis/seeder/src/seed-humans.ts:157–169,234–238,497–525`; `genesis/seeder/src/seed-accounts.ts:2–4,156–160,255`; `justfile:535–538` |
| Custody manifests, shard locations and commitments. Default fixture commitments can be recreated. Existing holder assignments, promises and their history are not established by those defaults. | Fixture recipe: `genesis/seeder/src/seed-commitments.ts`. Actual manifests and commitments: **live only**. | `just seed apply alpha commitments`, with the intended blob and conductor inputs. No general custody-manifest restore established. | unknown | `elohim/elohim-storage/src/db/diesel_schema.rs:436–447,822`; `seed-commitments.ts:4–17,27–36,459–498`; `genesis/Jenkinsfile:1973–2001`; `justfile:541` |
| DTPR letter source, dated 2026-10-14, and its seed record. The README documents publication through alpha and doorway-alpha. This material is present in the repo. | `genesis/docs/content/elohim-protocol/letters/dtpr/`; `genesis/docs/content/elohim-protocol/letters/dtpr.zip`; `genesis/data/lamad/content/letter-to-dtpr.json` | Build with `python3 -I build.py` from the letter directory; then `just seed apply alpha content '' letter-to-dtpr` using the completed new-profile tooling | no — repo copy | `dtpr/source/letter.md:3–7`; `dtpr/README.md:11–20,24–32,65–76`; `letter-to-dtpr.json:33–41`; `justfile:524–525`; `plan.md:168` |
| Any live-published material absent from the seed inputs, including a newer DTPR version or publication history. The repo copy does not prove equality with alpha. | **live only** until compared or exported | No general replay established. Operator to confirm. | unknown | `genesis/docs/content/elohim-protocol/letters/dtpr/README.md:65–76`; `genesis/seeder/src/seed.ts:1131–1140`; `plan.md:159` |

## Recovery limits

The committed DNA baseline contains **packed** hashes. It explicitly forbids treating them as installed cell hashes. Phase 4 requires agreement on the five effective installed hashes before seeding. (`elohim/holochain/dna/dna-hashes.baseline:13–19`; `plan.md:160`)

The current storage-target fallback names only Matthew, Adam and Jessica. The current reset script deletes `content.db` through the old container target. The sprint requires its replacement. These sources do not prove that all seven peers will be cleared or restored. (`genesis/Jenkinsfile:98–110,1568–1576`; `genesis/scripts/ci/reset-storage-pod.sh:20–22`; `plan.md:159`)

The household prologue rebuilds a selected corpus, identities, hosted fixtures and custody fixtures. It is not an alpha backup. (`app/elohim-app/scripts/hc-mesh-prologue.sh:288–293,447–473,521–555`)

Brit notes, device keys, steward-space heads, the held eprfs tier store and preserved evidence remain outside the recast. Deleting them is a stop condition. (`plan.md:172,280`)

## Unique content that must survive

No row is proven to contain alpha-only content. Resolve every **unknown** before attestation:

- **Suspended-persona state:** operator to confirm whether retained cells, accounts or storage contain anything beyond fixtures. Preserve any exceptions before clearing them. (`deployments.json:183,212,231,339,361,383,447`; `plan.md:170`)
- **Staging and production state:** operator to confirm their live rosters and exclusion from this recast. Do not count them as alpha backups without evidence. (`genesis/orchestrator/environments/staging.env:1–9`; `genesis/orchestrator/environments/prod.env:1–9`)
- **All five DNA cell histories:** operator to confirm what must survive. Export path: **operator to confirm**. Reseeding creates profiles; it does not recover the old chains. (`happ.yaml:24–90`; `genesis/seeder/src/seed-conductor-identities.ts:35–52`; `plan.md:170`)
- **Additional storage bytes and revisions:** preserve unmatched payloads with their content records and metadata. Export path: **operator to confirm**. (`genesis/seeder/src/seed.ts:1131–1140`; `genesis/Jenkinsfile:1959–1965`)
- **Doorway account changes and extra registrants:** preserve required account and identity-binding state before recasting cells. Export path: **operator to confirm**. (`genesis/seeder/src/seed-humans.ts:234–238,327–360`)
- **Custody state:** preserve required manifests, holder records and commitments together with their referenced bytes. Export path: **operator to confirm**. (`elohim/elohim-storage/src/db/diesel_schema.rs:436–447,822`; `genesis/seeder/src/seed-commitments.ts:4–12`)
- **Live publications:** compare alpha’s DTPR version with the repo copy and identify other publications outside the seed inputs. Preserve differences and required publication history. Export path: **operator to confirm**. (`genesis/docs/content/elohim-protocol/letters/dtpr/README.md:65–76`; `genesis/seeder/src/seed.ts:1131–1140`)

Attestation (Matthew): ____ — alpha holds no unique content beyond the rows marked above / exports taken: ____