/**
 * Provision a deploy-service-agent keypair for substrate-correct Z.D deploys.
 *
 * Per spec §2 "Roles" and §7.1 key custody. Generates an Ed25519 keypair,
 * derives an agent CID from the public key, writes the public-side artifact
 * (committable to repo for audit), and prints the PRIVATE key JWK to stdout
 * for operator-stash into Jenkins as a "Secret text" credential.
 *
 * The private key NEVER touches the filesystem — operator pipes stdout into
 * the Jenkins credentials UI directly.
 *
 * ## Blueprint status
 *
 * This file is authored as a blueprint (T8 of Sprint 1, plan
 * 2026-05-28-sprint1-zd-substrate-correct-deploy.md). The orchestrator package
 * has NO TypeScript runtime today — no ts-node, no jose, no tsconfig.json. A
 * follow-up task wires the runtime infrastructure (ts-node + deps + tsconfig)
 * and the Jenkinsfile invocation. Until then, this script documents the exact
 * wire shape, the cryptographic invariants, and the operator-facing flow.
 *
 * ## Runtime infrastructure (deferred)
 *
 * - npm: `jose` (Ed25519 keypair generation), `multiformats/hashes/sha2` (CID derivation)
 * - ts-node + tsconfig (per genesis/orchestrator/tsconfig.json, to be added in follow-up)
 *
 * ## Invocation (after follow-up wires deps)
 *
 * ```bash
 * ts-node genesis/orchestrator/scripts/provision-deploy-agent.ts \
 *   --operator=matthew \
 *   --outDir=genesis/orchestrator/deploy-agents \
 *   --jenkinsCredId=deploy-svc-matthew-doorway
 * ```
 *
 * ## Output
 *
 * Writes: `<outDir>/agent:deploy-svc-<operator>-<hash16>.public.json` containing
 * `{ agentCid, operator, publicKey: <JWK>, provisionedAt }`. Commit this file.
 *
 * Prints to stdout: the private key JWK as JSON. Operator pastes this into
 * Jenkins → Credentials → "Secret text" with the ID specified by --jenkinsCredId.
 */

import { generateKeyPair, exportJWK } from 'jose';
import { writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';

interface ProvisionArgs {
  operator: string;       // e.g. "matthew"
  outDir: string;         // where to write the public-side artifact
  jenkinsCredId: string;  // Jenkins credential ID label the operator will use
}

function parseArgs(argv: string[]): ProvisionArgs {
  const args: Partial<ProvisionArgs> = {};
  for (const a of argv) {
    const m = a.match(/^--(\w+)=(.*)$/);
    if (m) (args as Record<string, string>)[m[1]] = m[2];
  }
  if (!args.operator || !args.outDir || !args.jenkinsCredId) {
    console.error(
      'Usage: ts-node provision-deploy-agent.ts --operator=<name> --outDir=<dir> --jenkinsCredId=<id>',
    );
    process.exit(2);
  }
  return args as ProvisionArgs;
}

async function main(args: ProvisionArgs): Promise<void> {
  // 1. Generate Ed25519 keypair (extractable so we can export both halves).
  const { privateKey, publicKey } = await generateKeyPair('EdDSA', {
    crv: 'Ed25519',
    extractable: true,
  });

  // 2. Derive agent CID = sha256(publicKey raw bytes), prefixed.
  //    JWK 'x' is base64url-encoded raw public key bytes for OKP/Ed25519.
  const pubJwk = await exportJWK(publicKey);
  const pubRawB64Url = (pubJwk as { x: string }).x;
  const pubBytes = Buffer.from(pubRawB64Url, 'base64url');
  const hashHex = createHash('sha256').update(pubBytes).digest('hex');
  const agentCid = `agent:deploy-svc-${args.operator}-${hashHex.slice(0, 16)}`;

  // 3. Write public-side artifact (committable to repo).
  mkdirSync(args.outDir, { recursive: true });
  const publicPath = `${args.outDir}/${agentCid}.public.json`;
  writeFileSync(
    publicPath,
    JSON.stringify(
      {
        agentCid,
        operator: args.operator,
        publicKey: pubJwk,
        provisionedAt: new Date().toISOString(),
      },
      null,
      2,
    ),
  );

  // 4. Print PRIVATE key JWK to stdout (NEVER written to filesystem).
  const privateJwk = await exportJWK(privateKey);
  console.log('---');
  console.log(`AgentCid: ${agentCid}`);
  console.log(`Public artifact: ${publicPath}`);
  console.log(`Jenkins credential ID to create: ${args.jenkinsCredId}`);
  console.log('PASTE THE FOLLOWING INTO JENKINS AS A "Secret text" CREDENTIAL:');
  console.log(JSON.stringify(privateJwk));
  console.log('---');
}

const requireMainModule = typeof require !== 'undefined' && require.main === module;
if (requireMainModule) {
  main(parseArgs(process.argv.slice(2))).catch((e) => {
    console.error('provision-deploy-agent failed:', e);
    process.exit(1);
  });
}
