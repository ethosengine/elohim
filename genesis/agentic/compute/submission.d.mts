// Types for submission.mjs, so TypeScript requesters (the hosted CI runner,
// genesis/a2o/scripts/compute-runner.ts) share the one submit shape.
import type { Payload } from "./payloads.mjs";

export declare const ENVELOPE_LIMIT: number;

export interface PublishSubmissionInput {
  envelope: Record<string, unknown>;
  inputPaths: [string, string];
  file?: string;
  root: string;
  executor: string;
  payload: Payload;
  execute?: (program: string, args: string[]) => Promise<string>;
  newRun?: boolean;
}

export declare function publishSubmission(input: PublishSubmissionInput): Promise<{
  envelope: Record<string, unknown>;
  taskCid: string;
  publishedTask: string;
}>;

export declare function submitRequest(input: {
  envelope: Record<string, unknown>;
  taskCid: string;
  grantActionHash: string | undefined;
}): {
  taskCid: string;
  provider: unknown;
  grantActionHash: string | undefined;
  envelope: Record<string, unknown>;
};
