use elohim_epr::cbor;
use ipld_core::ipld::Ipld;

#[test]
fn roundtrip_primitives() {
    for ipld in [
        Ipld::Null,
        Ipld::Bool(true),
        Ipld::Integer(42),
        Ipld::Integer(-7),
        Ipld::Float(1.5),
        Ipld::String("hello".into()),
        Ipld::Bytes(vec![1, 2, 3]),
    ] {
        let bytes = cbor::encode(&ipld).expect("encode");
        let decoded = cbor::decode(&bytes).expect("decode");
        assert_eq!(ipld, decoded);
    }
}

#[test]
fn deterministic_map_encoding() {
    // Two maps with different insertion order must encode identically
    let a: Ipld = Ipld::Map(
        [
            ("b".into(), Ipld::Integer(2)),
            ("a".into(), Ipld::Integer(1)),
        ]
        .into_iter()
        .collect(),
    );
    let b: Ipld = Ipld::Map(
        [
            ("a".into(), Ipld::Integer(1)),
            ("b".into(), Ipld::Integer(2)),
        ]
        .into_iter()
        .collect(),
    );
    let enc_a = cbor::encode(&a).unwrap();
    let enc_b = cbor::encode(&b).unwrap();
    assert_eq!(enc_a, enc_b, "canonical encoding must be order-independent");
}

#[test]
fn rejects_non_canonical_on_decode() {
    // Non-canonical CBOR: 0x18 0x01 encodes uint 1 with 1-byte length prefix,
    // but canonical form requires the shortest encoding (just 0x01).
    let non_canonical = vec![0x18u8, 0x01];
    assert!(cbor::decode_strict(&non_canonical).is_err());
}
