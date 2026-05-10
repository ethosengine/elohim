/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: inputs/create-token-transfer-input.schema.json -- DO NOT EDIT */

/**
 * Input for creating a token transfer between agents.
 */
export interface CreateTokenTransferInput {
  fromAgent: string;
  toAgent: string;
  amount: number;
  governanceLayer?: string;
  note?: string;
}
