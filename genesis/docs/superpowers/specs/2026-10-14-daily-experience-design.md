---
id: daily-experience-design
title: Daily experience — from human promises to proof
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: human:matthew
serves: release-retention
date: 2026-10-14
graduation-trigger: Each epic promise has a named owning contract and passing real-device acceptance evidence, with remaining gaps retained at their owners
cites:
  - "daily-life-epic | The human day whose promises this contract maps to owners and acceptance evidence | sha256:32e92aec21833229 | path: genesis/docs/content/elohim-protocol/epic-daily-life.md"
  - "portable-app-lifecycle-design | Owns shared app execution and residency contracts composed by the pin and unpin experience | sha256:2ca34a5e34b985fc | path: genesis/docs/superpowers/specs/2026-10-14-portable-app-lifecycle-design.md"
  - "portable-app-migration-plan | Keeps the existing runtime delivery order authoritative rather than creating another implementation plan | sha256:b32627755f2e10ec | path: genesis/docs/superpowers/plans/2026-10-14-portable-app-migration-plan.md"
  - "device-footprint-residency-carrying-capacity-design | Owns pins, archive, holds and carrying capacity behind truthful device space choices | sha256:99ab8f8330cc3f24 | path: genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md"
  - "working-version-sdk-standard-design | Owns private durable drafts, editor admission and publication used by the daily work scenes | sha256:66ba00931d6dd97e | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
  - "sync-state-contract | Owns per-copy freshness so continuity never implies an unreachable device is current | sha256:cea95b17140d2fd6 | path: genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md"
---

# Daily experience — from human promises to proof

## Purpose and standing

The epic **A day that stays with you** is the human guide for application composition: begin with
someone's day, then walk each promise into its existing specification, producer, consumer and
proof. This contract adds the missing experience acceptance between those owners. It creates no
new application framework, entity schema, transport, performance registry or parallel execution plan.

This is a reconstruction from the human's recovered instructions of 2026-10-14, not a recovered
copy of the lost assistant plan. GooglebookOS was the human's experiential reference: an integrated,
approachable daily environment. No vendor feature comparison or compatibility claim is made here.
Linux and Rust are implementation context; the person sees apps, work, devices and relationships.

Primary acceptance is the pin/offline/unpin journey under `release-retention`; the wider epic also
depends on the existing working-version, sync, privacy and runtime contracts. A green retention
habit does not establish completion of this broader experience. The portable lifecycle foundation
has contract evidence; its runtime and user journeys remain separately unproved. `idle-is-free`
and `runtime-performance` remain red. This document changes none of those standings.

## Local-first reading

- **Alone, key on device:** verified retained apps and data open; private edits survive closing and
  restarting; the person's own publication act can be prepared and signed. Other parties' acts and
  acquisition of missing bytes wait. No remote acknowledgement is required to keep a local edit.
- **Alone, key held by doorway:** retained verified material remains readable and working edits
  remain local; signing waits and the UI does not say the edit reached the account. The service
  holding the key can sign as the person and see plaintext its role processes.
- **Alone, visitor:** retained public material can be read; no durable account or cross-visit save
  is implied. Temporary work is identified as temporary before reliance on it.
- **Working version:** author's private draft by default; explicit admission widens editing.
  Publication is a separate submission through the existing authority path.
- **State:** selection stays on the device; presence passes without a durable activity log;
  working values use the existing document store; published versions use existing declared heads.
- **Arrivals:** own admitted devices add continuity and learn the state explicitly admitted to
  them; collaborators learn only the shared work; a hub adds availability and learns requests and
  plaintext it processes; a doorway adds browser access and sees its hosted traffic. Losing any
  of these preserves retained local functionality. A pool may support public purposes under its
  own contract, but this epic grants it no private attention or draft access.
- **Shown:** saved here, preparation incomplete, ready offline, other copy behind/unknown,
  connected access unavailable, publication waiting/current/not-current, actual released space.
- **Refusals:** no offline claim from code bytes alone; no deletion of personal work as unpin;
  no hub requirement for retained local work; no merged draft presented as accepted publication.
- **Stands:** existing lifecycle, working-version, sync-state and retention contracts own behavior.

Delete every doorway: the person's node still serves held apps, preserves working versions and
performs locally authorized acts. Close the laptop mid-edit and reopen without network: the last
durably accepted local edit is present, or the failure to keep it was shown before closure. The
first unavoidable external wait is for something absent locally or another party's act; it does
not block the person's own preparation.

## P2P design gate boundary

This pass adds prose and an acceptance specification only: zero new entities, DHT heads,
coordinator functions, routes or DNA changes at seed or one year. It reuses the portable lifecycle
design's classification: ephemeral lifecycle requests (C), private working state (B), existing
Content/Manifest release identities and elected heads. It does not make a device pin a new public
record. Any implementation changing those classes must run the full entity gate at that owner.

SDO/RWA reading: neither the launcher nor a funding view may become a cross-person activity
aggregator. Private app use, drafts and attention are not community payment records. A host can
observe what it serves; the UI must not imply host blindness. Supporting someone grants only the
specific admitted assistance, not blanket access to their account. Guardian and recovery mechanisms
remain work at their identity/governance owners; the scene is not evidence of a shipped mechanism.

## Promises and their authoritative homes

Paths below are relative to the repository. Existing specs own schemas and mechanics; this table
owns the connection to the human promise. The portable migration plan remains the execution order.

| Epic promise | Existing contract / producer | Acceptance and present boundary |
|---|---|---|
| Reopen the morning's unfinished work offline | Working-version SDK standard; SDK persistence and authorized storage | `genesis/a2o/features/federation/local-first-shared-sheet.feature`; prove crash/reopen persistence without implicitly sharing or publishing |
| Try through a hub, then keep locally | Portable app lifecycle; SDK composition, hub serving, storage acquisition, resident runtime | `genesis/a2o/features/devflow/portable-app-lifecycle.feature`; existing browser-to-local continuation is `@wip`; foundation contracts alone are insufficient |
| Know what is ready away from a connection | Device-footprint design and portable lifecycle | Cold reopen with complete code AND selected data; missing data or incomplete acquisition keeps the readiness claim off |
| Continue across devices without losing work | Working-version and sync-state contracts; SDK/store reconciliation | Per-copy caught-up/behind/unknown and preserved local edits; retained position does not prove a remote device is online |
| Invite help without publishing private thought | Working-version standard; signed editor admission and separate publication | Shared-sheet journey must refuse unauthorized discovery and distinguish merged work from accepted publication |
| Remove local installation, retain records, open through hub | Device-footprint and portable lifecycle; storage holds + runtime adapter + client | Missing explicit human unpin journey below; existing automatic reclamation scenario covers only part of it |
| Keep the device quiet and foreground work responsive | Device-footprint carrying capacity; node/Ark admission and storage scheduling | Existing `idle-is-free` and `runtime-performance` habits; no measured UX budget acceptance supplied by this authoring pass |
| Understand who carries the service and its cost | Resource-governance owners and existing REA/community contracts | Human-visible support/terms view and capacity-change journey still need owner-specific design; no billing schema or free-service promise is introduced here |
| Receive help without losing voice | Identity/governance owners; existing capability and consent designs | Supported-decision, accessibility and revocation journeys remain explicit design gaps; they cannot be inferred from hosted login working |

## First missing acceptance journey: pin, leave, return, release

### Continuous work is the default

Everything autosaves from the first edit, including while acquisition is incomplete. **Host on
this device** starts background pinning of the app and the working data needed for the current
task. The person does not enter an installation wizard, choose a save destination, or stop working.
Scope and footprint remain inspectable; additional offline collections can be chosen when needed,
but a data-selection ceremony is not a prerequisite for continuing the current task.

The pin/progress indicator answers whether this app and its working data are retained and ready
offline. Connectivity and live collaborator presence answer separate questions. A completed pin
does not imply somebody else is online; a live connection does not imply offline readiness.
Authorized shared editing continues during pinning. Completion preserves the same document,
selection and in-flight edits without relaunch, manual transfer or duplicate effects. Local edits
then require no host round trip; this removes network waiting from that interaction, not all
compute latency. Collaboration delivery still depends on connectivity.

On disconnect after pinning, the same app continues executing locally and autosaving edits.
Presence stops claiming the person is live; collaboration resumes and authorized changes reconcile
automatically on reconnect. Autosave is distinct from sharing, publication and remote acknowledgement.
Storage failure is surfaced truthfully, never reported as saved. Missing bytes before pin completion
remain an honest offline limitation without blocking connected work during preparation.

The story is declared in `genesis/a2o/features/devflow/daily-app-residency.feature` under
`@concern:release-retention`, alongside the existing portable-app lifecycle feature. It stays
`@wip` until real UI/runtime steps exist. Required fixtures: two admitted devices,
one authorized serving hub, a compatible app, selected retained records, a private unsynced edit,
and shared executable bytes also needed by a second app. Ordinary browser access must preserve
the connected fallback. Cold offline execution requires a supported local runtime/connector or
a browser implementation with verified persistent app and data storage; a plain tab alone does
not imply native execution. Qualify each supported mode separately.

1. **Try:** open the authorized hub-served app without a local installation. No local offline
   claim appears. A visitor or unauthorized person cannot read private records.
2. **Host:** choose Host on this device while editing. Show pin/sync progress and inspectable
   footprint; keep editing, autosaving and collaborating during acquisition. Incomplete or failed
   acquisition stays visibly incomplete and preserves the connected path.
3. **Leave:** complete the pin without interrupting the current work, then disconnect and keep
   editing in the same app. Verify local execution and autosave without a hub round trip. Terminate
   and cold-reopen; assert retained records and edits made before, during and after pin completion.
4. **Continue:** reconnect and automatically synchronize permitted work to the second device/hub.
   Show per-copy progress; a failed sync must not erase the locally newer working version.
   Then create a second private edit that remains only on the laptop, so Release and Lose the hub
   actually prove preservation and readback of unsynced work.
5. **Release:** with the app inactive, choose Remove from this device. Release only executable
   bytes with no remaining hold; preserve keys, private/unsynced work and bytes another app needs.
   A running instance or other hold produces a truthful deferred/partial result. Measure actual
   released storage; do not promise that displayed app size equals reclaimable shared bytes.
6. **Return:** open through the available authorized hub without reinstalling locally. Read the
   records that actually reached that host. Local-only work remains local and is not advertised
   as remotely available. Offline readiness is gone until explicitly restored and verified.
7. **Lose the hub:** repeat Return while the host is unreachable. Retained personal work remains
   accessible through an appropriate retained reader/export path; the removed app is not claimed
   runnable offline. Show the unavailable connected path without deleting or rewriting records.

Missing node: `portable-app lifecycle / between local reclamation → continued hub use / explicit
unpin preserves personal state and removes offline readiness while authorized connected access
survives / probe: cold restart + storage/hold assertions + hub reopen / state: specified, unproved`.

The retained reader/export requirement is a dependency to resolve before presenting removal as
safe access to local-only work. Retention alone proves preservation, not that the person can read
those bytes after the only renderer has been removed.

UI wording is proposed, not a new API vocabulary. The existing contract distinguishes `pin(false)`
(remove retention protection) from `archive()` (release residency). **Remove from this device**
must compose those operations with their holds and failure semantics, or refuse before promising
space. Unpinning alone must not be reported as completed archival. Existing internal states such
as Kept offline, In use, Archived and Opened from my hub remain authoritative at their owner.

## Gradient and performance acceptance

Read one path at a time. Own-device continuation is `trustful-self`: carry existing admission
proof rather than asking permission per item, while keeping revocation checks. Acquisition from
an unfamiliar holder still verifies content. Background replication must not occupy the foreground
interaction lane. Per-release verification and bounded retained sets should replace repeated
per-frame checks and history-wide scans where the owning contract allows it. Mechanical leaks,
queues and stalls are owed fixes regardless of trust. These are design questions, not measured wins.

Reach applies separately to public app bytes and the private reference revealing a person's use.
Custody is bounded by declared offline scope and commitments, not every app on every peer.
Freshness is per copy and per task: reading a retained lesson differs from completing an agreement.
Hosts can observe access timing; no anonymity claim follows from content addressing. Local users,
households and communities carry identifiable resource costs under explicit arrangements.

For the first runtime qualification, select a representative device and record binary, workload,
data size, power mode, network and cohort. Capture bounded cold-open, warm-resume, edit durability,
idle, sync contention and unpin windows with the existing runtime-performance CLI. Record wall
time, CPU, memory, storage and attribution coverage. Choose thresholds before a comparative run;
none are fabricated here. Run its report/check and compare only matched cohorts. Missing evidence
is incomplete acceptance. Quiet idle cannot stand in for foreground responsiveness or safe recovery.

## Delivery and review

Follow the portable migration plan's existing dependency order. The next implementation brief
must bind the journey above to producer gates and browser/device evidence; do not build a mock
launcher that reports success without the retention/runtime producer. Domain examples (learning,
orchard work, collaboration) compose the common contract rather than duplicating persistence.

This authoring pass is complete when the epic stands alone under a blind-reader review, citations
resolve, and the decomposition identifies real owners and missing proofs. It does not deliver a
desktop shell, change runtime behavior, set new performance limits or turn any pending story green.
