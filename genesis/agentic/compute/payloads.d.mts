// Types for payloads.mjs (the parts TypeScript requesters reuse).

/** A payload transport: one chunk per call, leased to `owner` (a task CID). */
export type Payload = (
  cid: string,
  method?: "GET" | "PUT" | "DELETE",
  body?: Buffer,
  retention?: number,
  owner?: string,
) => Promise<Buffer | undefined>;

export interface ChunkDescriptor {
  cid: string;
  sha256: string;
  bytes: number;
}

export interface ArtifactDescriptor {
  cid?: string;
  sha256: string;
  bytes: number;
  chunks?: ChunkDescriptor[];
  name?: string;
}

export declare const CHUNK_BYTES: number;
export declare function leaseSeconds(
  policy: { maxAgeSeconds?: number | null; expireWhen?: string } | undefined,
): number;
export declare function materialize(
  descriptor: ArtifactDescriptor,
  path: string,
  payload: Payload,
  owner: string,
  retention?: number,
): Promise<void>;
