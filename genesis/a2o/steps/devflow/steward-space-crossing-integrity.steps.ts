/**
 * Red-first bindings for the steward-space crossing-integrity stations.
 *
 * These bindings enumerate the provenance fixture vocabulary for signed heads,
 * served trees, crossing receipts, brit notes, and mixed-version peers. Each
 * handler returns the Cucumber sentinel instead of fixture evidence, so
 * placeholder data cannot satisfy a station.
 */

import { Given, When, Then } from '@cucumber/cucumber';

const pending = (): 'pending' => 'pending';

Given(
  'two devices of {string}, {string} and {string}, each with a signing key and steward-space peer',
  pending
);

Given("matthew's roster admits devices A and B", pending);

Given("matthew's roster binds each admitted device to its workspace brit signer", pending);

Given(
  'fixture labels {string} and {string} have serialized device identifiers {string} and {string}',
  pending
);

Given('device A enforces crossing provenance for every crossed record', pending);

Given(
  'device B signs a head whose committed tree contains a green gate verdict for closure {string}',
  pending
);

Given(
  'that verdict names a brit attestation for the same green result and exact closure {string}',
  pending
);

Given("the attestation is signed by device B's workspace brit key", pending);

Given(
  'no red verdict for {string} remains that a later verified green has not declared rechecked and replaced',
  pending
);

When(
  "device A verifies device B's head signature, committed root, and verdict membership",
  pending
);

When('device A folds the verdict from device B', pending);

When('device A evaluates the named brit attestation for reuse', pending);

Then('the verdict resolves as proven from admitted {string}', pending);

Then(
  "device A verifies the named brit attestation with device B's workspace brit key",
  pending
);

Then(
  'device A may reuse the verdict for {string} instead of repeating the gate',
  pending
);

Given('device B has an active steward lease naming resource {string}', pending);

Given(
  "device B signs a head whose committed tree contains that lease and an inbox note",
  pending
);

When(
  "device A verifies device B's head signature, committed root, and both records' membership",
  pending
);

When('device A folds those records from admitted {string}', pending);

Then(
  "device B's crossed lease holds {string} on device A",
  pending
);

Then("device A's inbox identifies {string} as the note's proven crossing peer", pending);

Given('that verdict names a brit attestation with an invalid signature', pending);

Then("the verdict's crossing provenance resolves as proven", pending);

Then('device A refuses the named brit attestation', pending);

Then('device A repeats the gate for {string}', pending);

Given(
  "that verdict names a brit attestation signed by device B's bound workspace brit key",
  pending
);

Given(
  'the attestation matches the signer and green result but names different input closure {string}',
  pending
);

Then('device A refuses the mismatched brit attestation', pending);

Given(
  'device C has a known signing key and a green gate verdict for closure {string} under a tree committed by its verified signed head',
  pending
);

Given("device C is not admitted by matthew's present roster", pending);

When("device A verifies device C's signed tree and asks for a reusable verdict for {string}", pending);

Then("device C's verdict is marked revoked on device A", pending);

Then('device A receives no reusable sibling verdict for {string}', pending);

Given(
  "device A's local flow store contains a lease claim forged with {string}",
  pending
);

Given('no proven crossing from device B contains that claim', pending);

When('device A rebuilds its steward lease view', pending);

Then('the forged claim remains visible as unproven', pending);

Then('the forged claim holds no steward-scoped resource', pending);

Given(
  "device A's local flow store contains a green gate verdict forged with {string}",
  pending
);

Given('no proven crossing from device B contains that verdict', pending);

When("device A lists peer gate verdicts for the forged verdict's closure", pending);

Then('the forged verdict remains visible as unproven', pending);

Then('the peer verdict list contains no reusable green verdict for that closure', pending);

Given(
  "device A's local flow store contains an inbox note forged with {string}",
  pending
);

Given('no proven crossing from device B contains that note', pending);

When('device A reads its steward-space inbox', pending);

Then('the note remains visible as unproven', pending);

Then("the inbox does not identify device B as the note's proven crossing peer", pending);

Given('device A is still missing a quarantined record from device B', pending);

Given(
  "a local record forged with {string} claims device B read A's missing-record notice",
  pending
);

Given('no proven crossing from device B contains that acknowledgement', pending);

When('device A folds the next signed head from device B', pending);

Then('the forged acknowledgement remains visible as unproven', pending);

Then('the missing record remains outstanding despite the forged acknowledgement', pending);

Then('the synchronization attempt remains incomplete', pending);

When('device B publishes the quarantined record under a later signed head', pending);

When('device A folds that later head', pending);

Then('device A fetches the quarantined record', pending);

Then("device A verifies its membership under the later head's committed root", pending);

Then('the record resolves as proven from device B', pending);

Then('the record is no longer outstanding', pending);

Then('the synchronization attempt completes', pending);

Given(
  'device A has recorded signed head {string} with committed root {string} for device B',
  pending
);

Given(
  'device B signs later head {string} with committed root {string}',
  pending
);

Given('device B serves a self-consistent tree rooted at {string}', pending);

When('device A folds the signed head from device B', pending);

Then('the fold fails crossing-integrity verification', pending);

Then(
  'device A keeps {string} as device B\'s folded head and {string} as its committed root',
  pending
);

Then('no record from {string} gains proven crossing provenance', pending);

Given(
  'the local brit note index contains a green gate note for closure {string}',
  pending
);

Given("that note is not signed by device A's workspace brit key", pending);

When('device A asks brit for reusable local evidence for {string}', pending);

Then(
  "brit refuses that note because its signature is not from device A's bound workspace brit key",
  pending
);

Then('device A runs the gate instead of reusing that note', pending);

Given('device A has a proven green sibling verdict for closure {string}', pending);

Given(
  'device A has an unproven crossed red verdict for the same closure',
  pending
);

Given(
  'no later verified green declares that it rechecked and replaced that red verdict',
  pending
);

When('device A asks for reusable gate evidence for {string}', pending);

Then('the red verdict vetoes the proven green verdict', pending);

Then('device A runs the gate locally', pending);

Given(
  "device A's brit index has a red note for the same closure with an invalid signature",
  pending
);

Given(
  'no later verified green declares that it rechecked and replaced that red note',
  pending
);

Then('brit does not accept the red note as positive reusable evidence', pending);

Then(
  'the red note still vetoes the proven green verdict because forcing a local gate grants nothing',
  pending
);

Given(
  'device A has an applicable red verdict with content address {string} for closure {string}',
  pending
);

When(
  'device B publishes a later candidate green for the same closure that names {string} as the verdict it rechecked and replaces',
  pending
);

When(
  'device A verifies its signed-head provenance and matching brit attestation',
  pending
);

Then('the replacement green verdict is proven', pending);

Then('the earlier red no longer vetoes closure {string}', pending);

Then('device A may reuse the replacement green verdict', pending);

Given(
  'device A holds an unproven gate verdict from device B',
  pending
);

Given("device B's retained events tree under its signed head contains the original verdict", pending);

Given("device A's held set already contains that record", pending);

When(
  "device A's daemon processes device B's retained signed head through the ordinary planned fold with the populated held set",
  pending
);

When(
  'the planned fold verifies the retained head signature and its committed events-tree root',
  pending
);

When(
  "the planned fold verifies the held verdict's membership at the same original content address",
  pending
);

Then(
  'the planned fold reconstructs a crossing receipt from the retained head and tree membership',
  pending
);

Then("the record resolves as proven under device B's carrying signed head", pending);

Then('device A may evaluate that verdict for gate reuse', pending);

Given(
  /^device A's local flow store contains a forged (lease claim|inbox note|green gate verdict) with "([^"]+)"$/,
  pending
);

Given(
  "device A's own key validly signs the head anchor",
  pending
);

Given("the anchor's committed tree contains the record and matches its receipt", pending);

When("device A resolves the record's crossing provenance", pending);

Then(
  'the record resolves as unproven because its claimed origin is the local device',
  pending
);

Then(
  /^the record grants no (crossed lease effect|crossing-peer identity|reusable green gate result)$/,
  pending
);

Given(
  "Matthew's roster admits real workspaces {string} and {string}",
  pending
);

Given('both device homes declare enforce posture', pending);

Given(
  'workspace {string} runs an older peer binary that treats unproven crossed records as authoritative',
  pending
);

Given(
  'the pair requests admission for shared lease coordination on {string}',
  pending
);

When(
  'each workspace probes its running binary and emits a receipt with the observed release and crossing-enforcement capability',
  pending
);

When(
  'the compatibility check compares the two receipts before the berth admits shared coordination',
  pending
);

Then('the compatibility station exits nonzero for workspaces {string} and {string}', pending);

Then('the station reports that workspace {string} cannot enforce crossing provenance', pending);

Then(
  'the berth grants no cross-workspace lease on {string} until both peers enforce the declared posture',
  pending
);
