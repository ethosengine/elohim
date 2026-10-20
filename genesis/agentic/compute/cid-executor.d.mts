// Types for cid-executor.mjs: the naming half of the compute-executor, for TypeScript requesters.
export declare function artifactDescriptor(
  bytes: Uint8Array,
): Promise<{ cid: string; sha256: string; bytes: number }>;
export declare function jsonCid(value: unknown): Promise<string>;
export declare const REFUSAL: string;
/** Same stdout contract as `execute(executor, args)` in common.mjs; refuses every verb but
 * `artifact` and `cid`. */
export declare function execute(program: string, args: string[]): Promise<string>;
