# elohim-peer-fabric — shared peer-traffic spine (pure logic)

Write-once defense + ranking logic consumed by BOTH `doorway-service` and `elohim-storage`, feature-gated
per node role. Pattern mirrors `elohim-facings`: **pure logic, no diesel** — the dependency graph is the
boundary (a `use diesel;` here won't compile; that compile-failure IS the enforcement).

- `guard`: `assess(store, clock, cfg, source) -> Verdict` (Allow/Shape/Challenge/Deny + ban). Runtimes
  implement `GuardStore` (SQLite for storage; in-memory/edge for doorway) + `Clock`.
- `score`: `rank(candidates, min_capability)` and `select_diverse(..)` — capability×headroom×attested-RTT×
  delivery×bond ranking with graceful degradation (unknown RTT → neutral; all-saturated → empty ⇒ caller sheds).

**Features (node role):** `edge-defense` (doorway guard), `peer-defense` (storage guard), `serve-routing`
(storage score), `identity-routing` (doorway conductor-axis, fast-follow).

Spec: `genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md`.
Do NOT add I/O deps here (no diesel/serde/tokio) — keep it pure.
