/**
 * IIntegrityAnchor — A single Holochain zome call that provides
 * cryptographic proof of network agreement.
 *
 * Holochain DNAs are cryptographically signed to their schema,
 * requiring parallel conductors to agree on upgrade paths.
 * Each anchor is a verification point — not a data fetch.
 *
 * The verify() method calls one zome function and returns the
 * DHT-attested result. Services wrap anchors with caching,
 * fallback, and orchestration logic.
 */
export interface IIntegrityAnchor<TInput, TOutput> {
  readonly zomeName: string;
  readonly fnName: string;

  /**
   * Verify data against the DHT's cryptographic integrity.
   * Returns the network-attested result.
   */
  verify(input: TInput): Promise<TOutput>;
}
