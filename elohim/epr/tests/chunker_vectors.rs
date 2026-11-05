//! The `fastcdc-2020` chunker's epoch-0 vectors, held in the shared fixture file
//! `elohim/eprfs/fixtures/tree-vectors.json` under its `chunker` section.
//!
//! Three things must agree for every fixture: the pinned `(offset, length, chunk CID)` list,
//! `profile::chunk_bytes` (the fastcdc `=5.0.0` reference), and `spec_chunks` below — an
//! independent re-implementation of the specification text in `src/profile.rs`, so the text a
//! TypeScript twin is written from is the text that is tested.
//!
//! Compare-only: `REGENERATE_VECTORS=1` may ADD a fixture that is absent; it refuses to change
//! an existing epoch-0 entry. A changed expectation is a behaviour change and needs review and,
//! if it stands, a new profile id.

// gate-reads: elohim/eprfs/fixtures/tree-vectors.json

use std::io::Read;
use std::path::PathBuf;

use elohim_epr::cid::compute_bytes_cid;
use elohim_epr::profile::{
    chunk_bytes, chunk_stream, fastcdc_2020_table_digest, Chunker, ValidatedProfile,
    BYTE_PROFILE_0, FASTCDC_2020_TABLE_DIGEST,
};
use serde_json::{json, Value};

const SEED: u64 = 0x454c_4f48_494d;
const MAIN_LEN: usize = 10_485_760;
const MIN: usize = 262_144;
const AVG: usize = 1_048_576;
const MAX: usize = 4_194_304;

/// xorshift64* (shifts 12/25/27, multiplier 0x2545F4914F6CDD1D) from `SEED`; each step's
/// 64-bit product is emitted little-endian, the stream truncated to `len`.
fn xorshift_bytes(len: usize) -> Vec<u8> {
    let mut state = SEED;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        out.extend_from_slice(&state.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes());
    }
    out.truncate(len);
    out
}

struct Fixture {
    name: &'static str,
    source: &'static str,
    len: usize,
}

const PATTERN: &[u8] = b"elohim-";

fn fixtures() -> Vec<Fixture> {
    let x = |name, len| Fixture {
        name,
        source: "xorshift64*",
        len,
    };
    vec![
        x("main-10MiB", MAIN_LEN),
        x("empty", 0),
        x("365B", 365),
        x("min-1", MIN - 1),
        x("min+1", MIN + 1),
        x("avg-1", AVG - 1),
        x("avg+1", AVG + 1),
        x("max-1", MAX - 1),
        x("max+1", MAX + 1),
        x("odd", 3_333_333),
        Fixture {
            name: "repetitive",
            source: "pattern",
            len: 5_000_000,
        },
        Fixture {
            name: "forced-max",
            source: "zeros",
            len: 9_000_000,
        },
        Fixture {
            name: "short-tail",
            source: "zeros",
            len: 2 * MAX + 100,
        },
    ]
}

fn input(f: &Fixture) -> Vec<u8> {
    match f.source {
        "xorshift64*" => xorshift_bytes(f.len),
        "zeros" => vec![0u8; f.len],
        "pattern" => PATTERN.iter().copied().cycle().take(f.len).collect(),
        other => panic!("unknown fixture source {other}"),
    }
}

/// The specification text of `src/profile.rs`, implemented without the fastcdc crate except
/// for its published tables (identified by digest).
fn spec_chunks(
    bytes: &[u8],
    min: usize,
    avg: usize,
    max: usize,
    level: u32,
) -> Vec<(usize, usize)> {
    let (gear, gear_ls) = fastcdc::v2020::get_gear_with_seed(0);
    let bits = (avg as f64).log2().round() as u32;
    let mask_s = fastcdc::v2020::MASKS[(bits + level) as usize];
    let mask_l = fastcdc::v2020::MASKS[(bits - level) as usize];
    let (mask_s_ls, mask_l_ls) = (mask_s << 1, mask_l << 1);
    let mut out = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let w = &bytes[start..];
        let n = w.len();
        let len = if n <= min {
            n
        } else {
            let limit = n.min(max);
            let center = if limit >= avg { avg } else { limit };
            let mut hash: u64 = 0;
            let mut i = min / 2;
            let mut cut = None;
            while cut.is_none() && i < limit / 2 {
                let (ms_ls, ms) = if i < center / 2 {
                    (mask_s_ls, mask_s)
                } else {
                    (mask_l_ls, mask_l)
                };
                let a = 2 * i;
                hash = (hash << 2).wrapping_add(gear_ls[w[a] as usize]);
                if hash & ms_ls == 0 {
                    cut = Some(a);
                    break;
                }
                hash = hash.wrapping_add(gear[w[a + 1] as usize]);
                if hash & ms == 0 {
                    cut = Some(a + 1);
                }
                i += 1;
            }
            cut.unwrap_or(limit)
        };
        out.push((start, len));
        start += len;
    }
    out
}

/// A reader that hands out at most `split` bytes per read.
struct Split<'a> {
    bytes: &'a [u8],
    split: usize,
}

impl Read for Split<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.split.min(buf.len()).min(self.bytes.len());
        buf[..n].copy_from_slice(&self.bytes[..n]);
        self.bytes = &self.bytes[n..];
        Ok(n)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn blake3_hex(bytes: &[u8]) -> String {
    use multihash_codetable::{Code, MultihashDigest};
    hex(Code::Blake3_256.digest(bytes).digest())
}

fn entry(f: &Fixture, bytes: &[u8], chunks: &[(usize, usize)]) -> Value {
    let mut e = json!({
        "name": f.name,
        "source": f.source,
        "length": f.len,
        "inputBlake3": blake3_hex(bytes),
        "wholeCid": compute_bytes_cid(bytes).to_string(),
        "chunks": chunks
            .iter()
            .map(|&(o, l)| json!([o, l, compute_bytes_cid(&bytes[o..o + l]).to_string()]))
            .collect::<Vec<_>>(),
    });
    if f.source == "pattern" {
        e["pattern"] = json!(String::from_utf8_lossy(PATTERN));
    }
    e
}

fn header() -> Value {
    json!({
        "about": "fastcdc-2020 chunker vectors, byte profile epoch 0 (elohim/epr/src/profile.rs). Written once by elohim/epr/tests/chunker_vectors.rs; compare-only after that: REGENERATE_VECTORS=1 may add an absent fixture and refuses to change an existing one. Chunk and whole CIDs are the byte profile (raw 0x55 + BLAKE3-256).",
        "profileEpoch": 0,
        "chunker": serde_json::to_value(BYTE_PROFILE_0.chunker).unwrap(),
        "reference": "fastcdc =5.0.0, v2020::FastCDC::with_level(bytes, min, avg, max, Normalization::Level1), seed 0",
        "tableDigest": FASTCDC_2020_TABLE_DIGEST,
        "tableDigestOf": "BLAKE3-256 of GEAR || GEAR_LS || MASKS, each u64 as 8 little-endian bytes",
        "generator": {
            "name": "xorshift64*",
            "shifts": [12, 25, 27],
            "multiplier": "0x2545f4914f6cdd1d",
            "seed": "0x454c4f48494d",
            "output": "each step's 64-bit product as 8 little-endian bytes, the stream truncated to length; every xorshift64* fixture is a prefix of the same stream",
        },
    })
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../eprfs/fixtures/tree-vectors.json")
}

#[test]
fn chunker_vector_is_stable() {
    assert_eq!(fastcdc_2020_table_digest(), FASTCDC_2020_TABLE_DIGEST);
    let Chunker::Fastcdc2020 {
        min,
        avg,
        max,
        level,
    } = BYTE_PROFILE_0.chunker;
    assert_eq!(
        (min as usize, avg as usize, max as usize, level),
        (MIN, AVG, MAX, 1)
    );

    let computed: Vec<Value> = fixtures()
        .iter()
        .map(|f| {
            let bytes = input(f);
            let chunks = chunk_bytes(&ValidatedProfile::EPOCH_0, &bytes);
            assert_eq!(
                chunks,
                spec_chunks(&bytes, MIN, AVG, MAX, 1),
                "{}: the specification text and the reference disagree",
                f.name
            );
            assert_eq!(chunks.iter().map(|c| c.1).sum::<usize>(), bytes.len());
            entry(f, &bytes, &chunks)
        })
        .collect();

    let path = fixture_path();
    let text = std::fs::read_to_string(&path).expect("tree-vectors.json readable");
    let mut file: Value = serde_json::from_str(&text).expect("tree-vectors.json parses");
    let regenerate = std::env::var_os("REGENERATE_VECTORS").is_some();

    let mut added = Vec::new();
    match file.get_mut("chunker") {
        None => {
            assert!(
                regenerate,
                "tree-vectors.json has no `chunker` section; write it once with REGENERATE_VECTORS=1"
            );
            let mut section = header();
            section["vectors"] = json!(computed);
            file["chunker"] = section;
            added.push("<section>".to_string());
        }
        Some(section) => {
            let mut expected_header = header();
            expected_header["vectors"] = section["vectors"].clone();
            assert_eq!(
                *section, expected_header,
                "the chunker section's declaration changed: refusing (epoch-0 vectors are compare-only)"
            );
            let pinned = section["vectors"].as_array_mut().expect("vectors array");
            for c in &computed {
                match pinned.iter().find(|p| p["name"] == c["name"]) {
                    Some(p) => assert_eq!(
                        p, c,
                        "epoch-0 chunker vector {} changed: refusing to rewrite it",
                        c["name"]
                    ),
                    None => {
                        assert!(regenerate, "fixture {} is not pinned yet", c["name"]);
                        added.push(c["name"].to_string());
                    }
                }
            }
            for p in pinned.iter() {
                assert!(
                    computed.iter().any(|c| c["name"] == p["name"]),
                    "pinned fixture {} is no longer computed",
                    p["name"]
                );
            }
            for c in &computed {
                if !pinned.iter().any(|p| p["name"] == c["name"]) {
                    pinned.push(c.clone());
                }
            }
        }
    }
    if !added.is_empty() {
        std::fs::write(&path, serde_json::to_string_pretty(&file).unwrap() + "\n").unwrap();
        eprintln!("added chunker vectors: {added:?}");
    }
}

#[test]
fn fixture_shapes_hold() {
    let by_name = |name: &str| {
        let f = fixtures().into_iter().find(|f| f.name == name).unwrap();
        let bytes = input(&f);
        chunk_bytes(&ValidatedProfile::EPOCH_0, &bytes)
    };
    assert!(by_name("empty").is_empty());
    assert_eq!(by_name("365B"), vec![(0, 365)]);
    assert_eq!(by_name("min-1"), vec![(0, MIN - 1)]);
    let forced = by_name("forced-max");
    assert!(
        forced[..forced.len() - 1].iter().all(|c| c.1 == MAX),
        "{forced:?}"
    );
    let tail = by_name("short-tail");
    assert_eq!(tail.last().unwrap().1, 100, "final chunk may be below min");
    assert!(by_name("max+1").iter().all(|c| c.1 <= MAX));
}

#[test]
fn streaming_splits_equal_one_shot() {
    for f in fixtures() {
        if !matches!(
            f.name,
            "main-10MiB" | "odd" | "max+1" | "365B" | "empty" | "short-tail"
        ) {
            continue;
        }
        let bytes = input(&f);
        let one_shot = chunk_bytes(&ValidatedProfile::EPOCH_0, &bytes);
        for split in [1024, 7 * 1024, 1024 * 1024 + 1] {
            let streamed: Vec<(usize, usize)> = chunk_stream(
                &BYTE_PROFILE_0,
                Split {
                    bytes: &bytes,
                    split,
                },
            )
            .unwrap()
            .map(|c| {
                let c = c.unwrap();
                assert_eq!(
                    c.data[..],
                    bytes[c.offset as usize..c.offset as usize + c.data.len()]
                );
                (c.offset as usize, c.data.len())
            })
            .collect();
            assert_eq!(streamed, one_shot, "{} split {split}", f.name);
        }
    }
}
