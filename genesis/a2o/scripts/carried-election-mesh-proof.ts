/**
 * Carried-election mesh proof (2026-08-31) — the local-mesh evidence leg for
 * carry-the-election (feat 41716774b, habit dataplane-convergence).
 *
 * Proves, on the 3-peer household mesh:
 *   1. the new coordinator externs are live on the installed DNA;
 *   2. two peers can hold divergent DECLARED heads for one id (the fleet's
 *      frozen class), with an EARNED canonical on one;
 *   3. THE CRUX — evidence from the declaring peer's conductor
 *      (get_canonical_election_evidence) verifies on the OTHER peer's
 *      conductor (verify_carried_election) and yields the earned election;
 *   4. tampered evidence is REFUSED (anti-regression);
 *   5. the disagreeing peer's storage sweep converges its served head, and the
 *      obeyed metric names which path carried it.
 *
 * Run: cd genesis/a2o && pnpm exec tsx scripts/carried-election-mesh-proof.ts
 *
 * 2026-09-05: the staging helpers this script authored (conductor rail, author+
 * declare, served-head read, the tamper) moved to
 * `src/framework/dataplane/carried-election.ts` so
 * `steps/dataplane/federation-deploy.steps.ts` stages the SAME divergence
 * rather than a second, drifting copy. The sequence below is unchanged.
 */
import { encodeHashToBase64 } from '@holochain/client';

import {
  authorDeclare,
  canonicalElectionEvidence,
  connectConductor,
  meshConductorPorts,
  meshStorageUrl,
  servedHead,
  tamperLinkRecord,
  verifyCarriedElection,
} from '../src/framework/dataplane/carried-election.js';

const ID = `carried-election-proof-${Date.now()}`;

async function main() {
  const M = meshStorageUrl(0);
  const J = meshStorageUrl(1);
  const matthewPorts = meshConductorPorts(0);
  const jessicaPorts = meshConductorPorts(1);
  const matthew = await connectConductor(matthewPorts.adminPort, matthewPorts.appPort);
  const jessica = await connectConductor(jessicaPorts.adminPort, jessicaPorts.appPort);
  console.log('matthew agent:', matthew.agent);
  console.log('jessica agent:', jessica.agent);

  // (1) Extern delivery check — unknown-function here means a stale DNA.
  const probe = await matthew.call('get_canonical_election_evidence', 'no-such-id');
  console.log('extern delivery: get_canonical_election_evidence(no-such-id) =', probe);

  // (2) Divergent declared heads, one id, two roots.
  const headM = await authorDeclare({
    storageUrl: M,
    id: ID,
    body: `# Version A (matthew)\n\nThe elected version.`,
    agent: matthew.agent,
    ensureLocalRoot: true,
    title: `Carried election proof (${M})`,
    description: 'mesh proof fixture',
  });
  console.log(`${M}: authored + declared head ${headM}`);
  const headJ = await authorDeclare({
    storageUrl: J,
    id: ID,
    body: `# Version B (jessica)\n\nThe stale version.`,
    agent: jessica.agent,
    ensureLocalRoot: true,
    title: `Carried election proof (${J})`,
    description: 'mesh proof fixture',
  });
  console.log(`${J}: authored + declared head ${headJ}`);
  if (headM === headJ) throw new Error('fixture failure: heads did not diverge');

  // EARNED canonical on matthew for his head.
  const earned = (await matthew.call('declare_earned_canonical_head', {
    id: ID,
    head_action_hash: headM,
    carried_record: null,
    adopt_before_author: false,
    delegation: null,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- zome answer is untyped at this boundary
  })) as any;
  console.log('EARNED canonical declared on matthew:', {
    head: encodeHashToBase64(earned.head_action_hash),
    canonical: earned.canonical,
  });

  // (3) THE CRUX — carry the election matthew → jessica, verify in wasm.
  const evidence = await canonicalElectionEvidence(matthew, ID);
  if (!evidence?.link_record)
    throw new Error(
      `matthew served no election evidence: ${JSON.stringify(evidence).slice(0, 200)}`
    );
  const linkRecord: Uint8Array = evidence.link_record;
  console.log('evidence from matthew:', {
    winner: evidence.election?.winner_target,
    earned: evidence.election?.canonical_earned,
    linkRecordBytes: linkRecord.length,
  });
  const verified = await verifyCarriedElection(jessica, ID, linkRecord);
  if (!verified) throw new Error('jessica verified NOTHING from carried evidence');
  console.log('VERIFIED on jessica:', {
    winner: verified.winner_target,
    earned: verified.canonical_earned,
    declared_at: verified.canonical_declared_at,
  });
  const winnerB64 = String(verified.winner_target);
  if (winnerB64 !== headM)
    throw new Error(`jessica's merged election chose ${winnerB64}, expected ${headM}`);

  // (4) Anti-regression: a tampered link record must be refused.
  const tampered = tamperLinkRecord(linkRecord);
  let refused = false;
  try {
    await verifyCarriedElection(jessica, ID, tampered);
  } catch (e) {
    refused = true;
    console.log('tampered evidence REFUSED (correct):', String(e).slice(0, 160));
  }
  if (!refused) throw new Error('SECURITY: tampered evidence was NOT refused');

  // (5) Watch jessica's served head converge to the elected head.
  const deadline = Date.now() + 6 * 60_000;
  let converged = false;
  while (Date.now() < deadline) {
    const h = await servedHead(J, ID);
    if (h === headM) {
      converged = true;
      break;
    }
    await new Promise(r => setTimeout(r, 10_000));
  }
  const finalM = await servedHead(M, ID);
  const finalJ = await servedHead(J, ID);
  console.log('final heads:', { matthew: finalM, jessica: finalJ, elected: headM });
  const metrics = await (await fetch(`${J}/metrics`)).text();
  for (const line of metrics.split('\n'))
    if (
      /election_obeyed_total|election_obey_probe_total|election_obey_failed_total/.test(line) &&
      !line.startsWith('#')
    )
      console.log('jessica metric:', line);
  console.log(
    converged
      ? 'CONVERGED: jessica serves the elected head'
      : 'NOT-CONVERGED within 6 min (crux still proven above)'
  );
  process.exit(converged ? 0 : 3);
}
main().catch(e => {
  console.error(String(e).slice(0, 600));
  process.exit(1);
});
