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
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';

const APP_ID = 'elohim';
const ID = `carried-election-proof-${Date.now()}`;

async function conductor(adminPort: number, appPort: number) {
  const wsOpts: any = { origin: APP_ID };
  const admin = await AdminWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${adminPort}`),
    wsClientOptions: wsOpts,
  });
  const apps = await admin.listApps({});
  const app = apps.find((a) => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info))
    for (const info of infos as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
  const lamad = cells.get('lamad');
  await admin.authorizeSigningCredentials(lamad);
  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${appPort}`),
    token: (await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id }))
      .token,
    wsClientOptions: wsOpts,
  });
  const call = (fn_name: string, payload: any) =>
    appWs.callZome({ cell_id: lamad, zome_name: 'content_store', fn_name, payload });
  return { admin, appWs, call, agent: encodeHashToBase64(lamad[1]) };
}

async function authorDeclare(storage: string, body: string, agent: string) {
  const bulk = await fetch(`${storage}/db/content/bulk`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify([
      {
        id: ID,
        title: `Carried election proof (${storage})`,
        description: 'mesh proof fixture',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: body,
        reach: 'commons',
      },
    ]),
  });
  if (!bulk.ok) throw new Error(`bulk create on ${storage}: ${bulk.status} ${await bulk.text()}`);
  const patch = await fetch(`${storage}/db/content/${ID}`, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ contentBody: body, reach: 'commons' }),
  });
  const patched: any = await patch.json().catch(() => ({}));
  const anchor = patched.dhtAnchorHash;
  if (!anchor)
    throw new Error(`no dhtAnchorHash after PATCH on ${storage}: ${JSON.stringify(patched).slice(0, 200)}`);
  const head = await fetch(`${storage}/db/content/${ID}/head`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'X-Agent-Cid': agent },
    body: JSON.stringify({ headActionHash: anchor }),
  });
  if (!head.ok) throw new Error(`declare on ${storage}: ${head.status} ${await head.text()}`);
  console.log(`${storage}: authored + declared head ${anchor}`);
  return anchor as string;
}

async function servedHead(storage: string): Promise<string | null> {
  const r = await fetch(`${storage}/db/content/${ID}/head`);
  if (!r.ok) return null;
  const j: any = await r.json().catch(() => null);
  return j?.headActionHash ?? null;
}

async function main() {
  const M = 'http://localhost:8090';
  const J = 'http://localhost:8091';
  const matthew = await conductor(4444, 4445);
  const jessica = await conductor(4454, 4455);
  console.log('matthew agent:', matthew.agent);
  console.log('jessica agent:', jessica.agent);

  // (1) Extern delivery check — unknown-function here means a stale DNA.
  const probe = await matthew.call('get_canonical_election_evidence', 'no-such-id');
  console.log('extern delivery: get_canonical_election_evidence(no-such-id) =', probe);

  // (2) Divergent declared heads, one id, two roots.
  const headM = await authorDeclare(M, `# Version A (matthew)\n\nThe elected version.`, matthew.agent);
  const headJ = await authorDeclare(J, `# Version B (jessica)\n\nThe stale version.`, jessica.agent);
  if (headM === headJ) throw new Error('fixture failure: heads did not diverge');

  // EARNED canonical on matthew for his head.
  const earned: any = await matthew.call('declare_earned_canonical_head', {
    id: ID,
    head_action_hash: headM,
    carried_record: null,
    adopt_before_author: false,
    delegation: null,
  });
  console.log('EARNED canonical declared on matthew:', {
    head: encodeHashToBase64(earned.head_action_hash),
    canonical: earned.canonical,
  });

  // (3) THE CRUX — carry the election matthew → jessica, verify in wasm.
  const evidence: any = await matthew.call('get_canonical_election_evidence', ID);
  if (!evidence?.link_record) throw new Error(`matthew served no election evidence: ${JSON.stringify(evidence).slice(0, 200)}`);
  const linkRecord: Uint8Array = evidence.link_record;
  console.log('evidence from matthew:', {
    winner: evidence.election.winner_target,
    earned: evidence.election.canonical_earned,
    linkRecordBytes: linkRecord.length,
  });
  const verified: any = await jessica.call('verify_carried_election', {
    id: ID,
    link_record: linkRecord,
  });
  if (!verified) throw new Error('jessica verified NOTHING from carried evidence');
  console.log('VERIFIED on jessica:', {
    winner: verified.winner_target,
    earned: verified.canonical_earned,
    declared_at: verified.canonical_declared_at,
  });
  const winnerB64 = String(verified.winner_target);
  if (winnerB64 !== headM) throw new Error(`jessica's merged election chose ${winnerB64}, expected ${headM}`);

  // (4) Anti-regression: a tampered link record must be refused.
  const tampered = new Uint8Array(linkRecord);
  tampered[tampered.length - 5] ^= 0xff;
  let refused = false;
  try {
    await jessica.call('verify_carried_election', { id: ID, link_record: tampered });
  } catch (e) {
    refused = true;
    console.log('tampered evidence REFUSED (correct):', String(e).slice(0, 160));
  }
  if (!refused) throw new Error('SECURITY: tampered evidence was NOT refused');

  // (5) Watch jessica's served head converge to the elected head.
  const deadline = Date.now() + 6 * 60_000;
  let converged = false;
  while (Date.now() < deadline) {
    const h = await servedHead(J);
    if (h === headM) {
      converged = true;
      break;
    }
    await new Promise((r) => setTimeout(r, 10_000));
  }
  const finalM = await servedHead(M);
  const finalJ = await servedHead(J);
  console.log('final heads:', { matthew: finalM, jessica: finalJ, elected: headM });
  const metrics = await (await fetch(`${J}/metrics`)).text();
  for (const line of metrics.split('\n'))
    if (/election_obeyed_total|election_obey_probe_total|election_obey_failed_total/.test(line) && !line.startsWith('#'))
      console.log('jessica metric:', line);
  console.log(converged ? 'CONVERGED: jessica serves the elected head' : 'NOT-CONVERGED within 6 min (crux still proven above)');
  process.exit(converged ? 0 : 3);
}
main().catch((e) => {
  console.error(String(e).slice(0, 600));
  process.exit(1);
});
