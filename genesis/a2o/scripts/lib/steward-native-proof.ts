/** Receiver-local reads are not DHT integration proof: correlate the returned
 * action/link hashes with the fork's op timing dumps separately. */
import { decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

import type { Conductor } from './steward-conductor.js';

export interface NativePublicationTarget {
  id: string;
  head: string;
  root: string;
  dna: string;
}
export interface NativeReceiverProof {
  agent: string;
  dna: string;
  target: NativePublicationTarget;
  deadlineMs: number;
  accepted: boolean;
  headObservedAt?: number;
  electionObservedAt?: number;
  lineageObservedAt?: number;
  acceptedAt?: number;
  electionLink?: string;
  lastError?: string;
}
interface Head {
  head_action_hash: Uint8Array;
  canonical: boolean;
  canonical_earned?: boolean;
}
interface Election {
  winner_target: string;
  canonical_earned: boolean;
  canonical_link_hash: string;
}
interface Lineage {
  referenced_action_hash: Uint8Array;
  root_action_hash: Uint8Array;
  content_id: string;
  truncated: boolean;
}
interface Clock {
  now(): number;
  sleep(ms: number): Promise<void>;
}
const clock: Clock = {
  now: Date.now,
  sleep: async ms => {
    await new Promise(resolve => setTimeout(resolve, ms));
  },
};

/** Use the SAME absolute deadline as publication and serving. Connect before
 * publication with existing credentials. No signing grants or network ancestry reads. */
export async function awaitNativeReceiver(
  receiver: Conductor,
  target: NativePublicationTarget,
  deadlineMs: number,
  timing: Clock = clock
): Promise<NativeReceiverProof> {
  const proof: NativeReceiverProof = {
    agent: receiver.agent,
    dna: receiver.dna,
    target,
    deadlineMs,
    accepted: false,
  };
  if (receiver.dna !== target.dna) throw new Error('Native receiver DNA mismatch');
  if (!Number.isFinite(deadlineMs)) throw new Error('Native receiver deadline must be finite');
  const headHash = decodeHashFromBase64(target.head);
  while (timing.now() < deadlineMs) {
    try {
      const remaining = (): number => {
        const left = deadlineMs - timing.now();
        if (left <= 0) throw new Error('Shared publication deadline elapsed');
        return Math.min(5000, left);
      };
      const head = await receiver.call<Head | null>(
        'resolve_content_head_local',
        target.id,
        remaining()
      );
      const headMatches =
        head?.canonical === true &&
        head.canonical_earned === true &&
        encodeHashToBase64(head.head_action_hash) === target.head;
      if (headMatches) proof.headObservedAt ??= timing.now();
      const election = await receiver.call<Election | null>(
        'resolve_canonical_election',
        target.id,
        remaining()
      );
      const electionMatches =
        election?.canonical_earned === true &&
        election.winner_target === target.head &&
        Boolean(election.canonical_link_hash);
      if (electionMatches) {
        proof.electionObservedAt ??= timing.now();
        proof.electionLink = election.canonical_link_hash;
      }
      const lineage = await receiver.call<Lineage>(
        'get_content_lineage',
        { action_hash: headHash, local: true },
        remaining()
      );
      const lineageMatches =
        lineage.content_id === target.id &&
        !lineage.truncated &&
        encodeHashToBase64(lineage.referenced_action_hash) === target.head &&
        encodeHashToBase64(lineage.root_action_hash) === target.root;
      if (lineageMatches) proof.lineageObservedAt ??= timing.now();
      if (headMatches && electionMatches && lineageMatches && timing.now() <= deadlineMs) {
        proof.accepted = true;
        proof.acceptedAt = timing.now();
        return proof;
      }
      proof.lastError = `receiver pending: head=${headMatches}, election=${electionMatches}, lineage=${lineageMatches}`;
    } catch (error) {
      proof.lastError = String(error);
    }
    if (timing.now() < deadlineMs) await timing.sleep(Math.min(1000, deadlineMs - timing.now()));
  }
  return proof;
}
