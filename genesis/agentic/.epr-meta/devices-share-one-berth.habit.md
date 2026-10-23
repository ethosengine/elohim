---
epr-habit-version: 1
id: devices-share-one-berth
invariant: >
  A steward-scoped berth claim made on one of the steward's devices is effective only after every
  other admitted device's current head is folded. Otherwise it is refused naming the holder, or
  `unconfirmed` naming the silent device. A push to `dev` from one device is refused while another
  device holds `dev-integration`.
status: red
active: false
checks:
  - "a2o @concern:devices-share-one-berth (genesis/a2o/features/devflow/devices-share-one-berth.feature, two devices in one container: a held claim refused naming the holder; a push stopped while the other device holds dev-integration, alpha-fleet or jenkins-dispatch; a silent device read as unconfirmed; an override that binds the returning device; concurrent claims settled once; a retired device's lease dropped without restart; @act:host; cd genesis/a2o && pnpm exec cucumber-js --tags '@concern:devices-share-one-berth and not @requires:shem and not @wip'. Every scenario is @wip until T7 lands the steps, so this selects zero scenarios today)"
  - "a2o @concern:devices-share-one-berth and @requires:shem (the same feature's cross-workspace scenario: the held-claim refusal between the ethosengine and shem workspaces over their own link, no coordination record carried by git, a shared server or the operator; also @wip until T7, and held without shem)"
refs:
  - "spec: genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md §3.4 (steward-scoped resources, effectiveness, tie-break, refusal wording), §3.5 (pre-push claims dev-integration), §6 (this habit)"
  - "plan: genesis/docs/superpowers/plans/2026-10-23-workspaces-cooperate-as-devices-plan.md (T0 story and habit; T6 lease view; T7 pre-push and status; T8 the cross-workspace receipt that flips this green)"
  - "sibling: .epr-meta/ledgers-fold-once.habit.md (its first move, berth emitting every record kind as a FlowEvent, is what the channel carries); genesis/agentic/.epr-meta/measure-runs-on-a-peer.habit.md (container-scoped berth, unchanged here)"
retire-when: >
  when the lease view and the fold are served by the native peer runtime (lvi or storage) with no
  berth-specific code.
---
DELTA 2026-10-23 (BORN red): no mechanism exists yet. berth has no `scope: steward` resources, no device announces a channel head, no `epr flow fold` verb or `epr peer` daemon exists, and the pre-push hook claims nothing; the feature `genesis/a2o/features/devflow/devices-share-one-berth.feature` is written with every scenario `@wip` and no step definitions (T7 owns them). Status flips green only on the T8 cross-workspace receipt, never on the in-container scenarios alone.
DELTA 2026-10-23 (T5, still red): each device now projects its crossing berth records into a channel tree, gossips a signed head carrying its acknowledgement vector, and folds the other devices' records via that device (`epr peer serve | status`, `epr flow fold`); the in-container smoke `genesis/local-dev/workspaces-cooperate/smoke-t5-two-daemons.sh` showed each device listing the other's head in about 1.3 s and a `berth claim` on b folded into a's flow store in about 1.1 s. No steward-scoped claim is judged yet (T6), so nothing this habit promises is kept.
