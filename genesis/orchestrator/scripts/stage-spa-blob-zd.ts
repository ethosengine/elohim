/**
 * Substrate-correct deploy script — replaces stage-spa-blob.sh Z.1 anti-pattern.
 *
 * Implements the 8-step Z.D CI flow per spec §2:
 *
 *   1. Tar the dist directory and compute its blob CID (sha256 + dag-cbor codec).
 *   2. Upload the bytes via PUT /blob/<cid>.
 *   3. Fetch the previous EprHead envelope (for supersedes).
 *   4. Construct the new EprHead envelope.
 *   5. Sign canonically with the deploy-svc-agent's Ed25519 key (dag-cbor-encoded body).
 *   6. Compute the new EprHead CID.
 *   7. Construct the republish-epr EconomicEvent referencing the deploy commitment.
 *   8. PUT /api/v1/epr/<eprHeadCid> with { envelope, payload, event } body.
 *
 * Schema: elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json.
 *
 * ## Blueprint status
 *
 * This file is authored as a blueprint (T10 of Sprint 1, plan
 * 2026-05-28-sprint1-zd-substrate-correct-deploy.md). The orchestrator package
 * has NO TypeScript runtime today — no ts-node, no jose, no @ipld/dag-cbor, no
 * multiformats. A follow-up task wires the runtime infrastructure and the
 * Jenkinsfile invocation (Task 11 of the plan). Until then, this script
 * documents the exact 8-step substrate-correct deploy flow and the wire shape
 * the doorway/storage endpoint expects.
 *
 * ## Runtime infrastructure (deferred)
 *
 * - npm: `jose` (signing), `@ipld/dag-cbor`, `multiformats/cid`, `multiformats/hashes/sha2`
 *
 * ## Invocation (from Jenkinsfile — see Task 11)
 *
 * ```bash
 * ts-node genesis/orchestrator/scripts/stage-spa-blob-zd.ts \
 *   --distDir=app/elohim-app/dist \
 *   --deploySvcAgentCid="$DEPLOY_SVC_AGENT_CID" \
 *   --deployCommitmentCid="$DEPLOY_COMMITMENT_CID" \
 *   --privateKeyJwk="$DEPLOY_PRIVATE_KEY_JWK" \
 *   --storageApiBase="https://doorway.elohim.host" \
 *   --manifestPath=app/elohim-app/dist/manifest.json
 * ```
 */

import { readFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { importJWK, CompactSign } from 'jose';
import { encode as cborEncode } from '@ipld/dag-cbor';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

interface Args {
  distDir: string;
  deploySvcAgentCid: string;
  deployCommitmentCid: string;
  privateKeyJwk: string;   // JSON-encoded private JWK
  storageApiBase: string;
  manifestPath: string;
}

function parseArgs(argv: string[]): Args {
  const args: Partial<Args> = {};
  for (const a of argv) {
    const m = a.match(/^--(\w+)=(.*)$/);
    if (m) (args as Record<string, string>)[m[1]] = m[2];
  }
  const required = [
    'distDir',
    'deploySvcAgentCid',
    'deployCommitmentCid',
    'privateKeyJwk',
    'storageApiBase',
    'manifestPath',
  ];
  for (const k of required) {
    if (!(k in args)) {
      console.error(`Missing --${k}`);
      process.exit(2);
    }
  }
  return args as Args;
}

interface BundleManifest {
  eprIdentity: string;
  reach?: string;
  bundlePath?: string;
}

async function main(args: Args): Promise<void> {
  // Step 1: tar dist + compute blob CID
  // dag-cbor codec = 0x71 per elohim/epr/src/cid.rs convention.
  const tarBytes = execSync(`tar c -C ${args.distDir} .`);
  const blobMultihash = await sha256.digest(tarBytes);
  const blobCid = CID.create(1, 0x71, blobMultihash);
  const blobCidString = blobCid.toString();

  // Step 2: PUT blob bytes
  const blobResp = await fetch(`${args.storageApiBase}/blob/${blobCidString}`, {
    method: 'PUT',
    body: tarBytes,
    headers: { 'Content-Type': 'application/octet-stream' },
  });
  if (!blobResp.ok) {
    throw new Error(`PUT /blob failed (${blobResp.status}): ${await blobResp.text()}`);
  }

  // Step 3: fetch prev EprHead envelope (if any)
  const manifest = JSON.parse(readFileSync(args.manifestPath, 'utf-8')) as BundleManifest;
  const prevResp = await fetch(
    `${args.storageApiBase}/api/v1/epr/by-identity/${manifest.eprIdentity}/head`,
  );
  const prevEnvelope = prevResp.ok ? ((await prevResp.json()) as { cid?: string }) : null;
  const supersedes = prevEnvelope?.cid ?? null;

  // Step 4: construct new EprHead envelope
  const reach = manifest.reach || 'commons';
  const envelopeBody = {
    kind: 'Content',
    reach,
    coupling: { knowledge: manifest.eprIdentity },
    payload: { cid: blobCidString },
    supersedes,
    proof: { signer: args.deploySvcAgentCid },
  };

  // Step 5: sign canonically (dag-cbor encode + JWS detached sig)
  const envelopeBytes = cborEncode(envelopeBody);
  const privateKey = await importJWK(JSON.parse(args.privateKeyJwk), 'EdDSA');
  const jws = await new CompactSign(envelopeBytes)
    .setProtectedHeader({ alg: 'EdDSA' })
    .sign(privateKey);
  const envelope = {
    ...envelopeBody,
    proof: { ...envelopeBody.proof, signature: jws },
  };

  // Step 6: compute EprHead CID
  const eprHeadBytes = cborEncode(envelope);
  const eprHeadMultihash = await sha256.digest(eprHeadBytes);
  const eprHeadCid = CID.create(1, 0x71, eprHeadMultihash).toString();

  // Step 7: construct republish-epr EconomicEvent per
  // elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json.
  // Required fields: action, performer, bounded_by, target, payload, signed_at.
  // payload requires: blob_cid, epr_kind, reach.
  const event = {
    action: 'republish-epr',
    performer: args.deploySvcAgentCid,
    bounded_by: args.deployCommitmentCid,
    target: eprHeadCid,
    supersedes,
    payload: {
      blob_cid: blobCidString,
      epr_kind: 'Content',
      reach,
      bundle_path: manifest.bundlePath ?? null,
    },
    signed_at: new Date().toISOString(),
  };

  // Step 8: PUT /api/v1/epr/<eprHeadCid> with { envelope, payload (hex), event }
  const putResp = await fetch(`${args.storageApiBase}/api/v1/epr/${eprHeadCid}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      envelope,
      payload: Buffer.from(envelopeBytes).toString('hex'),
      event,
    }),
  });
  if (!putResp.ok) {
    throw new Error(`PUT /api/v1/epr failed (${putResp.status}): ${await putResp.text()}`);
  }

  console.log(`Z.D deploy succeeded: EprHead=${eprHeadCid}, blob=${blobCidString}`);
}

main(parseArgs(process.argv.slice(2))).catch((e) => {
  console.error('Z.D deploy failed:', e);
  process.exit(1);
});
