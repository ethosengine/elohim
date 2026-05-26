/**
 * IIntegrityAnchor — A single Holochain zome call that provides
 * cryptographic proof of network agreement.
 *
 * P-migrated from @app/elohim/integrity/integrity-anchor.interface (Slice 2.1c).
 * Moved here alongside BlobMetadataAnchor.
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
