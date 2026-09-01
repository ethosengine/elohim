/**
 * Release soak-attestation rail probe (2026-09-01) — the household-mesh evidence
 * leg for `task-release-soak-attestation-rail`
 * (spec: `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md` §5,
 * BuildAttestation / SoakAttestation).
 *
 * This probe drives the SAME rail
 * `elohim-storage/src/services/release_attestation.rs` composes —
 * `content_store::issue_attestation` to author, then the conductor's own
 * link walk + entry read to count. It is the wire-level mirror of that module,
 * so it can run against a mesh whose storage binary predates it. It STARTS
 * NOTHING: the mesh must already be up.
 *
 * ## Legs
 *
 *   0. FLOOR 1 IS REAL — `attestation:release-soak` (a NEW kind) is REFUSED by
 *      the integrity zome. This is the constraint the whole design bends
 *      around: the generated `ATTESTATION_KINDS` list is compiled INTO
 *      `content_store_integrity`, so a new kind is a DNA-hash move. MVP rides
 *      an existing kind.
 *   1. FLOOR 8 IS REAL — `proof_evidence.class = "audit"` with no `merkle_root`
 *      is REFUSED. The chosen metadata shape therefore passes a LIVE validator
 *      floor, not an unenforced one.
 *   2. Two non-builder peers author `attestation:device-health` entries
 *      carrying the `release-soak` discriminator + context in `proof_evidence`,
 *      anchored on the release CID. Both land.
 *   3. The release's own BUILDER authors one — the C1 negative control.
 *   4. A THIRD peer counts, through ITS OWN conductor, applying the reader's
 *      rule: the link walk gives the AUTHENTICATED (cid, issuer) pairs; the
 *      entry read gives `author_id` + context; the two must agree.
 *
 * ## Two measured substrate defects the read leg reports rather than hides
 *
 * Both live in files the owning atom must not edit, and both DEFLATE a count
 * (never inflate one), so the reader is fail-closed against them:
 *
 *   - **Identity collapse** — the coordinator stamps
 *     `Content.id = "attest-{kind}-{issuer}"`, which is not unique per
 *     attestation and IS the projection's primary key.
 *   - **Provenance laundering** — the generic content-replication path
 *     re-authors that id on receiving peers under THEIR key, so the projected
 *     `issuer_cid` for a foreign attestation is the local agent.
 *
 * The probe prints the SQL projection alongside the conductor read precisely so
 * the divergence is visible in the transcript.
 *
 * ## Exit codes
 *   0 — full DoD: the third peer counts 2 qualifying, builder excluded.
 *   3 — rail proven (floors + authoring + context round-trip) but the
 *       cross-peer count is blocked by a NAMED substrate defect. Not a pass.
 *   2 — an assertion about the rail itself failed.
 *   1 — probe error (mesh down, etc).
 *
 * Run (mesh must already be up):
 *   cd genesis/a2o && pnpm exec tsx scripts/release-attestation-probe.ts
 *
 * Env overrides: MESH_PEER_A/B/C (`name:adminPort:appPort:httpPort`),
 * MESH_GOSSIP_DEADLINE_SECS (default 600).
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';

const APP_ID = 'elohim';

/** The existing generated kind release attestations ride — see the module docs
 *  in `release_attestation.rs` for why device-health and not content-quality. */
const RIDDEN_KIND = 'attestation:device-health';
/** `metadata_json.proof_evidence.kind` discriminator. */
const SOAK_DISCRIMINATOR = 'release-soak';

type PeerSpec = { name: string; admin: number; app: number; http: number };

function peer(envKey: string, fallback: string): PeerSpec {
  const [name, admin, app, http] = (process.env[envKey] ?? fallback).split(':');
  return { name, admin: Number(admin), app: Number(app), http: Number(http) };
}

// Port scheme per peer index i (app/elohim-app/scripts/hc-mesh.sh:244):
// admin 4444+10i, app 4445+10i, storage http 8090+i.
const PEER_A = peer('MESH_PEER_A', 'matthew:4444:4445:8090');
const PEER_B = peer('MESH_PEER_B', 'jessica:4454:4455:8091');
const PEER_C = peer('MESH_PEER_C', 'james:4464:4465:8092');
const GOSSIP_DEADLINE_SECS = Number(process.env.MESH_GOSSIP_DEADLINE_SECS ?? 600);

async function conductor(spec: PeerSpec) {
  const wsOpts: any = { origin: APP_ID };
  const admin = await AdminWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${spec.admin}`),
    wsClientOptions: wsOpts,
  });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === APP_ID) ?? apps[0];
  const cells = new Map<string, any>();
  for (const [role, infos] of Object.entries(app.cell_info))
    for (const info of infos as any[])
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
  const lamad = cells.get('lamad');
  if (!lamad) throw new Error(`${spec.name}: no provisioned lamad cell on :${spec.admin}`);
  await admin.authorizeSigningCredentials(lamad);
  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${spec.app}`),
    token: (await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id }))
      .token,
    wsClientOptions: wsOpts,
  });
  const call = (fn_name: string, payload: any) =>
    appWs.callZome({ cell_id: lamad, zome_name: 'content_store', fn_name, payload });
  return { ...spec, call, agent: encodeHashToBase64(lamad[1]) };
}

type Peer = Awaited<ReturnType<typeof conductor>>;

const rfc3339 = (d: Date) => d.toISOString().replace(/\.\d+Z$/, 'Z');

/**
 * The wire shape `release_attestation::build_soak_input` produces.
 * `metadata` stays conformant to the ridden kind's declared metadata schema
 * (device-health-metadata.schema.json, additionalProperties:false); the
 * release discriminator + context ride in `proof_evidence`, which floor 8
 * validates and which the projection carries verbatim.
 */
function soakInput(opts: {
  releaseCid: string;
  channelId: string;
  deviceId: string;
  deviceArchetype: string;
  capabilityLevel: number;
  region: string;
  outcome: 'pass' | 'fail';
  probeResults: { name: string; ok: boolean; detail?: string }[];
  buildInfo: Record<string, string>;
  windowStart: Date;
  windowEnd: Date;
}) {
  const passed = opts.probeResults.filter(p => p.ok).length;
  return {
    attestation_kind: RIDDEN_KIND,
    subject_cid: opts.releaseCid,
    subject_kind: 'content',
    title: `Release soak: ${opts.releaseCid} on ${opts.deviceArchetype}`,
    description: `soak ${opts.outcome} — ${passed}/${opts.probeResults.length} probes green`,
    reach: 'community',
    metadata: {
      device_id: opts.deviceId,
      health_metric: 'availability',
      period_start: rfc3339(opts.windowStart),
      period_end: rfc3339(opts.windowEnd),
      sample_count: Math.max(opts.probeResults.length, 1),
      summary_value: `${SOAK_DISCRIMINATOR} ${opts.outcome} ${passed}/${opts.probeResults.length}`,
    },
    parent_governance_action_cid: null,
    vote_value: null,
    proof_class: 'witness',
    proof_evidence: {
      class: 'witness',
      kind: SOAK_DISCRIMINATOR,
      releaseCid: opts.releaseCid,
      channelId: opts.channelId,
      deviceArchetype: opts.deviceArchetype,
      capabilityLevel: opts.capabilityLevel,
      region: opts.region,
      outcome: opts.outcome,
      probeResults: opts.probeResults,
      buildInfo: opts.buildInfo,
      soakWindow: { start: rfc3339(opts.windowStart), end: rfc3339(opts.windowEnd) },
    },
    expires_at: null,
  };
}

/** The coordinator's deterministic content id (content_store/src/attestation.rs). */
const attestationContentId = (kind: string, issuer: string) => `attest-${kind}-${issuer}`;

type Verdict =
  | { v: 'qualifies'; agent: string; archetype: string }
  | { v: 'excluded' }
  | { v: 'failed' }
  | { v: 'provenance-mismatch'; linkIssuer: string; entryAuthor: string | null }
  | { v: 'unresolved'; why: string }
  | { v: 'not-release-evidence' };

/**
 * Wire mirror of `release_attestation::classify` + `::tally`.
 * Reads THROUGH the conductor: the link walk supplies the authenticated issuer,
 * the entry read supplies the context, and the two must agree.
 */
async function countQualifying(reader: Peer, releaseCid: string, builderAgent: string) {
  const linked: any[] = (await reader.call('get_attestations_for_subject', releaseCid)) as any[];
  const issuers = [...new Set(linked.map(l => String(l.issuer_cid)))];
  const verdicts: Verdict[] = [];
  for (const issuer of issuers) {
    const got: any = await reader.call('get_content_by_id', {
      id: attestationContentId(RIDDEN_KIND, issuer),
    });
    if (!got?.content) {
      verdicts.push({ v: 'unresolved', why: 'entry not on this conductor' });
      continue;
    }
    const c = got.content;
    if (c.author_id !== issuer) {
      verdicts.push({
        v: 'provenance-mismatch',
        linkIssuer: issuer,
        entryAuthor: c.author_id ?? null,
      });
      continue;
    }
    let meta: any = {};
    try {
      meta = JSON.parse(c.metadata_json ?? '{}');
    } catch {
      verdicts.push({ v: 'unresolved', why: 'metadata_json unparseable' });
      continue;
    }
    const pe = meta.proof_evidence ?? {};
    if (meta.subject_cid !== releaseCid || pe.releaseCid !== releaseCid) {
      verdicts.push({
        v: 'unresolved',
        why: `id collision — resolved to ${pe.releaseCid ?? 'a non-release entry'}`,
      });
      continue;
    }
    if (pe.kind !== SOAK_DISCRIMINATOR || meta.revocation) {
      verdicts.push({ v: 'not-release-evidence' });
      continue;
    }
    if (issuer === builderAgent) {
      verdicts.push({ v: 'excluded' });
      continue;
    }
    if (pe.outcome !== 'pass') {
      verdicts.push({ v: 'failed' });
      continue;
    }
    verdicts.push({
      v: 'qualifies',
      agent: issuer,
      archetype: String(pe.deviceArchetype ?? 'unknown'),
    });
  }

  const byArchetype: Record<string, number> = {};
  const counted = new Set<string>();
  let qualifying = 0,
    total = 0,
    excluded = 0,
    failed = 0,
    mismatched = 0,
    unresolved = 0;
  for (const v of verdicts) {
    if (v.v === 'not-release-evidence') continue;
    total += 1;
    if (v.v === 'qualifies') {
      if (counted.has(v.agent)) continue;
      counted.add(v.agent);
      qualifying += 1;
      byArchetype[v.archetype] = (byArchetype[v.archetype] ?? 0) + 1;
    } else if (v.v === 'excluded') excluded += 1;
    else if (v.v === 'failed') failed += 1;
    else if (v.v === 'provenance-mismatch') mismatched += 1;
    else unresolved += 1;
  }
  return {
    linkedCount: linked.length,
    qualifying,
    total,
    byArchetype,
    excluded,
    failed,
    mismatched,
    unresolved,
    verdicts,
  };
}

/** The SQL projection, printed for CONTRAST — it is not the reader's source. */
async function projection(http: number, releaseCid: string) {
  const url =
    `http://localhost:${http}/api/v1/attestations/unified` +
    `?subjectCid=${encodeURIComponent(releaseCid)}&kind=${encodeURIComponent(RIDDEN_KIND)}`;
  const r = await fetch(url);
  if (!r.ok) return [];
  return (await r.json()) as any[];
}

async function refused(label: string, fn: () => Promise<unknown>): Promise<string> {
  try {
    await fn();
  } catch (e) {
    return String(e);
  }
  throw new Error(`${label}: expected a REFUSAL, got acceptance`);
}

async function main() {
  const a = await conductor(PEER_A);
  const b = await conductor(PEER_B);
  const c = await conductor(PEER_C);
  console.log(
    `peers: ${a.name}=${a.agent}\n       ${b.name}=${b.agent}\n       ${c.name}=${c.agent}`
  );

  // C is BOTH the release's builder (provenance.builderAgent) and the reader —
  // the sharpest shape for C1: the peer doing the counting is the one whose own
  // attestation must not count.
  const builderAgent = c.agent;
  const RELEASE = `release-soak-probe-${Date.now()}`;
  const CHANNEL = 'runtime:coordinator-bundle:household:probe';
  const BUILD_INFO = { version: '0.0.0-probe', commit: 'probe', service: 'elohim-storage' };
  console.log(`fixture release cid: ${RELEASE}`);
  console.log(`builder agent (C1 exclusion): ${builderAgent}`);

  const base = {
    releaseCid: RELEASE,
    channelId: CHANNEL,
    region: 'household',
    buildInfo: BUILD_INFO,
    outcome: 'pass' as const,
  };
  const window = () => ({ windowStart: new Date(Date.now() - 600_000), windowEnd: new Date() });

  // ---- 0. Extern delivery + FLOOR 1 is real ------------------------------
  const emptyRead = await c.call('get_attestations_for_subject', RELEASE);
  console.log(`extern delivery ok — link walk for a fresh cid = ${JSON.stringify(emptyRead)}`);

  const floor1 = await refused('floor1', () =>
    a.call('issue_attestation', {
      ...soakInput({
        ...base,
        deviceId: a.agent,
        deviceArchetype: 'workstation',
        capabilityLevel: 4,
        probeResults: [{ name: 'noop', ok: true }],
        ...window(),
      }),
      attestation_kind: 'attestation:release-soak', // a NEW kind — must be refused
    })
  );
  if (!/unknown_attestation_subtype/.test(floor1))
    throw new Error(
      `floor1 refusal did not name unknown_attestation_subtype: ${floor1.slice(0, 300)}`
    );
  console.log('FLOOR 1 PROVEN: a new attestation kind is refused (a new kind IS a DNA-hash move)');

  // ---- 1. FLOOR 8 is real -------------------------------------------------
  const floor8 = await refused('floor8', () => {
    const bad: any = soakInput({
      ...base,
      deviceId: a.agent,
      deviceArchetype: 'workstation',
      capabilityLevel: 4,
      probeResults: [{ name: 'noop', ok: true }],
      ...window(),
    });
    bad.proof_evidence = { ...bad.proof_evidence, class: 'audit' }; // audit w/o merkle_root
    return a.call('issue_attestation', bad);
  });
  if (!/floor8_failed/.test(floor8))
    throw new Error(`floor8 refusal did not name floor8_failed: ${floor8.slice(0, 300)}`);
  console.log(
    'FLOOR 8 PROVEN: proof_evidence floors are live — the chosen shape passes a real floor'
  );

  // ---- 2. Two non-builder peers author the soak evidence ------------------
  const authored: Record<string, string> = {};
  for (const [p, archetype, capability] of [
    [a, 'workstation', 4],
    [b, 'home-server', 3],
  ] as [Peer, string, number][]) {
    const out: any = await p.call(
      'issue_attestation',
      soakInput({
        ...base,
        deviceId: p.agent,
        deviceArchetype: archetype,
        capabilityLevel: capability,
        probeResults: [
          { name: 'health', ok: true },
          { name: 'p2p-status', ok: true },
          { name: 'canonical-head', ok: true },
        ],
        ...window(),
      })
    );
    authored[p.name] = out.cid;
    console.log(`${p.name} authored soak attestation ${out.cid} (${archetype})`);
  }

  // ---- 3. The BUILDER authors one too — the C1 negative control -----------
  const builderOut: any = await c.call(
    'issue_attestation',
    soakInput({
      ...base,
      deviceId: c.agent,
      deviceArchetype: 'builder-node',
      capabilityLevel: 4,
      probeResults: [{ name: 'self-soak', ok: true }],
      ...window(),
    })
  );
  authored[`${c.name} (BUILDER)`] = builderOut.cid;
  console.log(`${c.name} (BUILDER) authored ${builderOut.cid} — must NOT qualify`);
  console.log('CONTEXT ROUND-TRIP: all three authored — floors passed for the chosen shape');

  // ---- 4. The third peer counts, through its OWN conductor ----------------
  const deadline = Date.now() + GOSSIP_DEADLINE_SECS * 1000;
  let evidence = await countQualifying(c, RELEASE, builderAgent);
  while (Date.now() < deadline && evidence.total < 3) {
    await new Promise(r => setTimeout(r, 15_000));
    evidence = await countQualifying(c, RELEASE, builderAgent);
    console.log(
      `  …${c.name} link walk sees ${evidence.linkedCount}/3; qualifying=${evidence.qualifying}`
    );
  }

  console.log(
    "CONDUCTOR READ (the reader's source):",
    JSON.stringify({
      linkedCount: evidence.linkedCount,
      qualifying: evidence.qualifying,
      total: evidence.total,
      excluded: evidence.excluded,
      mismatched: evidence.mismatched,
      unresolved: evidence.unresolved,
      byArchetype: evidence.byArchetype,
    })
  );
  for (const v of evidence.verdicts) console.log('   verdict:', JSON.stringify(v));

  // Contrast: the SQL projection, which is NOT the reader's source.
  const proj = await projection(c.http, RELEASE);
  console.log(`SQL PROJECTION on ${c.name} (contrast only): ${proj.length} row(s)`);
  for (const row of proj) {
    const idIssuer = String(row.id).replace(`attest-${RIDDEN_KIND}-`, '');
    const laundered = idIssuer !== row.issuerCid;
    console.log(
      `   id-issuer=${idIssuer.slice(0, 20)}… issuerCid=${String(row.issuerCid).slice(0, 20)}…` +
        (laundered ? '  ← LAUNDERED (issuer column rewritten to the local agent)' : '')
    );
  }

  // ---- Verdict ------------------------------------------------------------
  if (evidence.qualifying === 2 && evidence.excluded === 1 && evidence.total === 3) {
    const arch = Object.keys(evidence.byArchetype).sort().join(',');
    if (arch !== 'home-server,workstation')
      throw new Error(`byArchetype not context-bearing: ${JSON.stringify(evidence.byArchetype)}`);
    console.log(
      `PROBE PASS: release=${RELEASE} total=3 qualifying=2 builder-excluded=1 ` +
        `byArchetype=${JSON.stringify(evidence.byArchetype)}`
    );
    process.exit(0);
  }

  console.log('\nPROBE: rail PROVEN, cross-peer count BLOCKED. Named diagnosis:');
  if (evidence.linkedCount < 3)
    console.log(
      `  - LINK GOSSIP: ${c.name}'s conductor sees ${evidence.linkedCount}/3 AttestationToSubject ` +
        `links after ${GOSSIP_DEADLINE_SECS}s. A threshold reader cannot count attestations its ` +
        `own conductor cannot see. (Not a defect in this rail — a substrate convergence question.)`
    );
  if (evidence.mismatched > 0)
    console.log(
      `  - PROVENANCE LAUNDERING: ${evidence.mismatched} entr(ies) whose author_id disagrees with ` +
        `the link-walk issuer — a re-authored copy shadowing the real one. Fail-closed here, but ` +
        `it deflates the count. Root: Content.id = "attest-{kind}-{issuer}" is replicated as an ` +
        `ordinary content id.`
    );
  if (evidence.unresolved > 0)
    console.log(
      `  - UNRESOLVED: ${evidence.unresolved}. Root: the same non-unique Content.id — the issuer's ` +
        `attestation id can resolve to a different release, or not be present locally at all.`
    );
  console.log(`  authored: ${JSON.stringify(authored)}`);
  console.log(
    '  Proven regardless: floors 1 + 8 are live, all three attestations committed, and the\n' +
      '  discriminator + context round-tripped intact through metadata_json.'
  );
  process.exit(3);
}

main().catch(e => {
  console.error('PROBE ERROR:', String(e).slice(0, 900));
  process.exit(1);
});
