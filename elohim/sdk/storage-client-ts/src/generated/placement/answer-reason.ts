/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/answer-reason.schema.json -- DO NOT EDIT */

/**
 * WHY a boundary answer is not `present` — the vocabulary a client renders when it must explain a non-answer (concern class C4; the `unverifiable` arm is C5's may-believe clause: evidence that cannot be evaluated confers nothing, in either direction). Exactly one reason (`observed_absent`) accompanies answerState `absent`; every other reason accompanies `unreachable`. A `present` answer takes no reason. Rust authority: seam_contracts::AnswerReason (crates/seam-contracts/src/answer.rs); agreement is asserted by elohim/elohim-storage/tests/schema_contract.rs.
 */
export type AnswerReason =
  | 'observed_absent'
  | 'timeout'
  | 'transport_error'
  | 'refused'
  | 'unverifiable'
  | 'not_yet_delivered';
