/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: p2p/epr-atom-message.schema.json -- DO NOT EDIT */

/**
 * Transient wire contract for /elohim/epr-atom/1.0.0 request-response protocol. CBOR-encoded on the wire. This is a projection-free wire-only contract — the notarized source of truth remains the ed25519-signed CBOR Envelope (content-addressed by CID).
 */
export type EprAtomMessage = EprAtomRequest | EprAtomResponse;
export type EprAtomRequest =
  | {
      tag: 'fetch';
      cid: string;
    }
  | {
      tag: 'announce';
      envelope_bytes: string;
    }
  | {
      tag: 'fetch_batch';
      /**
       * @minItems 1
       * @maxItems 128
       */
      cids: [string, ...string[]];
    }
  | {
      tag: 'integrity_notify';
      /**
       * Integrity event discriminator: KeyRevocation | KeyRotation | RevocationAttestation | AgentPeerBinding
       */
      kind: string;
      /**
       * Encoded integrity payload (MessagePack for KeyRevocation, CBOR for others)
       */
      payload_bytes: string;
    };
export type EprAtomResponse =
  | {
      tag: 'atom';
      envelope_bytes: string;
    }
  | {
      tag: 'atom_batch';
      atoms: (null | string)[];
    }
  | {
      tag: 'announced';
      accepted: boolean;
      reason?: string | null;
    }
  | {
      tag: 'not_found';
    }
  | {
      tag: 'error';
      message: string;
    }
  | {
      tag: 'integrity_ack';
      /**
       * True if payload was successfully decoded and routed to the integrity handler
       */
      received: boolean;
      /**
       * Non-secret diagnostic string when received is false
       */
      reason?: string | null;
    };
