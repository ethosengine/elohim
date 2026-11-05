//! `ByteCid::parse` is the byte surface's admission of untrusted text: it must return an error,
//! never panic, on every input, in debug and release builds alike (the gate runs this file in
//! both). A deterministic xorshift64* stream drives four generators of 59-character inputs —
//! base32-lower bodies, encodings of random 36-byte headers drawn from the interesting varint
//! bytes, arbitrary ASCII and multi-byte text, and single-character mutations of a valid CID —
//! and any input that panics is named in the failure.

use std::panic;

use elohim_address::{compute_bytes_cid, ByteCid, BYTE_CID_LEN, BYTE_CID_PREFIX};

const ROUNDS: usize = 50_000;
const BASE32_LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
/// Header bytes that reach the varint, version, codec, multihash and length checks.
const HEADER_BYTES: &[u8] = &[
    0x00, 0x01, 0x12, 0x1e, 0x20, 0x21, 0x55, 0x70, 0x71, 0x7f, 0x80, 0x81, 0x95, 0x9e, 0xa0, 0xd5,
    0xff,
];

struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.below(from.len())]
    }
}

fn base32_lower(bytes: &[u8]) -> String {
    let mut spec = data_encoding::Specification::new();
    spec.symbols.push_str("abcdefghijklmnopqrstuvwxyz234567");
    let mut text = String::from("b");
    text.push_str(&spec.encoding().unwrap().encode(bytes));
    text
}

/// Parse `text` both ways; a panic fails naming the input, an admission must round-trip.
fn parse_without_panic(text: &str) {
    let parsed = panic::catch_unwind(|| (ByteCid::parse(text), text.parse::<ByteCid>()))
        .unwrap_or_else(|_| panic!("ByteCid::parse panicked on {text:?}"));
    match parsed {
        (Ok(cid), Ok(again)) => {
            assert_eq!(cid, again, "{text:?}");
            assert_eq!(cid.to_string(), text, "an admitted address is canonical");
            assert!(text.starts_with(BYTE_CID_PREFIX));
        }
        (Err(a), Err(b)) => assert_eq!(a, b, "{text:?}: FromStr is the same parser"),
        (a, b) => panic!("{text:?}: parse {a:?} but FromStr {b:?}"),
    }
}

#[test]
fn random_base32_bodies_never_panic() {
    let mut rng = Xorshift(0x454c_4f48_494d);
    for _ in 0..ROUNDS {
        let mut text = String::from("b");
        while text.len() < BYTE_CID_LEN {
            text.push(char::from(rng.pick(BASE32_LOWER)));
        }
        parse_without_panic(&text);
    }
}

#[test]
fn random_headers_over_36_bytes_never_panic() {
    let mut rng = Xorshift(0x0061_6464_7265_7373);
    for _ in 0..ROUNDS {
        let mut bytes = [0u8; 36];
        for b in bytes.iter_mut() {
            *b = rng.next() as u8;
        }
        // The first ten bytes are drawn mostly from the varint-interesting set, so the
        // version, codec, multihash and length checks — and their non-minimal spellings — are
        // all reached.
        for b in bytes.iter_mut().take(10) {
            if rng.below(4) != 0 {
                *b = rng.pick(HEADER_BYTES);
            }
        }
        let text = base32_lower(&bytes);
        assert_eq!(text.len(), BYTE_CID_LEN);
        parse_without_panic(&text);
    }
}

#[test]
fn random_text_never_panics() {
    let mut rng = Xorshift(0x7465_7874);
    let odd = [
        'é', 'ß', '€', '😀', '\u{0}', '\u{7f}', '=', 'B', 'z', '0', '1', '8', '9',
    ];
    for round in 0..ROUNDS {
        let mut text = String::new();
        // Half of the inputs are exactly 59 bytes of ASCII; half are 59 characters that may be
        // multi-byte, so the byte-length and alphabet checks both see them.
        if round % 2 == 0 {
            while text.len() < BYTE_CID_LEN {
                text.push(char::from(32 + rng.below(95) as u8));
            }
        } else {
            for _ in 0..BYTE_CID_LEN {
                if rng.below(3) == 0 {
                    text.push(rng.pick(&odd));
                } else {
                    text.push(char::from(rng.pick(BASE32_LOWER)));
                }
            }
        }
        parse_without_panic(&text);
    }
}

#[test]
fn single_character_mutations_never_panic() {
    let mut rng = Xorshift(0x6d75_7461_7465);
    let valid = compute_bytes_cid(b"hello world").to_string();
    for _ in 0..ROUNDS {
        let mut chars: Vec<char> = valid.chars().collect();
        let at = rng.below(chars.len());
        chars[at] = if rng.below(8) == 0 {
            char::from(32 + rng.below(95) as u8)
        } else {
            char::from(rng.pick(BASE32_LOWER))
        };
        let text: String = chars.into_iter().collect();
        parse_without_panic(&text);
    }
    parse_without_panic(&valid);
}
