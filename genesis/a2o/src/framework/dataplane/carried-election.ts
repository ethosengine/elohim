/**
 * Carried-election staging rails — the household-mesh helpers that stage a
 * DIVERGENCE between two peers' declared heads for one EPR, and read back what
 * each peer serves.
 *
 * ## Why this module exists
 *
 * These four helpers (`connectConductor`, `authorDeclare`, `servedHead`, and
 * the write ledger below) were written on 2026-08-31 inside
 * `scripts/carried-election-mesh-proof.ts` — the one-shot script that first
 * PROVED carry-the-election on the 3-peer household mesh (two peers holding
 * divergent DECLARED heads for one id, an EARNED canonical on one, the
 * declaring peer's conductor serving the winning declaration LINK's signed
 * Record, the OTHER peer's conductor re-deriving it in wasm, a tampered record
 * refused, and the disagreeing peer's sweep converging its served head).
 *
 * `steps/dataplane/federation-deploy.steps.ts` binds the federation-deploy
 * feature's final scenario to that same proven path. It needs the SAME staging
 * — not a second, subtly-different one — so the helpers moved HERE and both
 * call sites import them. Copying them would have created two fixtures that
 * drift, and the a2o suite has already paid for that shape once (see the
 * "Reuse over reinvention" note in `steps/dataplane/resiliency-saga.steps.ts`).
 *
 * ## The write ledger — a STRUCTURAL organic-path witness
 *
 * The scenario's whole claim is that convergence happens ORGANICALLY: the
 * disagreeing peer's own reconcile sweep moves its row, with no declaration
 * call, no per-host upload, and no doorway credential from the fixture. A
 * fixture that merely *promises* not to write is not evidence. So every
 * mutating call in this module — and only this module's mutating calls stage
 * anything — is funnelled through {@link recordStagingWrite}, which appends to
 * an append-only ledger. The steps snapshot the ledger's length when the WHEN
 * phase opens and assert it has not grown when the THEN phase asserts
 * convergence. The receipt carries the ledger verbatim, so a reader can see
 * exactly which writes the fixture made and when it stopped making them.
 *
 * ## Port scheme
 *
 * The local household mesh (`app/elohim-app/scripts/hc-mesh.sh`) derives ports
 * from the peer index: `admin_port(i) = 4444 + 10i`, `app_port(i) = 4445 + 10i`,
 * `http_port(i) = 8090 + i`, with `i=0` matthew, `1` jessica, `2` james — the
 * same convention `scripts/release-ceremony.ts`'s `--conductors` CSV default
 * carries. {@link meshConductorPorts} is the one derivation both this module's
 * callers use, so a mesh-layout change lands in one place.
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64, type CellId } from '@holochain/client';

/** The app id every household-mesh conductor installs. */
export const MESH_APP_ID = 'elohim';

/** The role whose cell carries the `content_store` coordinator zome. */
export const CONTENT_STORE_ROLE = 'lamad';

/** The coordinator zome the carried-election externs live in. */
export const CONTENT_STORE_ZOME = 'content_store';

/** Household-mesh peer order — the index that drives every port derivation. */
export const MESH_PEER_ORDER = ['matthew', 'jessica', 'james'] as const;
export type MeshPeerName = (typeof MESH_PEER_ORDER)[number];

/**
 * Admin/app websocket ports for a household-mesh peer index.
 * See the module doc's "Port scheme" section for the derivation's source.
 */
export function meshConductorPorts(peerIndex: number): { adminPort: number; appPort: number } {
  return { adminPort: 4444 + 10 * peerIndex, appPort: 4445 + 10 * peerIndex };
}

/** Storage HTTP base URL for a household-mesh peer index (`http_port(i) = 8090 + i`). */
export function meshStorageUrl(peerIndex: number): string {
  return `http://localhost:${8090 + peerIndex}`;
}

// ---------------------------------------------------------------------------
// The staging write ledger
// ---------------------------------------------------------------------------

/** One mutating call the fixture made while staging. */
export interface StagingWrite {
  /** HTTP method, or `zome` for a conductor call that authors/declares. */
  method: string;
  /** The URL or `<zome>.<fn>` target. */
  target: string;
  /** Wall-clock ISO stamp, so a receipt reader can order writes against the WHEN phase. */
  at: string;
}

const stagingWriteLedger: StagingWrite[] = [];

/**
 * Append one mutating call to the append-only staging ledger.
 * Every write helper in this module calls this; nothing else may.
 */
export function recordStagingWrite(method: string, target: string): void {
  stagingWriteLedger.push({ method, target, at: new Date().toISOString() });
}

/** The ledger so far, as a copy — callers must never mutate the real one. */
export function stagingWrites(): readonly StagingWrite[] {
  return [...stagingWriteLedger];
}

/** How many mutating calls the fixture has made. The organic-path watermark. */
export function stagingWriteCount(): number {
  return stagingWriteLedger.length;
}

/** Drop the ledger — used between scenarios so one world's writes are not another's. */
export function resetStagingWrites(): void {
  stagingWriteLedger.length = 0;
}

// ---------------------------------------------------------------------------
// Conductor rail
// ---------------------------------------------------------------------------

/**
 * A connected conductor: the `content_store` call rail plus the agent key the
 * cell is provisioned for. Shape-compatible with
 * `steps/delivery/lineage-commitments.ts`'s `ConductorRail`.
 */
export interface CarriedElectionRail {
  /** Call a `content_store` coordinator function on this conductor. */
  call: (fnName: string, payload: unknown) => Promise<unknown>;
  /** This cell's agent public key, base64 — the `X-Agent-Cid` a declaration presents. */
  agent: string;
  /** Close both websockets. Always call it; an open AppWebsocket keeps node alive. */
  close: () => Promise<void>;
}

/**
 * Connect to one household-mesh conductor and return its `content_store` rail.
 *
 * Lifted verbatim (behaviour-for-behaviour) from
 * `scripts/carried-election-mesh-proof.ts`'s local `conductor()`: connect admin,
 * find the `elohim` app (falling back to the first installed app), take the
 * `lamad` role's provisioned cell, authorize signing credentials for it, then
 * open an app websocket with an app-authentication token.
 *
 * @throws if no app is installed, or the app has no provisioned `lamad` cell —
 * both of which mean the mesh is not up in the shape this fixture needs, and
 * are far more legible as a thrown message than as a later undefined-cell TypeError.
 */
export async function connectConductor(
  adminPort: number,
  appPort: number
): Promise<CarriedElectionRail> {
  const wsClientOptions = { origin: MESH_APP_ID };
  const admin = await AdminWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${adminPort}`),
    wsClientOptions,
  });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === MESH_APP_ID) ?? apps[0];
  if (!app) {
    throw new Error(
      `no installed app on conductor admin port ${adminPort} — is the household mesh up (just mesh start)?`
    );
  }
  const cells = new Map<string, CellId>();
  for (const [role, infos] of Object.entries(app.cell_info)) {
    for (const info of infos as { type?: string; value?: { cell_id?: CellId } }[]) {
      if (info.type === 'provisioned' && info.value?.cell_id) cells.set(role, info.value.cell_id);
    }
  }
  const cell = cells.get(CONTENT_STORE_ROLE);
  if (!cell) {
    throw new Error(
      `app "${app.installed_app_id}" on admin port ${adminPort} has no provisioned "${CONTENT_STORE_ROLE}" cell ` +
        `(roles seen: ${[...cells.keys()].join(', ') || 'none'})`
    );
  }
  await admin.authorizeSigningCredentials(cell);
  const appWs = await AppWebsocket.connect({
    url: new URL(`ws://127.0.0.1:${appPort}`),
    token: (await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id }))
      .token,
    wsClientOptions,
  });
  return {
    call: async (fnName: string, payload: unknown) =>
      appWs.callZome({
        cell_id: cell,
        zome_name: CONTENT_STORE_ZOME,
        fn_name: fnName,
        payload,
      }),
    agent: encodeHashToBase64(cell[1]),
    close: async () => {
      await closeTransport(appWs.client);
      await closeTransport(admin.client);
    },
  };
}

/**
 * Best-effort transport close.
 *
 * `@holochain/client` 0.20's `AppClientTransport` (and the admin client's
 * transport) do NOT declare `close()` on their public interface, even though the
 * websocket implementations behind them have one — the original mesh proof
 * side-stepped this entirely by ending in `process.exit`. A cucumber world
 * cannot: a left-open websocket keeps the node process alive past the run. So
 * the closer is duck-typed and never throws — a transport that cannot be closed
 * is a cleanup nuisance, never a reason to fail a scenario that already passed.
 */
async function closeTransport(transport: unknown): Promise<void> {
  const closer = (transport as { close?: () => unknown }).close;
  if (typeof closer !== 'function') return;
  try {
    await closer.call(transport);
  } catch {
    // Already closed, or a transport without a real socket. Nothing to do.
  }
}

// ---------------------------------------------------------------------------
// Staging: author a revision and declare it as this peer's head
// ---------------------------------------------------------------------------

export interface AuthorDeclareOptions {
  /** Storage base URL for the peer that authors (e.g. `http://localhost:8090`). */
  storageUrl: string;
  /** The EPR id both peers will disagree about. */
  id: string;
  /** The revision body this peer authors. */
  body: string;
  /** The authoring agent's base64 key — presented as `X-Agent-Cid` on the declaration. */
  agent: string;
  /** Human-readable title for the created row. */
  title?: string;
  /** Description for the created row. */
  description?: string;
}

/**
 * Author a revision of `id` on one peer and DECLARE it as that peer's head.
 *
 * Three calls, in the order the proof made them:
 *   1. `POST /db/content/bulk`  — create the row, ONLY when this peer does not
 *      already hold it (see the note below).
 *   2. `PATCH /db/content/{id}` — author the revision; the answer's
 *      `dhtAnchorHash` is the new notarized action this peer's chain now holds.
 *   3. `POST /db/content/{id}/head` — declare that action as this peer's head.
 *
 * ### Why the create is conditional, and why staging is non-destructive
 *
 * The proof staged on a fresh per-run id, so its unconditional bulk create was
 * harmless. The federation-deploy scenario stages on a REAL, seeded EPR
 * (`elohim-host-landing`) — the page a visitor actually lands on — and a bulk
 * create there would overwrite the row's title, description, contentType and
 * contentFormat with this fixture's own. So the row is probed first and created
 * only when absent; both call sites keep one code path.
 *
 * What the PATCH then does to an ALREADY-ANCHORED row is exactly what this
 * fixture wants and nothing more (traced in elohim-storage 2026-09-05):
 * `reach` being present routes the update through the conductor, which calls
 * `update_entry` and therefore mints a NEW notarized action on this peer's
 * chain every time — the divergence. `content_body` is not a field of the
 * anchored-row update input at all, so the page's BYTES are unchanged;
 * `blob_hash` is rewritten to its existing value and `server_blob_hash` is
 * never touched, so neither blob pointer moves. `reach` is read back from the
 * row and re-sent unchanged rather than forced, so the row's audience is not
 * silently widened by a test fixture.
 *
 * All three calls are STAGING writes: they exist to create the disagreement the
 * scenario then asks the substrate to heal on its own. Each is recorded in the
 * write ledger (see the module doc) so the WHEN phase can prove it added none.
 *
 * @returns the declared head action hash.
 */
export async function authorDeclare(opts: AuthorDeclareOptions): Promise<string> {
  const { storageUrl, id, body, agent } = opts;

  const existing = await fetch(`${storageUrl}/db/content/${id}`);
  const existingRow = existing.ok
    ? ((await existing.json().catch(() => null)) as { reach?: string } | null)
    : null;

  if (!existingRow) {
    recordStagingWrite('POST', `${storageUrl}/db/content/bulk`);
    const bulk = await fetch(`${storageUrl}/db/content/bulk`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify([
        {
          id,
          title: opts.title ?? `Carried election staging (${storageUrl})`,
          description: opts.description ?? 'carried-election divergence fixture',
          contentType: 'concept',
          contentFormat: 'markdown',
          content: body,
          reach: 'commons',
        },
      ]),
    });
    if (!bulk.ok) {
      throw new Error(`bulk create on ${storageUrl}: ${bulk.status} ${await bulk.text()}`);
    }
  }

  // `reach` must be PRESENT for the update to route through the conductor (and
  // so mint a new action); it is deliberately the row's own current value when
  // the row already existed.
  const reach = existingRow?.reach ?? 'commons';
  recordStagingWrite('PATCH', `${storageUrl}/db/content/${id}`);
  const patch = await fetch(`${storageUrl}/db/content/${id}`, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ contentBody: body, reach }),
  });
  const patched = (await patch.json().catch(() => ({}))) as { dhtAnchorHash?: string };
  const anchor = patched.dhtAnchorHash;
  if (!anchor) {
    throw new Error(
      `no dhtAnchorHash after PATCH on ${storageUrl}: ${JSON.stringify(patched).slice(0, 200)}`
    );
  }

  recordStagingWrite('POST', `${storageUrl}/db/content/${id}/head`);
  const head = await fetch(`${storageUrl}/db/content/${id}/head`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'X-Agent-Cid': agent },
    body: JSON.stringify({ headActionHash: anchor }),
  });
  if (!head.ok) {
    throw new Error(`declare on ${storageUrl}: ${head.status} ${await head.text()}`);
  }
  return anchor;
}

/**
 * READ what one peer currently serves as the head of `id`.
 * `null` when the surface answers non-2xx or carries no `headActionHash` — the
 * caller decides whether that is "not yet" or "broken". Never a staging write.
 */
export async function servedHead(storageUrl: string, id: string): Promise<string | null> {
  const r = await fetch(`${storageUrl}/db/content/${id}/head`);
  if (!r.ok) return null;
  const j = (await r.json().catch(() => null)) as { headActionHash?: string } | null;
  return j?.headActionHash ?? null;
}

// ---------------------------------------------------------------------------
// Carried-election evidence
// ---------------------------------------------------------------------------

/** What `get_canonical_election_evidence` answers on the declaring peer. */
export interface CanonicalElectionEvidence {
  /** The winning declaration LINK's own signed Record, as raw bytes. */
  link_record?: Uint8Array;
  election?: {
    winner_target?: string;
    canonical_earned?: boolean;
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

/** What `verify_carried_election` answers on the RECEIVING peer, re-derived in wasm. */
export interface VerifiedCarriedElection {
  winner_target?: string;
  canonical_earned?: boolean;
  canonical_declared_at?: unknown;
  [key: string]: unknown;
}

/**
 * Ask the declaring peer's conductor for the winning declaration link's signed
 * Record. A read, not a write — nothing is authored by asking.
 */
export async function canonicalElectionEvidence(
  rail: CarriedElectionRail,
  id: string
): Promise<CanonicalElectionEvidence> {
  return (await rail.call('get_canonical_election_evidence', id)) as CanonicalElectionEvidence;
}

/**
 * Re-derive a carried election IN WASM on the receiving peer's own conductor:
 * the link's bytes must hash to the address they claim, the author's signature
 * must verify, the link must bind to this EPR's anchor, and the tier must parse.
 * Throws (from the conductor) when any of those fail — which is exactly what the
 * scenario's anti-regression line asserts.
 */
export async function verifyCarriedElection(
  rail: CarriedElectionRail,
  id: string,
  linkRecord: Uint8Array
): Promise<VerifiedCarriedElection> {
  return (await rail.call('verify_carried_election', {
    id,
    link_record: linkRecord,
  })) as VerifiedCarriedElection;
}

/**
 * Flip one byte near the end of a signed link record.
 *
 * The proof's own tamper: `tampered[len - 5] ^= 0xff`. Kept identical so the
 * anti-regression assertion in the feature exercises the SAME refusal the
 * 2026-08-31 proof measured, not a new one whose refusal path is unmeasured.
 */
export function tamperLinkRecord(linkRecord: Uint8Array): Uint8Array {
  const tampered = new Uint8Array(linkRecord);
  tampered[tampered.length - 5] ^= 0xff;
  return tampered;
}
