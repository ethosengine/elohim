# elohim-epr

Canonical codec for the Elohim EPR (EntityPortalReference) atom defined in
`genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`.

## What this crate provides

- **Canonical CBOR** (dag-cbor, RFC 8949 §4.2.1 deterministic encoding)
- **CIDv1** derivation (codec 0x71, multihash sha2-256)
- **Ed25519** signing and verification
- **Envelope** struct (kind, schemaRef, schemaKey, reach, coupling, claims, supersedes, proof)
- **Epr** = Envelope + payload bytes, with builder + sign + verify
- **Structural validator** — coupling requirement enforcement per EprKind

## What this crate does NOT provide (Phase 2+)

- Persistence / storage
- Payload schema validation (requires Manifest resolver)
- GraphQL surface
- Subscriptions or federation

## Cross-language interop

The companion TypeScript package `@elohim/epr` (at `elohim/sdk/epr-ts/`) is a parallel
implementation verified against shared test vectors at `elohim/epr/tests/vectors/`.
Regenerate vectors with:

    cargo run -p elohim-epr --example gen_vectors

Regenerating after any logic change catches Rust↔TS drift immediately; both sides'
CI run every vector through full verify.

## Usage (Rust)

    use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
    use chrono::Utc;

    let kp = AgentKeypair::from_secret(&[42u8; 32]).unwrap();
    let agent_cid = compute_cid(&[100]);
    let manifest_cid = compute_cid(&[1]);

    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(manifest_cid)
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(compute_cid(&[2])),
            value: Some(compute_cid(&[3])),
            governance: Some(compute_cid(&[4])),
        })
        .issued_at(Utc::now())
        .payload(b"hello world".to_vec())
        .sign(&kp, agent_cid)
        .unwrap();

    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_ok());
    elohim_epr::validate_coupling(&epr.envelope).unwrap();

## Usage (TypeScript)

    import { verifyEpr } from '@elohim/epr';

    const result = await verifyEpr(epr, publicKey);
    if (!result.ok) throw new Error(result.error.message);
