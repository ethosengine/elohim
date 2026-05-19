#!/usr/bin/env node
/**
 * Author the in-kind hosting REA Commitment for the elohim-host-landing
 * ContentNode.
 *
 *   provider  = matthew  (compute / projection / DNS)
 *   receiver  = matthew  (steward of elohim-host-landing)
 *   action    = "deliver-service"
 *   signal_kind     (via metadata)  = "compute-allocation"
 *   trigger_kind    (via metadata)  = "subscription"
 *   inScopeOf = ["host:alpha.elohim.host", "epr_root:elohim-host-landing"]
 *
 * Idempotent: lists existing commitments for matthew and skips if an
 * equivalent in-scope subscription already exists.
 *
 * Usage:
 *   STORAGE_URL=http://localhost:8090 node genesis/scripts/author-landing-commitment.mjs
 */

const STORAGE_URL = process.env.STORAGE_URL ?? 'http://localhost:8090';
const REQUESTER = process.env.MATTHEW_ID ?? 'matthew';

const SCOPE = ['host:alpha.elohim.host', 'epr_root:elohim-host-landing'];

const COMMITMENT = {
  action: 'deliver-service',
  provider: REQUESTER,
  receiver: REQUESTER,
  resourceClassifiedAs: ['http-requests', 'egress-bandwidth', 'ssr-cpu-seconds'],
  resourceQuantity: { hasNumericalValue: 0, hasUnit: 'subscription-window' },
  hasBeginning: new Date().toISOString(),
  inScopeOf: SCOPE,
  note:
    'In-kind self-hosting agreement: Matthew (steward) hosts the ' +
    'elohim-host-landing ContentNode at alpha.elohim.host root via Matthew ' +
    '(doorway operator). signal_kind="compute-allocation"; ' +
    'trigger_kind="subscription".',
  metadata: {
    signalKind: 'compute-allocation',
    triggerKind: 'subscription',
    inKind: true,
  },
};

async function listExisting() {
  const url = new URL(`${STORAGE_URL}/api/v1/commitments`);
  url.searchParams.set('provider', REQUESTER);
  url.searchParams.set('receiver', REQUESTER);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`list failed: ${res.status} ${await res.text()}`);
  return res.json();
}

function isScopeEqual(a = [], b = []) {
  if (a.length !== b.length) return false;
  const aSet = new Set(a);
  for (const item of b) if (!aSet.has(item)) return false;
  return true;
}

async function main() {
  const existing = await listExisting();
  const match = existing.find(
    c =>
      c.action === 'deliver-service' &&
      isScopeEqual(c.inScopeOf ?? [], SCOPE) &&
      c.state !== 'cancelled' &&
      c.state !== 'breached',
  );
  if (match) {
    console.log(`[author-commitment] already exists: ${match.id} (state=${match.state})`);
    return;
  }

  const res = await fetch(`${STORAGE_URL}/api/v1/commitments`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(COMMITMENT),
  });
  if (!res.ok) {
    throw new Error(`create failed: ${res.status} ${await res.text()}`);
  }
  const created = await res.json();
  console.log(`[author-commitment] created: ${created.id}`);
  console.log(JSON.stringify(created, null, 2));
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
