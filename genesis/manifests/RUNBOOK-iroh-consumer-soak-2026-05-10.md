# RUNBOOK — Iroh Consumer-Grade Soak (Gate #9)

**Soak window opened:** 2026-05-10
**Soak window closes when:** 7-day per-archetype trial complete and all
plane decisions recorded in §"Decision log" below
**Gate:** #9 of iroh cutover (see master plan `2026-05-10-iroh-delivery-master.md`)

---

## Purpose

The alpha cluster (Gate #7) proves iroh works on server-grade nodes with stable
routing. Gate #9 proves iroh works — or documents that it does not — on the
three device archetypes that represent the majority of human-scale contributors:

1. **Phone / cellular** — Android/iOS device on LTE/5G; strict NAT, CGNAT,
   battery-constrained, connection drops common.
2. **Chromebook / school WiFi** — 802.11ac or older behind a typical
   school/library firewall; UDP often blocked; ICE-unfriendly environment.
3. **Residential CGN** — desktop or laptop behind a carrier-grade NAT; no
   UPnP; IPv4 only in many cases.

**The structural rule (per spec line 518):**

> "If iroh fails any of these, the affected plane stays libp2p-canonical
> for that device class **permanently**."

This is a one-way decision per (plane, device-archetype) combination.
A plane that fails iroh on a given archetype gets `libp2p` as its permanent
default for that archetype in `peer_transport_manifest.capability_level`.
It does NOT block iroh from being tried on other archetypes.

---

## Sub-runbook 1 — Phone / Cellular

### Device setup

- Device: Android 12+ or iOS 16+ with a SIM card (not WiFi-only).
- Network: LTE or 5G; disable WiFi for the duration of the test.
- elohim-storage: run as a Tauri mobile sidecar or as a local HTTP sidecar
  reachable via `adb forward` / iOS USB tethering.
- `TRANSPORT_BACKEND=dual-stack` in the sidecar config.

### Test procedure (7 days)

Each day, run a blob-fetch sequence against an alpha-cluster peer:

```bash
# From the phone-connected dev machine (adb tunnel or USB):
for PLANE in blob gossip sync epr epr-atom shard view-fed identity-handshake trust; do
  curl -s "http://localhost:8090/p2p/status/$PLANE" >> phone-cellular-day$(date +%d).log
done
```

Log every attempt: the `/p2p/status/{plane}` endpoint returns
`{ transport: "iroh" | "libp2p" | "none", ok: bool }` per plane.

### Failure definition

A plane **fails** for this archetype if either:
- `transport: "none"` appears on any attempt for that plane (no fallback found), OR
- iroh is never selected (`transport` is always `"libp2p"`) across all 7 days.
  This indicates iroh cannot traverse the cellular NAT and libp2p is doing all
  the work — iroh has no foothold.

A plane **passes** if iroh is selected on ≥99% of attempts over 7 days.

---

## Sub-runbook 2 — Chromebook / School WiFi

### Device setup

- Device: Chromebook (Linux beta enabled) or any laptop on a school/library
  WiFi network.
- Network characteristics: UDP port 443 and 53 may be the only open ports.
  QUIC (iroh's transport) uses UDP/443 by default — this is the critical test.
- `TRANSPORT_BACKEND=dual-stack` in elohim-storage sidecar config.

### Test procedure (7 days)

```bash
# On the Chromebook, run elohim-storage sidecar and check per-plane transport:
for PLANE in blob gossip sync epr epr-atom shard view-fed identity-handshake trust; do
  curl -s "http://localhost:8090/p2p/status/$PLANE" \
    >> chromebook-school-wifi-day$(date +%d).log
done
```

### Special consideration: UDP blocking

Many school firewalls block all UDP except DNS (53). If iroh cannot use
QUIC/UDP, it must fall back. The dual-stack selector should handle this
automatically. If it does not (transport stuck at "none"), investigate
whether elohim-storage honours `IROH_NO_UDP=1` as an escape hatch.

A plane **fails** for this archetype if `transport: "none"` occurs or iroh
never establishes a QUIC path (always falls back to libp2p TCP).

---

## Sub-runbook 3 — Residential CGN

### Device setup

- Device: desktop or laptop on a home broadband connection behind a
  carrier-grade NAT (common with ISPs that have run out of IPv4 space).
- Identifying CGN: `traceroute` shows RFC 1918 hops (100.64.x.x range is
  CGNAT-specific per RFC 6598).
- `TRANSPORT_BACKEND=dual-stack` in elohim-storage.

### Test procedure (7 days)

```bash
for PLANE in blob gossip sync epr epr-atom shard view-fed identity-handshake trust; do
  curl -s "http://localhost:8090/p2p/status/$PLANE" \
    >> residential-cgn-day$(date +%d).log
done
```

### Special consideration: pkarr + iroh relay

iroh can traverse CGN via the n0 relay network or self-hosted pkarr relays.
Gate #10 (pkarr resolver) is a dependency for optimal CGN traversal.
If gate #10 is not yet closed, document that CGN results may improve after
pkarr deployment. Do not block gate #9 on gate #10 — record results as-is
and note the dependency.

---

## Decision flow per (plane, archetype)

After 7 days, for each combination of plane and device archetype:

```
iroh_success_rate = (attempts where transport == "iroh") / total_attempts

if iroh_success_rate >= 0.99:
    DECISION: iroh-canonical for this archetype on this plane
    ACTION: Update peer_transport_manifest.capability_level seed defaults (Plan 1)

elif iroh_success_rate == 0.0 and "none" never occurred:
    DECISION: libp2p-canonical for this archetype on this plane PERMANENTLY
    ACTION: Update capability_level; note in §"Decision log" below

elif "none" occurred at any point:
    DECISION: BLOCKED — neither transport reliably serves this archetype/plane
    ACTION: Escalate; do not close gate #9 until resolved
```

The decision is **permanent** for the failing (plane, archetype) pair. iroh is
not retried on that combination in the future unless a major iroh version bump
specifically addresses the failure mode.

---

## Plan 1 update after gate closure

Once all decisions are recorded, update
`elohim/elohim-storage/src/db/seeds/peer_transport_manifest_capability_defaults.rs`
(or equivalent capability_level seed table) with per-archetype defaults:

```rust
// Example — fill in actual decisions from §"Decision log"
CapabilityDefaults {
    archetype: DeviceArchetype::PhoneCellular,
    blob_plane: Transport::Iroh,       // or Transport::Libp2p if failed
    gossip_plane: Transport::Iroh,
    // ... one entry per plane
}
```

---

## Decision log (fill in after 7-day window)

| Plane | Phone/Cellular | Chromebook/School-WiFi | Residential CGN |
|---|---|---|---|
| blob | OPEN | OPEN | OPEN |
| gossip | OPEN | OPEN | OPEN |
| sync | OPEN | OPEN | OPEN |
| epr | OPEN | OPEN | OPEN |
| epr-atom | OPEN | OPEN | OPEN |
| shard | OPEN | OPEN | OPEN |
| view-fed | OPEN | OPEN | OPEN |
| identity-handshake | OPEN | OPEN | OPEN |
| trust | OPEN | OPEN | OPEN |

**Gate #9 closed:** _(date; all cells filled; Plan 1 capability_level seeds updated; signed off by)_
