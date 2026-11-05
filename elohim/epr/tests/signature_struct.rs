//! The signature envelope: scheme agility, the signed transcript, multikey canonical form, and
//! one wire shape in CBOR and JSON.

use cid::Cid;
use elohim_epr::proof::AgentKeypair;
use elohim_epr::signature::{
    signing_transcript, Multikey, MultikeyError, Signature, SignatureCheck, SignatureEnvelope,
    SignatureScheme, TranscriptError, ED25519_PUB_MULTICODEC,
};

const DOMAIN: &[u8] = b"elohim:test:v0:";

fn test_cid(b: u8) -> Cid {
    elohim_epr::cid::compute_cid(&[b])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The envelope vector, derived outside this crate with @noble/ed25519 over the transcript-v1
/// bytes `000f ‖ "elohim:test:v0:" ‖ 0000 ‖ 0000 ‖ 0022 ‖ ed01 ‖ pk ‖ "abc"`, framed by hand;
/// the multikey text with a hand-written base58btc encoder.
const VECTOR_PUBLIC_KEY: &str = "197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61";
const VECTOR_MULTIKEY: &str = "z6MkgAnvkP45uNxwCKeNdt6wrYkEjpYX4f7Nrd8MQqFL8Fbn";
const VECTOR_TRANSCRIPT: &str = "000f656c6f68696d3a746573743a76303a000000000022ed01197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61616263";
const VECTOR_SIG: &str = "e70b241c0e9a88696849cc40901f4856da3fab9909dda129358f56e0d15a27947fef04767fa469767eff6d2c24117f1ba691a3cc1edf761a5d711c0a582af90e";

fn vector_key() -> AgentKeypair {
    AgentKeypair::from_secret(&[42u8; 32]).unwrap()
}

#[test]
fn envelope_vector() {
    let kp = vector_key();
    assert_eq!(hex(&kp.public_key_bytes()), VECTOR_PUBLIC_KEY);
    let key = Multikey::ed25519(&kp.public_key_bytes());
    assert_eq!(key.to_string(), VECTOR_MULTIKEY);
    assert_eq!(Multikey::parse(VECTOR_MULTIKEY).unwrap(), key);

    let env = SignatureEnvelope::sign_ed25519(&kp, DOMAIN, 0, b"abc");
    assert_eq!(env.scheme, 0);
    assert_eq!(env.key, key.as_bytes());
    assert_eq!(
        hex(&env.transcript(DOMAIN, 0, b"abc").unwrap()),
        VECTOR_TRANSCRIPT
    );
    assert_eq!(
        hex(&signing_transcript(DOMAIN, 0, 0, key.as_bytes(), b"abc").unwrap()),
        VECTOR_TRANSCRIPT
    );
    assert_eq!(hex(&env.sig), VECTOR_SIG);
    assert_eq!(env.check(DOMAIN, 0, b"abc"), SignatureCheck::Verified);
    assert_eq!(
        env.check_for(&key, DOMAIN, 0, b"abc"),
        SignatureCheck::Verified
    );
}

#[test]
fn envelope_unknown_scheme_is_held_not_invalid() {
    let kp = vector_key();
    let mut env = SignatureEnvelope::sign_ed25519(&kp, DOMAIN, 0, b"abc");
    env.scheme = 7;
    assert_eq!(
        env.check(DOMAIN, 0, b"abc"),
        SignatureCheck::Held { scheme: 7 }
    );
    // Held whatever the key and signature bytes hold, and whoever the caller expects.
    env.key = vec![0x80, 0x24, 1, 2, 3];
    env.sig = vec![0u8; 3];
    assert_eq!(
        env.check(DOMAIN, 0, b"abc"),
        SignatureCheck::Held { scheme: 7 }
    );
    let expected = Multikey::ed25519(&kp.public_key_bytes());
    assert_eq!(
        env.check_for(&expected, DOMAIN, 0, b"abc"),
        SignatureCheck::Held { scheme: 7 }
    );
    assert_eq!(SignatureScheme::from_code(7), None);
    assert_eq!(
        SignatureScheme::from_code(0),
        Some(SignatureScheme::Ed25519)
    );
}

#[test]
fn envelope_scheme0_rejects_non_ed25519_key_codec() {
    let kp = vector_key();
    let good = SignatureEnvelope::sign_ed25519(&kp, DOMAIN, 0, b"abc");
    // secp256k1-pub (0xe7) codec in front of the same key bytes: scheme 0 is Invalid, not Held.
    let mut other_codec = good.clone();
    other_codec.key[0] = 0xe7;
    assert_eq!(
        other_codec.check(DOMAIN, 0, b"abc"),
        SignatureCheck::Invalid
    );
    // A raw 32-byte key with no codec at all.
    let mut bare = good.clone();
    bare.key = kp.public_key_bytes().to_vec();
    assert_eq!(bare.check(DOMAIN, 0, b"abc"), SignatureCheck::Invalid);
    // A trailing byte after the key.
    let mut long = good.clone();
    long.key.push(0);
    assert_eq!(long.check(DOMAIN, 0, b"abc"), SignatureCheck::Invalid);
    // A short signature.
    let mut short = good.clone();
    short.sig.pop();
    assert_eq!(short.check(DOMAIN, 0, b"abc"), SignatureCheck::Invalid);
    // Another signer's key with this signature, and this key with another expected signer.
    let other = AgentKeypair::from_secret(&[43u8; 32]).unwrap();
    let mut swapped = good.clone();
    swapped.key = Multikey::ed25519(&other.public_key_bytes())
        .as_bytes()
        .to_vec();
    assert_eq!(swapped.check(DOMAIN, 0, b"abc"), SignatureCheck::Invalid);
    assert_eq!(
        good.check_for(
            &Multikey::ed25519(&other.public_key_bytes()),
            DOMAIN,
            0,
            b"abc"
        ),
        SignatureCheck::Invalid
    );
}

#[test]
fn transcript_binds_epoch_and_scheme() {
    let kp = vector_key();
    let env = SignatureEnvelope::sign_ed25519(&kp, DOMAIN, 0, b"abc");
    assert_eq!(env.check(DOMAIN, 0, b"abc"), SignatureCheck::Verified);
    assert_eq!(
        env.check(DOMAIN, 1, b"abc"),
        SignatureCheck::Invalid,
        "epoch"
    );
    assert_eq!(
        env.check(b"elohim:other:v0:", 0, b"abc"),
        SignatureCheck::Invalid,
        "domain"
    );
    assert_eq!(
        env.check(DOMAIN, 0, b"abd"),
        SignatureCheck::Invalid,
        "body"
    );
    // The scheme code is inside the transcript: the same bytes signed as if under another
    // scheme number do not verify as scheme 0.
    let foreign = signing_transcript(DOMAIN, 0, 1, &env.key, b"abc").unwrap();
    let sig = elohim_epr::proof::sign(&kp, &foreign);
    let relabelled = SignatureEnvelope {
        scheme: 0,
        key: env.key.clone(),
        sig,
    };
    assert_eq!(relabelled.check(DOMAIN, 0, b"abc"), SignatureCheck::Invalid);
    // And a bare ed25519 signature over the body alone (the pre-envelope proof) never passes.
    let bare = SignatureEnvelope {
        scheme: 0,
        key: env.key.clone(),
        sig: elohim_epr::proof::sign(&kp, b"abc"),
    };
    assert_eq!(bare.check(DOMAIN, 0, b"abc"), SignatureCheck::Invalid);
}

/// Transcript v1 is injective. The review's pair: with `K` a valid Ed25519 multikey,
/// `P = 00000000 ‖ K`, `D = "elohim:test:v0:"` and `B = "abc"`, the unframed rule mapped
/// `(D, K, ":" ‖ P ‖ B)` and `(D ‖ P ‖ ":", K, B)` to the same bytes at epoch/scheme 0.
#[test]
fn transcript_collision_regression_domain_and_body() {
    let k = Multikey::ed25519(&vector_key().public_key_bytes());
    let mut p = vec![0u8, 0, 0, 0];
    p.extend_from_slice(k.as_bytes());
    let mut body_a = b":".to_vec();
    body_a.extend_from_slice(&p);
    body_a.extend_from_slice(b"abc");
    let mut domain_b = DOMAIN.to_vec();
    domain_b.extend_from_slice(&p);
    domain_b.extend_from_slice(b":");

    // The old framing collided — the regression this test pins.
    let unframed =
        |d: &[u8], key: &[u8], body: &[u8]| -> Vec<u8> { [d, &[0, 0, 0, 0], key, body].concat() };
    assert_eq!(
        unframed(DOMAIN, k.as_bytes(), &body_a),
        unframed(&domain_b, k.as_bytes(), b"abc")
    );
    let a = signing_transcript(DOMAIN, 0, 0, k.as_bytes(), &body_a).unwrap();
    let b = signing_transcript(&domain_b, 0, 0, k.as_bytes(), b"abc").unwrap();
    assert_ne!(a, b);

    // And a signature over one never verifies as the other.
    let kp = vector_key();
    let env = SignatureEnvelope::sign_ed25519(&kp, DOMAIN, 0, &body_a);
    assert_eq!(env.check(DOMAIN, 0, &body_a), SignatureCheck::Verified);
    assert_eq!(env.check(&domain_b, 0, b"abc"), SignatureCheck::Invalid);
}

/// Unrestricted key lengths let `(D, K ‖ x, B)` and `(D, K, x ‖ B)` share bytes under the unframed
/// rule; the key's length prefix separates them.
#[test]
fn transcript_collision_regression_key_length() {
    let k = Multikey::ed25519(&vector_key().public_key_bytes());
    let mut kx = k.as_bytes().to_vec();
    kx.push(b'x');
    let a = signing_transcript(DOMAIN, 0, 0, &kx, b"abc").unwrap();
    let b = signing_transcript(DOMAIN, 0, 0, k.as_bytes(), b"xabc").unwrap();
    assert_ne!(a, b);
    // Every field boundary moves the bytes: the same concatenation split differently differs.
    let c = signing_transcript(b"elohim:test:v0", 0, 0, k.as_bytes(), b":abc").unwrap();
    let d = signing_transcript(DOMAIN, 0, 0, k.as_bytes(), b"abc").unwrap();
    assert_ne!(c, d);
}

#[test]
fn transcript_frames_lengths_and_refuses_overlong_fields() {
    let t = signing_transcript(b"d", 0x0102, 0x0304, &[9, 9], b"zz").unwrap();
    assert_eq!(t, [0, 1, b'd', 1, 2, 3, 4, 0, 2, 9, 9, b'z', b'z']);
    let max = vec![0u8; usize::from(u16::MAX)];
    assert!(signing_transcript(&max, 0, 0, &max, b"").is_ok());
    let over = vec![0u8; usize::from(u16::MAX) + 1];
    assert_eq!(
        signing_transcript(&over, 0, 0, b"k", b""),
        Err(TranscriptError::DomainTooLong { len: over.len() })
    );
    assert_eq!(
        signing_transcript(DOMAIN, 0, 0, &over, b""),
        Err(TranscriptError::KeyTooLong { len: over.len() })
    );
    // An envelope whose key cannot be framed is Invalid under scheme 0 and still Held under an
    // unknown scheme; it never panics.
    let mut env = SignatureEnvelope::sign_ed25519(&vector_key(), DOMAIN, 0, b"abc");
    env.key = over.clone();
    assert!(env.transcript(DOMAIN, 0, b"abc").is_err());
    assert_eq!(env.check(DOMAIN, 0, b"abc"), SignatureCheck::Invalid);
    env.scheme = 3;
    assert_eq!(
        env.check(DOMAIN, 0, b"abc"),
        SignatureCheck::Held { scheme: 3 }
    );
}

#[test]
fn multikey_equality_is_by_decoded_bytes_minimal_varint() {
    let pk = [5u8; 32];
    let a = Multikey::ed25519(&pk);
    let mut bytes = vec![0xed, 0x01];
    bytes.extend_from_slice(&pk);
    let b = Multikey::from_bytes(bytes).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.codec(), ED25519_PUB_MULTICODEC);
    assert_eq!(a.key_bytes(), &pk);
    assert_eq!(a.ed25519_key(), Some(pk));
    // `ed 81 00` decodes to the same codec 0xed but is not minimal: refused, so one key can
    // never have two spellings.
    let mut padded = vec![0xed, 0x81, 0x00];
    padded.extend_from_slice(&pk);
    assert_eq!(
        Multikey::from_bytes(padded),
        Err(MultikeyError::NonMinimalVarint)
    );
    // A different key, or the same key under another codec, is a different multikey.
    assert_ne!(a, Multikey::ed25519(&[6u8; 32]));
    let mut other_codec = vec![0xe7, 0x01];
    other_codec.extend_from_slice(&pk);
    let c = Multikey::from_bytes(other_codec).unwrap();
    assert_ne!(a, c);
    assert_eq!(c.ed25519_key(), None);
    assert_eq!(Multikey::from_bytes(vec![]), Err(MultikeyError::Empty));
    assert_eq!(
        Multikey::from_bytes(vec![0xed, 0x01]),
        Err(MultikeyError::MissingKey)
    );
    assert_eq!(
        Multikey::from_bytes(vec![0xff; 12]),
        Err(MultikeyError::Varint)
    );
    assert!(Multikey::parse("6MkgAnvk").is_err(), "no multibase prefix");
}

#[test]
fn signature_wire_shape_roundtrips_cbor_and_json() {
    let kp = vector_key();
    let sig = Signature::new(
        test_cid(99),
        SignatureEnvelope::sign_ed25519(&kp, DOMAIN, 0, b"abc"),
    );

    let json = serde_json::to_value(&sig).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["envelope", "signer"]);
    let mut env_keys: Vec<&str> = json["envelope"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    env_keys.sort_unstable();
    assert_eq!(env_keys, ["key", "scheme", "sig"]);
    assert_eq!(json["envelope"]["scheme"], 0);
    assert_eq!(json["envelope"]["key"].as_array().unwrap().len(), 34);
    assert_eq!(json["envelope"]["sig"].as_array().unwrap().len(), 64);
    assert_eq!(serde_json::from_value::<Signature>(json).unwrap(), sig);

    let cbor = serde_ipld_dagcbor::to_vec(&sig).unwrap();
    assert_eq!(
        serde_ipld_dagcbor::from_slice::<Signature>(&cbor).unwrap(),
        sig
    );
    // key and sig are CBOR byte strings (major type 2, one-byte length), not integer arrays.
    let has = |needle: &[u8]| cbor.windows(needle.len()).any(|w| w == needle);
    let mut key_head = vec![0x58, 34];
    key_head.extend_from_slice(&sig.envelope.key[..4]);
    assert!(has(&key_head), "key is a 34-byte byte string");
    let mut sig_head = vec![0x58, 64];
    sig_head.extend_from_slice(&sig.envelope.sig[..4]);
    assert!(has(&sig_head), "sig is a 64-byte byte string");
}

#[test]
fn signed_epr_verifies_and_held_scheme_is_typed() {
    use chrono::{TimeZone, Utc};
    use elohim_epr::{Coupling, Epr, EprError, EprKind, Reach};
    let kp = vector_key();
    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(test_cid(1))
        .schema_key("k")
        .reach(Reach::Commons)
        .coupling(Coupling::default())
        .issued_at(Utc.with_ymd_and_hms(2026, 11, 5, 0, 0, 0).unwrap())
        .payload(b"abc".to_vec())
        .sign(&kp, test_cid(100))
        .unwrap();
    epr.verify_with_key(&kp.public_key_bytes()).unwrap();
    let other = AgentKeypair::from_secret(&[43u8; 32]).unwrap();
    assert!(matches!(
        epr.verify_with_key(&other.public_key_bytes()),
        Err(EprError::Signature(_))
    ));
    let mut held = epr.clone();
    held.envelope.proof.envelope.scheme = 9;
    assert!(matches!(
        held.verify_with_key(&kp.public_key_bytes()),
        Err(EprError::SignatureHeld(9))
    ));
}
