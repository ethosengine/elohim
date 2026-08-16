---
name: seed-workflow
description: Validate, seed, and verify Elohim content through the safe root just workflow; includes the no-dry-run safety boundary, statistics, diagnosis, and snapshot cautions.
metadata:
  runtime: codex
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/seed-workflow.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/seed-workflow"
---

# Content Seeding Workflow

Use the root developer CLI for the common path. Content is validated from
`genesis/data/lamad`, written through doorway/storage, and anchored through the
Holochain provenance layer.

## Safe workflow

```bash
just seed validate       # schema validation; never writes
just dev start           # isolated local stack
just seed apply local    # write content
just seed stats          # inspect resulting counts
```

For alpha, use `just seed apply alpha`. Production seeding remains CI-owned.

## Important safety boundary

There is no content-seed dry-run today. The retired `seed:dry-run` and
`seed:validate` npm aliases passed flags that `seed.ts` did not parse, so they
could execute a real seed. Never recreate a preview command until the seeder
has a tested non-writing execution mode.

`just seed validate` runs `src/schema-validation.ts` directly and is the
canonical non-writing check.

## Seeder CLI

From `genesis/seeder`, the content seeder supports:

```bash
pnpm exec tsx src/seed.ts --limit 50
pnpm exec tsx src/seed.ts --ids=a,b,c
pnpm exec tsx src/seed.ts --content-only
pnpm exec tsx src/seed.ts --paths-only
pnpm exec tsx src/seed.ts --force
pnpm exec tsx src/seed.ts --conductor-for <human-id>
```

Connection inputs are `DOORWAY_URL`, `STORAGE_URL`, and
`HOLOCHAIN_ADMIN_URL`. Alpha also requires the configured API key.

## Validation

```bash
just seed validate
cd genesis/seeder
pnpm run validate:verbose
pnpm run validate:all
```

Validation checks JSON shape and the domain validators declared by the seeder.
It does not prove that remote services are reachable.

## Verification and diagnosis

```bash
just seed stats
just seed diagnose
curl -s http://localhost:8888/db/stats | jq .
```

The local ports file is
`elohim/holochain/local-dev/.hc_ports`. Remote profiles should set explicit
URLs rather than reading it.

## Snapshot workflow

Snapshot commands live in `genesis/seeder/package.json` and operate under
`elohim/holochain/local-dev`. Treat restore and clean as destructive. Inspect
status/list first, and do not wrap snapshot operations in the public root CLI
until their conductor lifecycle is covered by a regression test.

## Recovery facts

- The bulk content path is skip-on-exists; a stale existing row is not an
  update. Remove or deliberately update it before reseeding.
- A content row without a DHT anchor can appear in graph/tag views but return
  404 from the canonical content route.
- Paths are `ContentNode`s; there is no `/db/paths` endpoint.
- Seed data must use lamad manifest formats such as `sophia-quiz-json`, not the
  broad protocol format `interactive`.

## Canonical files

| File | Purpose |
|---|---|
| `justfile` | safe public seed entrypoint |
| `genesis/seeder/src/seed.ts` | content seeding implementation |
| `genesis/seeder/src/schema-validation.ts` | non-writing validation |
| `genesis/seeder/src/stats.ts` | content statistics |
| `genesis/seeder/src/diagnose.ts` | connection/content diagnosis |
| `genesis/seeder/src/snapshot.ts` | snapshot lifecycle |
