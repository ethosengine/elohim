# Doorway Manifests

Doorway is the agency on-ramp — separate from the P2P StatefulSet, it provides five services in one Deployment:

1. **DNS/TLS gateway** — stateless HTTP/WebSocket routing
2. **Bootstrap/Signal** — agent discovery + WebRTC relay for the DHT
3. **Projection cache** — serves DHT content via REST API
4. **Identity host** — custodial agent keys for users transitioning from web2 to P2P
5. **Recovery registrar** — relationship-based identity recovery contracts

Connects to the edgenode StatefulSet via ClusterIP service for conductor and storage access.
Scaling is via the conductor pool behind doorway, not doorway replicas.

See `doorway/SCALING.md` for the full scaling model (graduation flywheel, conductor pool, human topology).

## Che-facing keyless peer-client posture (deploy-posture honesty gate)

The keyless Eclipse Che peer-client (`genesis/docs/superpowers/specs/2026-06-26-che-keyless-governed-peer-client-design.md`, Slice 1) drives `distribute_shards` on the live mesh **through a doorway** carrying only a portal JWT — no on-device key. That governance spine is only honest if the doorway it drives runs a coherent, non-dev posture. A **Che-facing** doorway deploy MUST set:

| Env | Required value | Why |
|-----|----------------|-----|
| `CHE_FACING` | `1` | Arms the boot-refusal below. |
| `DELEGATES_COMPUTE_OP_GATE` | `enforce` | `POST /db/content` + `/db/content/bulk` are gated per-request (fail-closed) by storage's `POST /api/v1/authorize-operation`; a revoked/absent `delegates-compute` commitment → 403. |
| `DEV_MODE` | unset / `false` | `DEV_MODE=true` disables auth and forces JWT passthrough (`config.rs::jwt_secret` → `dev-only-insecure-secret`), which would make the whole gate theater. |
| `JWT_SECRET` | present, **≥32 chars** | The credential carrier is the portal JWT; a weak/absent secret defeats performer-binding. |
| `ALLOW_SEED_SHARD_MANIFEST` | unset | Governed lighting only — never hand-seed shard manifests on a Che-facing node. |
| `ALLOW_SEED_DELEGATES_COMPUTE` | `1` **only** on the dogfood node that intentionally seeds the Matthew→Che self-contract; unset on any production-class node | The flag IS the dev/prod boundary (the `seed_shard_manifest` honesty model). |

**Runtime enforcement is already wired** — not a documentation promise. `doorway-service/src/config.rs::Args::validate()` (the `che_facing` block) **refuses to boot** on any incoherent combination: `CHE_FACING=1` with the gate not `enforce`, with `DEV_MODE=true`, or with a missing/`<32`-char `JWT_SECRET`. An incoherent Che-facing deploy fails fast at startup (`main.rs` → `process::exit(1)`) rather than serving an ungoverned surface.

**Load-bearing tension — read before designating a Che-facing node.** All four doorway deploys in this directory (`alpha.yaml`, `prod.yaml`, `staging.yaml`, `staging-read.yaml`) currently set `DEV_MODE: "true"` (alpha for the `FIXTURE_ONLY` steward-grant surface; the others carry it as drift against alpha's own "NEVER set on staging/prod" note). **None of them is a valid Che-facing `enforce` node as-is.** The Che-facing dogfood deploy is therefore a **distinct posture** (a dedicated deploy, or alpha shedding `DEV_MODE` — which would break the portal-handoff fixture scenarios). Choosing/provisioning that node, and flipping it to `enforce`, is an **operator/architect decision** — and it is coupled to first seeding the bounded `delegates-compute` commitment **on that node** (else `enforce` 403s all `POST /db/content` there). That live step is held with the live-mesh legs; this README intentionally flips **no** live manifest.
