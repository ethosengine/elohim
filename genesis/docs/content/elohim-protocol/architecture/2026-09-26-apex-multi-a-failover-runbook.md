---
title: Apex multi-A failover — the operator's runbook for two doorways behind elohim.host
id: apex-multi-a-failover-runbook
date: 2026-09-26
status: Operator runbook (staged; not yet applied)
author: serving-edge campaign story 2.3 (one-head-delivered sprint, task F4)
cites:
  - "serving-edge-failover-balance-stream-campaign-plan | story 2.3 this runbook discharges: two doorways behind one name, a real shed, the name keeps serving | sha256:a63d475974424ace | path: genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md"
  - "doorway-federation-failover-sprint-plan | WS3 ingress-topology precondition and the multi-A + client-retry grading the drill inherits | sha256:c66fd04c3b4f16e2 | path: genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md"
  - doorway/relay-addr-beacon/README.md
  - genesis/orchestrator/manifests/infra/alpha-coturn-shem.yaml
  - genesis/orchestrator/manifests/infra/alpha-coturn-operations.yaml
  - genesis/orchestrator/manifests/infra/alpha-relay-addr-beacons.yaml
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
  - doorway/doorway-service/.epr-meta/doorway-failover.habit.md
  - genesis/a2o/features/dataplane/doorway-apex-transition.feature
---

# Apex multi-A failover

**What it proves.** Two doorways stand behind the one public name `elohim.host`. One of them really stops serving, and the name keeps serving. The evidence is a beacon log that shows withdraw → re-admit, and an outside client loop that saw no non-200 response. Every step is an operator act. The repository stages only the commented `APEX-MULTI-A` blocks in the two coturn manifests: `genesis/orchestrator/manifests/infra/alpha-coturn-shem.yaml` (the shem leg) and `…/alpha-coturn-operations.yaml` (the ops leg).

**Posture today.** The Cloudflare zone `elohim.host` holds one apex A record, the shem WAN address. The shem coturn beacon owns it exclusively (`--record-name elohim.host`) and re-asserts it within about 30 s of any external write, logging `exclusive record DRIFTED` each time. `alpha.elohim.host` is a separate record, beacon-owned since 2026-07-28: it holds the ops WAN address and belongs to the ops leg. Both legs also contribute to `doorways.elohim.host`. The beacon sink hard-writes `ttl=60` and `proxied=false`. The edge pipeline applies both coturn manifests on every edge deploy. So each phase below must be committed to `dev` before the next edge deploy, or that deploy puts the old posture back.

**Shell setup.** Every section that calls Cloudflare starts with these lines, so each section works in a fresh shell:
```
export CF_API_TOKEN=$(kubectl get secret relay-addr-beacon-cloudflare -n elohim-alpha -o jsonpath='{.data.token}' | base64 -d)
ZID=$(curl -s -H "Authorization: Bearer $CF_API_TOKEN" "https://api.cloudflare.com/client/v4/zones?name=elohim.host" | jq -r '.result[0].id')
SHEM_WAN=$(dig +short elohim.host A @1.1.1.1 | head -1)   # before phase 1a; afterwards: dig +short turn-shem.elohim.host A
OPS_WAN=$(dig +short alpha.elohim.host A @1.1.1.1)
```

## 1. Preconditions — every one is a reading, not an intention

1. **Story 2.2 green on the household.** Run `just test mesh features/federation/name-routing.feature; echo EXIT=$?`. Expected: `N scenarios (N passed)` and `EXIT=0`.
2. **Both doorways serve the same declared head.** Expected: two identical digests, each followed by `true`.
   `for h in doorway-alpha.elohim.host elohim.host; do curl -s https://$h/api/v1/federation/coherence | jq -r '"\(.selfDigest) \(.inAgreement)"'; done`
3. **The N6 fleet leg has landed.** `grep -n 'N6 fleet leg' doorway/doorway-service/.epr-meta/doorway-failover.habit.md` must print the DELTA line, and `for h in doorway-alpha.elohim.host elohim.host; do curl -s https://$h/ | grep -o 'main-[A-Za-z0-9]*\.js'; done` must print the same file twice.
4. **GATE: each WAN reaches its own doorway.** This runbook does not make the ingress-topology decision. That decision belongs to plan A WS3 (a per-premise ingress class, or a controller split) or to sprint 5 (doorway-terminated TLS), and each carries its own rollback.
   - **Why the gate exists.** Today one Ingress claims the apex: `elohim-doorway-alpha-b` in `genesis/orchestrator/manifests/doorway/alpha-b.yaml`, with the rule `host: elohim.host` and the TLS SAN on `elohim-host-apex-tls`. Both WANs end at the same ingress-nginx controller, so both addresses reach doorway-B.
   - **The one check.** Run it from outside both premises. The doorway that answers lists its **sibling** as `peers[].doorwayId`. Doorway-A's id is `alpha-elohim-host` and doorway-B's is `apex-elohim-host`. Once story 2.2 sets `DOORWAY_MEMBERSHIP_DIR` on the fleet, the `x-elohim-served-by` header names the answering doorway directly.
     `for ip in $OPS_WAN $SHEM_WAN; do echo "$ip $(curl -s --resolve elohim.host:443:$ip https://elohim.host/api/v1/federation/coherence | jq -r '.peers[].doorwayId')"; done`
     **Pass:** `$OPS_WAN apex-elohim-host` (doorway-A answered) and `$SHEM_WAN alpha-elohim-host` (doorway-B answered). Today both lines print `alpha-elohim-host`.
   - **Until this gate passes, the drill in §4 cannot succeed and must not be run.** Flipping DNS without the gate only spreads the apex over two paths to the same doorway.
   - **Manifest check once the decision is made.** Exactly one ingress per controller class may claim `elohim.host`; with doorway TLS, none may. Run `grep -rnE '^\s+- (host: )?elohim\.host$' genesis/orchestrator/manifests/`: only the chosen owner's lines may print.
   - **Rendered-manifest check.** The edge pipeline renders a doorway manifest with `sed <placeholder substitutions> <manifest> > <manifest-minus-.yaml>-rendered.yaml` (`elohim/holochain/Jenkinsfile`, doorway deploy step), then runs the check on that file. The check reads only ingress names and `- host:` lines, which carry no placeholders, so an unsubstituted copy gives the same verdict:
     `cp genesis/orchestrator/manifests/doorway/alpha-b.yaml /tmp/alpha-b-rendered.yaml && bash genesis/orchestrator/scripts/check-ingress-conflicts.sh /tmp/alpha-b-rendered.yaml elohim-alpha` → expected `PASSED`. The script does not read `ingressClassName`, so a per-class pair fails the check until the script learns about classes.

## 2. The DNS change

Nobody hand-edits the apex records. Each beacon leg writes and deletes only the record whose comment carries its own stamp, `beacon-owner=<leg>; ts=…`. The target state is two A records at `elohim.host`, each with `ttl=60` and `proxied=false`: `$SHEM_WAN` stamped `beacon-owner=shem`, and `$OPS_WAN` stamped `beacon-owner=operations`. To check (after the shell setup above):
```
curl -s -H "Authorization: Bearer $CF_API_TOKEN" "https://api.cloudflare.com/client/v4/zones/$ZID/dns_records?type=A&name=elohim.host" | jq -r '.result[] | "\(.content) \(.comment) ttl=\(.ttl) proxied=\(.proxied)"'
dig +short elohim.host A @1.1.1.1
```
**Never let the apex reach zero records.** Resolvers cache an empty answer for the zone's SOA negative-cache TTL, which is much longer than 60 s. A beacon restarted with the serving probe starts withdrawn: it deletes its own records at once and earns them back only after 2 consecutive HTTP 200s.

## 3. The flip order, with the log lines to expect

Before any phase:
- Re-run the §1.4 check. It must print the Pass lines.
- **Standing rule (from phase 2 onward):** never change both coturn pod templates in one deploy. If both legs restart together, both withdraw at once and the apex reaches zero records.

Apply each phase with `kubectl apply -f <file>`, then commit the same edit. Read the leg's log after each phase:
`kubectl -n elohim-alpha logs deploy/coturn-shem -c beacon --since=10m | grep -E 'DRIFTED|projection applied|shared record|reaped'`. For the ops leg, use `deploy/coturn-ethosengine`.

| Phase | File and edit (its `APEX-MULTI-A` block) | Expected |
|---|---|---|
| 1a | `alpha-coturn-shem.yaml`: `--record-name turn-shem.elohim.host`, plus `--shared-record doorways.elohim.host=shem` and `--shared-record elohim.host=shem` | `cloudflare: created record record=turn-shem.elohim.host`. The apex keeps one A record, still shem's: the old record already carries `beacon-owner=shem`, so the new lane adopts it rather than duplicating it. |
| 1b | `alpha-coturn-operations.yaml`: `--shared-record doorways.elohim.host=operations` and `--shared-record elohim.host=operations` | The ops leg logs `cloudflare: shared record created`. The zone shows 2 A records. No `DRIFTED` from shem. |
| 2a | `alpha-coturn-operations.yaml`: add `--serving-probe-url http://elohim-doorway-alpha.elohim-alpha.svc.cluster.local:8080/health` | `shared membership projection applied lane=elohim.host owner=operations serving=false`, then `serving=true` about 30 s later. |
| 2b | `alpha-coturn-shem.yaml`: add `--serving-probe-url http://elohim-doorway-alpha-b.elohim-alpha.svc.cluster.local:8080/health` | The same pair of lines, with `owner=shem`. |

In each phase, `beacon-init` and the `beacon` sidecar get the same args.

**Failure mode.** If the ops record is added while shem still holds the exclusive claim, shem patches the first apex record back to its own address. It logs `exclusive record DRIFTED` every cycle, and the ops lane keeps re-creating its record. That is a write war: go back to phase 1a.

**How the probe behaves.**
- It governs **both** shared names on a leg, `doorways.elohim.host` included.
- Only HTTP 200 from `/health` on port 8080 counts as serving.
- It polls every `--serving-probe-interval-secs` (default 15 s, env `BEACON_SERVING_PROBE_INTERVAL_SECS`). The leg withdraws after 3 non-200s (about 45 s) and rejoins after 2 × 200 (about 30 s).

**Three names to move with the apex.**
- `turn:elohim.host:3478` (the conductor ICE URL) becomes a two-relay name. Both relays share the credential, so it still relays.
- Doorway-A's `FEDERATION_PEERS=https://elohim.host` (`manifests/doorway/alpha.yaml`) can now resolve to doorway-A itself. Point it at a name only doorway-B holds before relying on §1.2 again.
- `genesis/orchestrator/manifests/infra/alpha-relay-addr-beacons.yaml` re-homes these same lanes. Give its args the same edit in the same change, as its `APEX OWNERSHIP` header requires.

## 4. The shed drill

**Verify the posture is live first.**
- `dig +short elohim.host A @1.1.1.1` prints both `$SHEM_WAN` and `$OPS_WAN`.
- The last `projection applied lane=elohim.host` line in each leg's log (commands in §3) reads `serving=true`.
- If either reading fails, stop.

Run the drill from a host **outside both premises**, such as a phone hotspot or a VPS; the shem router hairpins apex 443 to its own web UI. `alpha.elohim.host` (the ops leg's own record) and `turn-shem.elohim.host` (created in phase 1a) give the per-leg addresses:
`SHEM_WAN=$(dig +short turn-shem.elohim.host A @1.1.1.1); OPS_WAN=$(dig +short alpha.elohim.host A @1.1.1.1)`.

1. Start the client loop and leave it running:
   `while true; do printf '%s %s\n' "$(date -u +%FT%TZ)" "$(curl -s -o /dev/null -m 10 -w '%{http_code} %{remote_ip}' https://elohim.host/)"; sleep 2; done | tee client-loop.txt`
2. Probe each address separately. Before the shed, both must print `200`:
   `for ip in $SHEM_WAN $OPS_WAN; do echo "$(date -u +%T) $ip $(curl -s -o /dev/null -m 10 --resolve elohim.host:443:$ip -w '%{http_code}' https://elohim.host/)"; done | tee -a resolve-probes.txt`
3. Shed doorway-B for real: `kubectl -n elohim-alpha scale deploy/elohim-doorway-alpha-b --replicas=0`.
4. Within about 45 s, the shem beacon logs `shared membership projection applied lane=elohim.host owner=shem serving=false`, and the same for `lane=doorways.elohim.host`. `dig +short elohim.host A @1.1.1.1` then prints only `$OPS_WAN`. Repeat step 2: `$SHEM_WAN` prints non-200 (the shed is real) and `$OPS_WAN` prints `200`.
5. After 5 minutes, restore: `kubectl -n elohim-alpha scale deploy/elohim-doorway-alpha-b --replicas=1`. Expect `… owner=shem serving=true` once `/health` answers 200 twice (about 30 s), and two addresses from `dig` again.
6. Stop the loop and count failures: `awk '$3!="200"' client-loop.txt | wc -l`. Expected: `0`.

**The exposure window.** Detection takes up to about 45 s, and resolvers may keep the withdrawn answer for up to 60 s more. A client that drew `$SHEM_WAN` inside that window can see the shed. If any non-200 appears, keep it in the record and check that each one carries `remote_ip = $SHEM_WAN` inside the window. That run proves withdraw → re-admit, but continuity is still owed; the cure is a client that retries across A records (plan A: "multi-A + client retry").

## 5. Rollback to single-A

Start a fresh shell and run the **Shell setup** block at the top of this runbook (it sets `CF_API_TOKEN` and `ZID`). Roll back in this order, never the reverse:

1. **Ops leg (`alpha-coturn-operations.yaml`).** Remove the probe and the `elohim.host=operations` lane, restore the legacy `doorways.elohim.host` lane, and apply. Removing a lane does **not** delete the record it wrote, so choose one:
   - a. Delete the record now:
     `curl -s -X DELETE -H "Authorization: Bearer $CF_API_TOKEN" "https://api.cloudflare.com/client/v4/zones/$ZID/dns_records/$(curl -s -H "Authorization: Bearer $CF_API_TOKEN" "https://api.cloudflare.com/client/v4/zones/$ZID/dns_records?type=A&name=elohim.host" | jq -r '.result[] | select(.comment|test("beacon-owner=operations")) | .id')" | jq -r .success` → expected `true`.
   - b. Wait up to 15 min (`--shared-stale-secs`, default 900) until shem logs `reaped stale sibling shared record`.
2. **Shem leg (`alpha-coturn-shem.yaml`).** Restore `--record-name elohim.host` and the legacy `doorways.elohim.host` lane, drop `elohim.host=shem` and the probe, and apply. Expected: `cloudflare: updated record`, or `exclusive record fresh` at debug level. `dig +short elohim.host A @1.1.1.1` prints one address.
3. **`alpha-relay-addr-beacons.yaml`.** Return it to shem's exclusive `--record-name elohim.host` and one `--shared-record doorways.elohim.host=<leg>` per leg, exactly as the header's rollback describes.
4. **The ingress decision.** It rolls back under the plan that made it (plan A WS3 or sprint 5), not here. Its check is the §1.4 one, inverted: both lines print the same `doorwayId` again.

If shem is rolled back while the ops record still exists, its exclusive lane can find that record first. It then logs `exclusive record DRIFTED` and overwrites the record with the shem address.

## 6. The proof to record

Put the evidence under `genesis/a2o/reports/recovery/serving-edge-$(date -u +%Y%m%d)/`:
- `beacon-shem.log` and `beacon-operations.log`: the grep output above, covering the drill window.
- `client-loop.txt` and `resolve-probes.txt`.
- `dns.txt`: the Cloudflare listing before, during and after the shed.

The habit `doorway-failover` gets one DELTA line naming the directory, the withdraw and re-admit timestamps, and the non-200 count. Its status flips only when **one recorded drill run** has a non-200 count of 0.
