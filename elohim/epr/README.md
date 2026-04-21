# elohim-epr

Canonical codec for the Elohim EPR (EntityPortalReference) atom defined in
`genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`.

Shipping wire primitives: canonical CBOR (dag-cbor / RFC 8949 §4.2.1),
CIDv1 (codec=0x71 dag-cbor, multihash=sha2-256), Ed25519 signatures.

Not a storage, resolver, or validator service — that's Phase 2+.
