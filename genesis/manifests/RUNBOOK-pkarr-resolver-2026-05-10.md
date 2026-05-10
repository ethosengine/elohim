# Runbook — Self-hostable pkarr resolver enablement (2026-05-10)

**Target:** any cluster running a doorway deployment.
**Namespace:** wherever doorway is deployed (run `kubectl get deployment -A -l app=doorway` to find).
**Risk:** Low. Adds a new HTTP route at `/pkarr/...` and opens the doorway pod to act as a pkarr relay. No existing routes change. The resolver is OFF by default and is opt-in via env var.
**Cutover gate:** #10 — "pkarr resolver running on doorway.elohim.host for one week with zero unavailability beyond the doorway itself's uptime" (genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md, line 421).

## What this enables

Two new HTTP endpoints on the doorway:

- `GET  https://<doorway>/pkarr/<z32-public-key>` — return the cached pkarr SignedPacket for the key, or 404.
- `PUT  https://<doorway>/pkarr/<z32-public-key>` — accept a self-signed pkarr SignedPacket. Body is the relay payload bytes (timestamp + signature + DNS payload, max 1104 bytes per pkarr spec).

These let any iroh peer (and any other pkarr client) use this doorway as a discovery resolver instead of n0's hosted dns.iroh.link.

## Apply

```bash
# Identify the deployment (output to be quoted here on first apply):
kubectl get deployment -A -l app=doorway
# Expected: <observed-on-first-apply>

# Patch in the new env vars + cache PVC:
kubectl apply -f genesis/manifests/doorway-pkarr-resolver.yaml -n <ns>
```

Expected output (verbatim, observed on first apply — fill in after first run):
```
<observed-on-first-apply>
```

## Verify the endpoint is serving

```bash
# 1. Sanity GET on a known-not-cached key returns 404 (not 5xx):
curl -sw '\nHTTP %{http_code}\n' https://<doorway>/pkarr/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
# Expected: HTTP 404 with body "no packet cached for key"

# 2. PUT a real signed packet using the iroh discovery binary (or any pkarr-cli):
iroh net node-id  # capture the node id
# then trigger a publish by starting any iroh-blobs server pointed at this resolver

# 3. Re-GET that key — should now 200:
curl -sw '\nHTTP %{http_code}\n' https://<doorway>/pkarr/<your-node-id-z32>
# Expected: HTTP 200, body bytes are the SignedPacket relay payload.
```

Per memory `feedback_head_vs_get_blob_asymmetry`: do NOT use HEAD on /pkarr — pkarr endpoints are GET-only; HEAD will 405.

## Monitor uptime (the gate's actual measurement)

The gate is one week of zero unavailability beyond the doorway's own uptime. Track via:

```bash
# External uptime probe (run from a cluster *outside* the doorway's own cluster):
while true; do
  code=$(curl -s -o /dev/null -w '%{http_code}' https://<doorway>/pkarr/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)
  [ "$code" = "404" ] || echo "$(date -u +%FT%TZ) anomaly: HTTP $code"
  sleep 60
done
```

A 404 for a not-cached key is the healthy signal (it proves the route is matched and the handler is running). Anything other than 404 (5xx, timeouts, 503) for the bare-route probe is a gate-violating event. Aggregate daily; gate #10 closes when a 7-day window shows zero non-404 events.

## Federation manifest declaration

Once the resolver is healthy, declare it in the doorway's federation manifest so other peers can discover it:

```bash
# Update the doorway's published federation entry to include itself in
# discovery_resolvers. The schema is at:
#   elohim/sdk/schemas/v1/manifests/discovery-resolvers.schema.json
# For the demo cluster, this is a config update on the doorway-service
# (federation publish path); for production it goes through the steward's
# admin UI once the panel exists.
```

## Rollback

```bash
# Revert the env vars (the resolver disables itself when DOORWAY_PKARR_RESOLVER_ENABLED=false):
kubectl set env deployment/doorway DOORWAY_PKARR_RESOLVER_ENABLED=false -n <ns>
kubectl rollout status deployment/doorway -n <ns>
```

After rollback, GET /pkarr/* returns 404 with body "pkarr resolver not enabled on this doorway". No data loss; the cache is held in the PVC and will reload if re-enabled.
