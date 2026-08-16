# Genesis Seeder

Validates and seeds the structured corpus under `genesis/data/lamad` through
doorway/storage, with Holochain providing the provenance anchors.

## Prerequisites

Use the repository development environment, with `just`, Node.js, pnpm, Rust,
and the Holochain CLI available. Run `pnpm install` once from the repository
root before the first seed. `jq` is optional and is used only to pretty-print
the example HTTP response below.

## First successful local run

From the repository root:

```bash
just seed validate       # schema check; never writes
just dev start           # isolated conductor + storage + doorway
just seed apply local    # seed the corpus
just seed stats          # verify counts
```

The positional seed parameters are `action [profile] [scope] [limit]`. Named profiles are
`local` and `alpha`; specialist corpus facets use a scope instead of a new
environment-suffixed npm alias, for example `just seed apply alpha humans` or
`just seed validate local deployments`. The optional numeric limit applies to
the `content` scope, for example `just seed apply alpha content 10`.

There is intentionally no seed dry-run. The retired `--dry-run` and
`--validate-only` aliases were ignored by `seed.ts` and could perform a real
write. Do not add a preview command until the seeder has a tested non-writing
execution mode.

## Focused seeder interface

Specialists can run the implementation directly from `genesis/seeder`:

```bash
pnpm exec tsx src/seed.ts --limit 50
pnpm exec tsx src/seed.ts --ids=a,b,c
pnpm exec tsx src/seed.ts --content-only
pnpm exec tsx src/seed.ts --paths-only
pnpm exec tsx src/seed.ts --force
pnpm exec tsx src/seed.ts --conductor-for <human-id>
```

Supported connection inputs are `DOORWAY_URL`, `STORAGE_URL`, and
`HOLOCHAIN_ADMIN_URL`. Local conductor ports are read from
`elohim/holochain/local-dev/.hc_ports`. Alpha seeding is available through
`just seed apply alpha`; production seeding is CI-owned.

## Verify and diagnose

```bash
just seed stats
just seed diagnose
curl -s http://localhost:8888/db/stats | jq .
```

Snapshot operations remain specialist commands in this package. `restore` and
`clean` are destructive; inspect `snapshot:status` or `snapshot:list` first.

## Data Structure

The seeder expects JSON files in:

```
genesis/data/lamad/
├── corpus.json        # Corpus trust declaration (see below)
├── content/           # Concepts and content nodes
│   ├── *.json
│   └── epic/
│       └── *.json
└── paths/             # Learning paths
    └── *.json
```

## Corpus trust declaration

**What this solves.** Everything under `genesis/data/` is test fixture data — it is
authored in a repository, not agreed between real peers. When a peer receives that
content it has to decide how hard to verify it, and doing full per-item verification
on a few thousand fixture rows is what makes a local or staging mesh take hours to
settle. The fix is not to skip verification, it is to let a corpus *say what it is*:
"I am preproduction fixture data, granted by this steward, for this corpus." A peer
can then accept the whole corpus on one declared grant instead of re-deciding for
every item.

That declaration lives **on the artifact, in `corpus.json` at the corpus root**. The
seeder reads it, verifies it, and mints the grant **once per deployment** as a
*stakes declaration* — a small JSON record saying which trust stage applies to which
corpus, on whose authority. The running node reads that record instead of guessing.

The same file also carries the corpus's **realism rung** — an existing convention in
this repository requiring every body of seed data to state how real it is, and why:

| Rung | Means |
|------|-------|
| `0` | Rows written straight into storage. No signature, no agent — the data exists, nobody claims to have done it. |
| `1` | Written on behalf of a persona, attributed but not signed by them. |
| `2` | Signed by a shared or synthetic key rather than the persona's own. |
| `3` | Authored by the persona's **own** conductor as a real signed event. The only rung at which an inter-party agreement, consent act, or identity binding is honest. |

The two declarations belong together — both answer *"what is this data, honestly?"* —
so they share one file rather than two competing ones. Fuller treatment of the rung
convention lives in the household-lattice design under `genesis/docs/`
(`.../architecture/2026-06-04-qahal-epr-household-lattice-design.md` §5).

```json
{
  "id": "genesis-lamad",
  "title": "Genesis Lamad Corpus",
  "environment": "preproduction",
  "realism": {
    "rung": 0,
    "why": "Corpus body stays rung 0 forever — replication carries the bytes."
  },
  "trustBootstrap": {
    "stage": "simulacra",
    "grantor": "human-adam-firstman",
    "scope": "genesis-lamad"
  }
}
```

| Field | Required | Meaning |
|-------|----------|---------|
| `id` | yes | Corpus id — also the **scope** the grant applies to |
| `title` | yes | Human-readable corpus label |
| `environment` | yes | Tier: `preproduction` \| `production` |
| `realism.rung` | yes | Realism-ladder rung `0`–`3` |
| `realism.why` | yes | Why this corpus sits at that rung |
| `trustBootstrap.stage` | when granting | `simulacra` \| `bootstrap` \| `coordinated` \| `enforced` |
| `trustBootstrap.grantor` | when granting | The steward granting it (see below) |
| `trustBootstrap.scope` | no (defaults to `id`) | Must equal `id` — a corpus grants only over itself |

**About the grantor.** A grant is always granted *by someone* — never by "the
repository". For the corpora committed here that someone is `human-adam-firstman`,
the fixture persona who stewards the genesis content. If you add a corpus stewarded
by a different persona, name that persona instead; do not copy this value by reflex.

### The four stages

`trustBootstrap.stage` names a **`NetworkStage`** — how much scrutiny a node applies
to content in this scope, from most permissive to strictest:

| Stage | Means |
|-------|-------|
| `simulacra` | Preproduction fixture data. Verification is cheapest; a peer may accept a whole declared corpus at once. Legal **only** under `environment: preproduction`. |
| `bootstrap` | The safe fallback. Structural checks only, no trust assumptions. This is what a node uses when nothing is declared. |
| `coordinated` | Trust mediated by the wider network's coordination. |
| `enforced` | Full enforcement — the real, live setting. |

Certain protections never get cheaper at any stage (family and household messages,
constitutional signatures, counter-evidence); the stage only prices *how hard a node
works to re-verify ordinary content*.

Write the stage lowercase and exact. `"dev"`, `"dev-mode"` and `"staging"` are **not**
vocabulary and are rejected. The stage is never derived from a `DEV_MODE` flag, from
the hostname, or from the fact that the bytes came out of this repository — it is
declared or it does not exist. The variants are defined in
`elohim/elohim-storage/src/trust/stage.rs`.

### The three rules

1. **Declared, never inferred.** `simulacra` is reached only by positive
   declaration on the artifact, verified at seed time.
2. **Never a default.** No `corpus.json`, or a `corpus.json` with no
   `trustBootstrap`, means **nothing is minted** — the runtime resolves
   `NetworkStage::Bootstrap` via `StakesProvenance::BootstrapDefault`. Absence is
   safe and silent.
3. **Malformed is fatal.** A declaration that exists must be valid. An unknown
   stage, a missing grantor, or `simulacra` under `environment: production` aborts
   the seed run with an error naming the offending field. There is no silent
   downgrade path.

### What the seed run emits

On a successful run over a corpus that declares a grant, the seeder logs:

```
stakes declaration: simulacra for corpus genesis-lamad by human-adam-firstman
```

and writes the stakes declaration to `genesis/seeder/stakes-declaration.json` (a
per-run artifact, not committed; its canonical filename is registered in
`genesis/orchestrator/build-artifacts.json` under `genesis.stakesDeclaration`, which
is where the build pipeline looks for it).

That file is the whole mint: **one grant per deployment covering one corpus**, rather
than one trust decision per content item. Nothing else about the seed run changes.

The running node does not read this file yet — there is no HTTP route that accepts a
policy record today, so the seeder writes it and logs it. The exact JSON contract, and
the `STAKES-DECLARATION-SEAM-Q6` marker showing where the POST goes once that route
exists, are documented at the bottom of `src/corpus-trust.ts`.

Validate every declaration under `genesis/data/` without seeding anything:

```bash
cd genesis/seeder
pnpm run validate:corpora     # also runs as part of pnpm run validate:all
```

It prints one line per corpus and exits non-zero if any declaration is invalid.
Finding no declarations at all is not an error — absence is the safe path.

### Concept JSON Schema

```json
{
  "id": "unique-concept-id",
  "title": "Display Title",
  "content": "Markdown content...",
  "contentFormat": "markdown",
  "contentType": "concept",
  "description": "Brief description",
  "summary": "Card preview text",
  "tags": ["tag1", "tag2"],
  "relatedNodeIds": ["other-concept-id"],
  "estimatedMinutes": 5,
  "thumbnailUrl": "https://..."
}
```

### Path JSON Schema

```json
{
  "id": "path-id",
  "title": "Learning Path Title",
  "description": "Path description",
  "difficulty": "beginner",
  "estimatedDuration": "2 hours",
  "chapters": [
    {
      "id": "chapter-1",
      "title": "Chapter Title",
      "modules": [
        {
          "id": "module-1",
          "title": "Module Title",
          "sections": [
            {
              "id": "section-1",
              "title": "Section Title",
              "conceptIds": ["concept-1", "concept-2"]
            }
          ]
        }
      ]
    }
  ]
}
```

## Troubleshooting

### Connection Errors

**Error:** `UnknownMessageType: incoming message has unknown type - error`

This usually means the seeder is connecting to the wrong interface. Make sure:
- Only `HOLOCHAIN_ADMIN_URL` is set (not `HOLOCHAIN_APP_URL`)
- API key is included in the URL: `?apiKey=...`

**Error:** `Failed to connect to admin WebSocket`

- Check the conductor is running
- Verify the URL and port
- For remote: ensure Doorway gateway is deployed and healthy

### Auth Errors

**Error:** `401 Unauthorized`

- Verify API key is correct
- Check the key has appropriate permissions (authenticated vs admin)

### Port Range Errors

**Error:** `Invalid route` or connection rejected

- Doorway has a port range limit (4445-65535)
- Ensure the conductor's app interface port is within range
