# Agency-Phase Registration Design

**Date:** 2026-04-10
**Status:** Draft
**Context:** Genesis seeding process fails because doorway's `handle_register` routes all `create_human` zome calls through a singleton ZomeCaller bound to the operator's conductor. This creates Matthew's Human profile for every registrant.

## Problem

Doorway treats every registration as a "hosted" user — calling `create_human` on the operator's conductor via a singleton `ZomeCaller`. This is architecturally wrong:

1. **Node/device humans** (Adam, Jessica, Frank, Pete, Timothy) have their own conductors with their own agent keys. `create_human` should run on *their* conductor, not the operator's.
2. **Hosted humans** (Nancy) should be provisioned on the operator's conductor, but `create_human` runs *before* provisioning — hitting the wrong conductor.
3. **The operator** (Matthew) is the only case where the current flow is correct.

The root cause is that registration doesn't model the human's *agency phase* — where they are in the graduated stewardship journey from visitor to sovereign peer.

## Agency Phases

The protocol defines a graduated journey of increasing self-direction:

| Phase | Identity Created By | Conductor | Doorway's Role |
|-------|-------------------|-----------|----------------|
| `visitor` | Nobody | None | Serves commons content |
| `hosted` | Operator's conductor (custodial) | Operator's conductor | Facilitates registration; operator's conductor holds custody |
| `device` | Human via Tauri app | Own local conductor | Ingress/recovery registration only |
| `node` | Human via own conductor | Own always-on conductor | Ingress/recovery registration only |
| `doorway` | Operator | Own conductor | N/A — IS the operator |

Key insight: agency phase and compute capacity are loosely coupled but correlated. Higher agency means more compute contribution; the `doorway` phase is a transitional super-phase that should diminish as the network matures from web2 to full protocol.

## Active Alpha Roster

7 humans with active conductors, modeling peer diversity:

| Human | Agency Phase | Conductor | Role in Network |
|-------|-------------|-----------|-----------------|
| Matthew | `doorway` | `elohim-matthew-alpha` | Doorway operator; conductor holds custody for hosted humans |
| Adam | `node` | `elohim-adam-alpha` | Genesis peer, alpha-pinned |
| Eve | `device` | `elohim-eve-alpha` (simulated) | Device steward, Adam's household |
| Jessica | `device` | `elohim-jessica-alpha` (simulated) | Device steward in Matthew's household |
| Pete | `device` | `elohim-pete-alpha` (simulated) | Device steward |
| Timothy | `device` | `elohim-timothy-alpha` (simulated) | Device steward |
| Nancy | `hosted` | `elohim-matthew-alpha` (pooled) | Hosted by Matthew's doorway |

The remaining 26 humans are story personas (`visitor` or unset) — not active participants on alpha.

## Changes

### 1. humans.json: Add `agencyPhase` field

```json
{
  "id": "human-adam-firstman",
  "displayName": "Adam",
  "agencyPhase": "node",
  ...
}
```

Valid values: `doorway`, `node`, `device`, `hosted`, `visitor`.
Absent or null treated as `visitor` (inactive persona).

### 2. Seeder: Send `agencyPhase` in register payload

The seeder reads `agencyPhase` from humans.json and includes it in the request:

```json
POST /auth/register
{
  "identifier": "nancy@test.elohim.host",
  "password": "Test2026!",
  "displayName": "Nancy",
  "agencyPhase": "hosted"
}
```

Seeder skips `visitor` humans entirely — no registration needed.

### 3. Doorway `handle_register`: Branch on agency phase

The registration flow changes based on `agencyPhase`:

**`doorway` (operator bootstrap):**
- Existing ZomeCaller flow — operator's own conductor
- `create_human` + DB credential record
- This is the first registration in a fresh deployment

**`hosted` (custodial):**
- Provision on operator's conductor first (reorder: provision before zome call)
- `create_human` on the provisioned conductor
- DB credential record — operator's conductor holds custodial keys
- Matthew is the custodian

**`node` / `device` (ingress registration):**
- NO `create_human` call — identity already exists on their conductor
- `provisioner.find_existing_app()` locates their conductor across `CONDUCTOR_URLS`
- DB credential record only — for ingress and account recovery
- Doorway does not create their identity, only facilitates access

**`visitor` / missing:**
- Reject — visitors don't register

### 4. Node/device identity creation (outside doorway)

For alpha, node/device conductors need their Human profiles created independently of doorway. Two options:

**Approach: Direct seeder call.** The seeder calls each node/device conductor's app interface directly to create the Human profile *before* registering with doorway. This models the real-world flow — the human sets up their identity on their own conductor, then registers with a doorway for ingress. The seeder derives conductor URLs from the deployment topology (same `CONDUCTOR_URLS` that doorway uses).

Doorway doesn't create their identity — it only registers them for ingress/recovery.

### 5. Recovery path for DB-cleared-but-conductor-intact

The existing 503 "Agent already has a Human profile" recovery (commit 65ab7b2c) continues to work for the `doorway` phase. For `node`/`device`, the recovery is simpler — doorway never calls `create_human`, so the zome error never occurs. For `hosted`, the reordered flow (provision first) ensures `create_human` targets the right conductor.

## Generation Chain

```
humans.json (agencyPhase per human)
  → Genesis pipeline
    → K8s StatefulSet manifests (for node/device humans)
    → CONDUCTOR_URLS for doorway ConfigMap
    → Seeder payloads with agencyPhase
```

Doorway receives its conductor topology as configuration (`CONDUCTOR_URLS`, existing `ConductorRegistry`). No new manifest-loading feature in doorway — configuration, not code.

## P2P Design Note

`agencyPhase` is operational metadata in the Genesis seeder and doorway's MongoDB — it is NOT a new DHT entry type. The Human entry type in imagodei already exists and is unchanged. No new zome functions, entry types, or link types are introduced. The `/auth/register` route already exists; this design changes its internal branching logic. Agency phase describes the human's relationship to infrastructure, not protocol-notarized state — a human either runs a node or doesn't, and the network can observe this directly.

## What This Does NOT Change

- Doorway's `ConductorRegistry` and `AgentProvisioner` — used as-is
- `CONDUCTOR_URLS` configuration — already supports multi-conductor
- `discover_existing_agents` startup flow — already discovers agent→conductor mappings
- The graduation flywheel — hosted humans still migrate off operator's conductor over time
- Multiple doorway registration for resilience — unchanged, orthogonal to agency phase

## Success Criteria

1. `seed-humans` completes with all 7 active humans registered (6 non-hosted skip `create_human`)
2. Each human's DB record points to the correct conductor
3. Login works for all 7 humans after seeding
4. Re-seeding after DB clear recovers correctly per phase
5. The 26 inactive humans are skipped cleanly
