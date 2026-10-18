---
epr-habit-version: 1
id: graduates-onto-own-key
invariant: >
  A hosted human who runs a node of their own can move their doorway account onto the key that
  node holds, from their own account page, and nobody else can do it for them: the doorway issues a
  single-use challenge only to the account holder who re-enters their password, the node signs it
  only for the human signed in to the node, and the doorway re-points the account only on a valid
  signature from the key it names. After the move the doorway hosts no cell for the old key, and
  hosts none for the new one: the account is node-phase and the doorway routes it, never re-hosts it.
  No operator surface, k8s action or database edit moves a person's key.
status: red
active: false
checks:
  - "a2o @concern:graduates-onto-own-key (genesis/a2o/features/auth/graduation/01-onto-my-own-node.feature; authority is the household lane: `just test mesh features/auth/graduation/01-onto-my-own-node.feature` — the finish line needs a late joiner, `just mesh join-peer`)"
  - "doorway-service unit tests node_key_bind::* (challenge build/parse, verdict refusals) via `just gate doorway-service`"
first_move: >
  Land the doorway challenge/bind routes, the node's sign route behind a key-bound session, and the
  account-page ceremony; write the step definitions so the stations run and this habit measures red
  on a real household run, then drive the finish line to green.
refs:
  - "genesis/docs/superpowers/specs/2026-10-16-graduate-onto-own-node-key-design.md (the design and P2P gate output)"
  - "genesis/a2o/features/auth/hosted-human/05-leaving.feature (the hosted series this one follows; 'graduation is a later story')"
  - "genesis/data/timeline/backlog/keyrotation-mint-path-witness-backed.md (the witnessed rotation this habit does not yet claim)"
retire-when: >
  when a doorway account points at the human's identity chain-root rather than a key, and a key
  change is a witnessed rotate_identity_key the doorway follows by construction — then there is no
  doorway-side key to move, and the practice under watch has become a property of the substrate.
---
DELTA 2026-10-16 (born UNWIRED): declared with the story and the design; no step definitions yet.
The 2026-10-16 hand edit of one account's Mongo row (hCAkmO-… → hCAkJH75…) is the event this habit
exists to make unnecessary.
DELTA 2026-10-16 (unwired -> RED, measured): household lane `just test mesh '@graduation and not @wip and not @browser'`
(sprint-report-household-20261016T214355Z-e638af9f): 6 scenarios, 4 passed, 2 failed. These ran against the live doorway A
with real ed25519 signatures: the password gate, a foreign-key signature refused, a challenge moves once, and no operator
move. The two node scenarios are red UNBOUND because no join-peer fixture tears its node down. The browser finish line
and the copied-session/arbitrary-text stations also wait on a key-bound node-session helper. The first live probe found
both node-key routes 404 behind AUTH_OWNED_PATHS while 1796 unit tests were green. They were fixed, and a test now
derives the owned list from the handler's arms.
