---
title: Doorway chaos operations
id: doorway-chaos-operations-runbook
tier: architecture
status: active
created: 2026-09-14
pillar coupling: doorway (web2 projection), elohim (notarized authority)
---

# Doorway chaos operations

This drill answers three separate questions about one governed site:

1. Does an already reachable sibling entrance keep serving during another premise's outage?
2. Does the same public hostname select a surviving entrance after the failed leg is withdrawn?
3. Does every usable entrance honor the current notarized authority after that authority changes during the split and after the failed entrance recovers?

A successful `elohim.host` request while Ethosengine is off answers only the first question unless the run also proves withdrawal and selection. A cached shell or `/health` response cannot answer the third question.

## Household operation

The maintained local operation is the existing a2o story and process controller. Start and stage the household normally, then run only its feature:

```bash
just mesh start
just mesh prologue
just test mesh features/dataplane/doorway-apex-transition.feature
```

The scenario `A current governed version crosses withdrawal and recovery` authors a fresh fixture EPR through the existing fixture identity and registers required cleanup. It records exact authority A, pauses one household-owned doorway, waits for its owner record to leave shared membership, then the fixture author publishes B through the serving alpha-A gateway. It checks B from the author operation as four distinct values: the returned notarized action, its blob address, the `commit` in `/apps/{blob}/version.json`, and the generated entry-script name. Each serving assertion checks the HTTP page and assets and boots that page in a real browser. It resumes the paused doorway in the scenario and in the unconditional `After` hook; cleanup verifies the guarded process identity is running and its health path answers before releasing the lease. After readmission, both entrances must serve and boot B. Run evidence is written by the existing mesh report path under `genesis/a2o/reports/`.

Do not run this scenario against a public fleet. Its process control is valid only for the household fixture this workstation owns.

## Public WAN observer

The WAN observer is read-only. An operator performs the withdrawal and recovery using the infrastructure's owned controls. The observer never runs `kubectl`, changes DNS, edits ingress, resets identity, or authors content. Run it from a machine outside the failure domain so a powered-off premise cannot also silence the witness.

The canonical fixture author operation emits a `doorway-chaos-author-receipt/v1` attachment before any serving-path observation. Save that attachment as a JSON file: it identifies the fixture author and operation, the authored timestamp and content id, and the exact action/blob/version/entry-script tuple. The observer accepts only `--author-receipt`; it records the receipt's absolute path and SHA-256 digest and derives every expected value from its body. A `/head` or `/db/content` response copied from the doorway is not this receipt and is refused. Keep one observer report path for all three invocations:

```bash
cd genesis/a2o

node --import tsx scripts/doorway-wan-chaos-observer.ts \
  --phase baseline \
  --public-name elohim.host \
  --content-id governed-site-fixture \
  --author-receipt reports/fixture-author-A.json \
  --boundary-at 2026-09-14T20:00:00Z \
  --window-ms 60000 --cadence-ms 5000 --max-gap-ms 7000 \
  --leg ethosengine=https://ETHOSENGINE_IP \
  --leg shem=https://SHEM_IP \
  --report reports/doorway-wan-chaos-RUN_ID.json
```

After the operator marks the fault onset, waits for three failed serving probes, and confirms the failed owner's withdrawal, run the fault phase with B and name the withdrawn leg:

```bash
node --import tsx scripts/doorway-wan-chaos-observer.ts \
  --phase fault \
  --public-name elohim.host \
  --content-id governed-site-fixture \
  --author-receipt reports/fixture-author-B.json \
  --fault-onset 2026-09-14T20:04:00Z \
  --boundary-at 2026-09-14T20:05:00Z \
  --max-event-observation-delay-ms 90000 \
  --window-ms 60000 --cadence-ms 5000 --max-gap-ms 7000 \
  --withdrawn-leg ethosengine \
  --leg ethosengine=https://ETHOSENGINE_IP \
  --leg shem=https://SHEM_IP \
  --report reports/doorway-wan-chaos-RUN_ID.json
```

After the operator restores the premise and observes two successful serving probes and readmission, run the recovery phase with the same B tuple:

```bash
node --import tsx scripts/doorway-wan-chaos-observer.ts \
  --phase recovery \
  --public-name elohim.host \
  --content-id governed-site-fixture \
  --author-receipt reports/fixture-author-B.json \
  --recovery-onset 2026-09-14T20:09:00Z \
  --boundary-at 2026-09-14T20:10:00Z \
  --max-event-observation-delay-ms 90000 \
  --window-ms 60000 --cadence-ms 5000 --max-gap-ms 7000 \
  --leg ethosengine=https://ETHOSENGINE_IP \
  --leg shem=https://SHEM_IP \
  --report reports/doorway-wan-chaos-RUN_ID.json
```

Each phase takes at least three samples across a bounded window. Set `--boundary-at` to the observation boundary immediately before invoking that phase. The fault phase also requires the actual operator `--fault-onset`; recovery requires `--recovery-onset` and inherits the immutable fault onset from the stored fault phase. `--max-event-observation-delay-ms` is the declared convergence allowance from each operator event to the completion of its first fully successful required sample. The observer enforces completed baseline < fault onset < B authorship <= first fault sample completion < completed fault observation < recovery onset <= first recovery sample completion. Sample starts are scheduled from the first observation instead of sleeping after a request, and the observer receipt records all four event/measurement timestamps, boundary-to-first-sample delay, event-to-first-sample start and completion delays, observed window, and largest start-to-start gap. The phase refuses continuity when observation began before the declared observation boundary, began more than `--max-gap-ms` after it, covered less than `--window-ms`, exceeded the maximum sample gap, or completed its first required sample after the event-to-observation allowance. Timing bounds must be finite. Each request carries no-cache headers and a unique query parameter.

The ordinary request uses public DNS with the apex hostname for DNS, TLS SNI, certificate validation, and HTTP Host. Each leg request keeps that same apex SNI and Host while pinning the connection to the declared address. The public root must name the externally expected entry script and its root `version.json` must carry the expected version; the addressed blob's `version.json`, content projection, and head must agree with the same externally supplied tuple. This proves virtual-host routing and current browser entry binding on a named leg separately from public DNS selection. DNS answers are recorded per sample; they are evidence about membership and are not required when the ordinary client proves fallback from a failed advertised address.

The receipt keeps three attribution levels apart:

- `declaredAddress` is operator input.
- `kernelVerifiedRemoteAddress` is the connected socket peer.
- `remotelyReportedDoorway` is an HTTP header claim and never proves backend identity by itself.

`PERMITTED` means the required observations produced a verdict. `REFUSED` means an observed value contradicted the contract. `NOT MEASURED` means a marker, phase, prerequisite, or observation was absent. Fault and recovery boundaries must be later than their preceding phase. Authority A and B must have distinct receipt digests; fault and recovery must use the same B receipt. A DNS change alone does not prove withdrawal. The DNS-membership-withdrawal claim remains unmeasured unless separately proven by the operator's membership authority. Automatic route selection is permitted only when every baseline ordinary sample connected to the leg later withdrawn, every fault sample found that pinned leg non-serving, a distinct pinned survivor's declared address matched the connected socket peer and served B, and the ordinary apex request connected to that survivor and served B throughout the measured fault window.

Three separately invoked windows do not observe the time between them. The receipt therefore keeps `continuousAcrossUnobservedPhaseGaps` at `NOT MEASURED`; the current-authority claim covers the bounded baseline, split, and recovery windows only. The owned household scenario proves the controlled transition and exact authority at its stations, but it also does not claim continuous request sampling across every instant of the fault. A future continuous WAN watch arm is the missing station for whole-outage request continuity.

The JSON file is durable history evidence. It is not protocol truth and confers no authority.
