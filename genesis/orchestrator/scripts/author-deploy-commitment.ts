/**
 * Operator-steward CLI to author a delegates-compute Commitment via the
 * Mishpat zome.
 *
 * Per spec §2 "Bounds (the Commitment payload)". Interactive prompts collect
 * the Commitment fields; the script signs via the operator's stewarded
 * Holochain identity (lair-managed keys), calls Mishpat's `create_commitment`
 * extern, and prints the resulting Commitment CID for operator-stash into
 * Jenkins as "DEPLOY_COMMITMENT_CID".
 *
 * Schema: elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json
 *
 * ## Blueprint status
 *
 * This file is authored as a blueprint (T9 of Sprint 1, plan
 * 2026-05-28-sprint1-zd-substrate-correct-deploy.md). The orchestrator package
 * has NO TypeScript runtime today — no ts-node, no inquirer, no
 * @holochain/client. A follow-up task wires the runtime infrastructure. Until
 * then, this script documents the interactive Commitment-authoring flow and
 * the exact wire shape that hits the Mishpat zome.
 *
 * ## Runtime infrastructure (deferred)
 *
 * - npm: `inquirer` (prompts), `@holochain/client` (AppWebsocket → conductor)
 *
 * ## Pre-conditions
 *
 * - hc-dev-orchestrator (or alpha conductor) running locally with app-port at
 *   ws://localhost:8888
 * - operator's steward identity unlocked in lair
 * - deploy-svc-agent provisioned via provision-deploy-agent.ts; agent CID
 *   copied from the public artifact
 *
 * ## Invocation
 *
 * ```bash
 * ts-node genesis/orchestrator/scripts/author-deploy-commitment.ts
 * ```
 */

import inquirer from 'inquirer';
import { AppWebsocket } from '@holochain/client';

interface CommitmentInput {
  provider: string;          // operator-steward CID
  recipient: string;         // deploy-svc-agent CID (from provision-deploy-agent output)
  epr_scope: string[];       // EPR identifiers, or ["*"] for bootstrap
  reach_ceiling: string;     // "commons" default
  rate_per_hour: number;     // 30 default
  rotation_ttl_days: number; // 90 default
}

// Reach ladder (low → high). Schema enforces that anything stricter than
// 'commons'/'community' requires reach_elevation_acknowledged=true.
const VALID_REACHES = [
  'private',
  'self',
  'intimate',
  'trusted',
  'familiar',
  'community',
  'public',
  'commons',
];

async function main(): Promise<void> {
  const answers = await inquirer.prompt<CommitmentInput>([
    { type: 'input', name: 'provider', message: 'Operator-steward agent CID:' },
    {
      type: 'input',
      name: 'recipient',
      message: 'Deploy-svc-agent CID (from provision-deploy-agent output):',
    },
    {
      type: 'input',
      name: 'epr_scope',
      message: 'EPR scope (comma-separated, or "*" for bootstrap):',
      filter: (s: string) => s.split(',').map((x) => x.trim()),
    },
    {
      type: 'list',
      name: 'reach_ceiling',
      message:
        'Reach ceiling (commons/community are default-allowed; higher requires reach_elevation_acknowledged):',
      default: 'commons',
      choices: VALID_REACHES,
    },
    { type: 'number', name: 'rate_per_hour', message: 'Rate per hour:', default: 30 },
    { type: 'number', name: 'rotation_ttl_days', message: 'Rotation TTL (days):', default: 90 },
  ]);

  const validFrom = new Date().toISOString();
  const validUntil = new Date(
    Date.now() + answers.rotation_ttl_days * 86400 * 1000,
  ).toISOString();

  // Per delegates-compute.schema.json shape. Required fields:
  // action, scope, provider, recipient, bounds, valid_from, valid_until.
  // bounds requires: epr_scope, reach_ceiling, rate_per_hour, rotation_ttl_days.
  const payload = {
    action: 'delegates-compute',
    scope: 'republish-epr',
    provider: answers.provider,
    recipient: answers.recipient,
    bounds: {
      epr_scope: answers.epr_scope,
      reach_ceiling: answers.reach_ceiling,
      rate_per_hour: answers.rate_per_hour,
      rotation_ttl_days: answers.rotation_ttl_days,
    },
    valid_from: validFrom,
    valid_until: validUntil,
  };

  const app = await AppWebsocket.connect({ url: new URL('ws://localhost:8888') });
  const result = await app.callZome({
    role_name: 'mishpat',
    zome_name: 'mishpat',
    fn_name: 'create_commitment',
    payload: {
      action: 'delegates-compute',
      payload_json: JSON.stringify(payload),
    },
  });

  const actionHash = (result as { action_hash: string }).action_hash;
  console.log('---');
  console.log(`Commitment CID: ${actionHash}`);
  console.log(
    'Stash this CID in Jenkins as "DEPLOY_COMMITMENT_CID" credential for the operator.',
  );
  console.log('---');
}

main().catch((e) => {
  console.error('author-deploy-commitment failed:', e);
  process.exit(1);
});
