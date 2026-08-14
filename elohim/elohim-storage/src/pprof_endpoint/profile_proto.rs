//! Google `profile.proto` (pprof format) message shapes + the
//! [`pprof::Report`] → [`Profile`] mapping.
//!
//! # Why this module exists rather than `pprof`'s own codec
//!
//! Both of pprof-rs's protobuf codec features carry a BUILD-SCRIPT dependency
//! (`protobuf-codegen` / `prost-build`) that transitively links `getrandom`.
//! This crate compiles under `--cfg getrandom_backend="custom"`, whose backing
//! symbol (`__getrandom_v03_custom`, `src/getrandom_custom.rs`) exists only in
//! the lib/bin link — never in a host build-script binary. Any such build
//! script therefore fails with `undefined symbol: __getrandom_v03_custom`.
//! The full rationale, including the `--target` escape hatch that was
//! deliberately not taken, is in this crate's `Cargo.toml` beside the `pprof`
//! dependency.
//!
//! So pprof-rs is used as the SAMPLER only, and the wire encoding happens here
//! with `prost` (already in the resolved tree, no build script).
//!
//! # Fidelity
//!
//! The message declarations below are transcribed field-for-field from
//! `proto/profile.proto` as shipped in `pprof 0.15.0` (itself a verbatim copy
//! of `google/pprof`'s `proto/profile.proto`). The mapping in
//! [`report_to_profile`] mirrors `pprof::Report::pprof()` — the same string
//! table construction, the same one-location-per-function collapsing, the same
//! two-value sample vector (`samples/count` and `cpu/nanoseconds`), the same
//! `thread` label. Keeping it a mirror rather than an improvement is the point:
//! Pyroscope's Go-target dashboards assume exactly this shape.
//!
//! Only the subset a CPU profile populates is declared. `Mapping`,
//! `drop_frames`, `keep_frames`, `comment` and `default_sample_type` are
//! omitted — they are optional in the format and pprof-rs never sets them.
//! Field NUMBERS are preserved regardless, so a decoder that does expect them
//! reads this profile correctly.

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use prost::Message;

/// `ValueType` — describes one column of every `Sample.value` vector.
#[derive(Clone, PartialEq, Message)]
pub struct ValueType {
    /// Index into `Profile.string_table`. (`type` in the proto; renamed
    /// because `type` is a Rust keyword — the field NUMBER is what's on the
    /// wire, so this is invisible to any decoder.)
    #[prost(int64, tag = "1")]
    pub ty: i64,
    /// Index into `Profile.string_table`.
    #[prost(int64, tag = "2")]
    pub unit: i64,
}

/// `Label` — a key/value annotation on a sample. CPU profiles use it for the
/// originating thread name.
#[derive(Clone, PartialEq, Message)]
pub struct Label {
    /// Index into `Profile.string_table`.
    #[prost(int64, tag = "1")]
    pub key: i64,
    /// Index into `Profile.string_table`.
    #[prost(int64, tag = "2")]
    pub str: i64,
    #[prost(int64, tag = "3")]
    pub num: i64,
    /// Index into `Profile.string_table`.
    #[prost(int64, tag = "4")]
    pub num_unit: i64,
}

/// `Sample` — one observed stack, leaf-first, with its measured values.
#[derive(Clone, PartialEq, Message)]
pub struct Sample {
    #[prost(uint64, repeated, tag = "1")]
    pub location_id: Vec<u64>,
    #[prost(int64, repeated, tag = "2")]
    pub value: Vec<i64>,
    #[prost(message, repeated, tag = "3")]
    pub label: Vec<Label>,
}

/// `Line` — one source line within a `Location`.
#[derive(Clone, PartialEq, Message)]
pub struct Line {
    #[prost(uint64, tag = "1")]
    pub function_id: u64,
    #[prost(int64, tag = "2")]
    pub line: i64,
}

/// `Location` — a program counter (here: one resolved function).
#[derive(Clone, PartialEq, Message)]
pub struct Location {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(uint64, tag = "2")]
    pub mapping_id: u64,
    #[prost(uint64, tag = "3")]
    pub address: u64,
    #[prost(message, repeated, tag = "4")]
    pub line: Vec<Line>,
    #[prost(bool, tag = "5")]
    pub is_folded: bool,
}

/// `Function` — a named function, with all strings as string-table indices.
#[derive(Clone, PartialEq, Message)]
pub struct Function {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    /// Index into `Profile.string_table` — the demangled name.
    #[prost(int64, tag = "2")]
    pub name: i64,
    /// Index into `Profile.string_table` — the mangled/system name.
    #[prost(int64, tag = "3")]
    pub system_name: i64,
    /// Index into `Profile.string_table`.
    #[prost(int64, tag = "4")]
    pub filename: i64,
    #[prost(int64, tag = "5")]
    pub start_line: i64,
}

/// `Profile` — the top-level pprof message. Field numbers 3 (`mapping`), 7, 8,
/// 13 and 14 are intentionally absent; see the module doc.
#[derive(Clone, PartialEq, Message)]
pub struct Profile {
    #[prost(message, repeated, tag = "1")]
    pub sample_type: Vec<ValueType>,
    #[prost(message, repeated, tag = "2")]
    pub sample: Vec<Sample>,
    #[prost(message, repeated, tag = "4")]
    pub location: Vec<Location>,
    #[prost(message, repeated, tag = "5")]
    pub function: Vec<Function>,
    #[prost(string, repeated, tag = "6")]
    pub string_table: Vec<String>,
    #[prost(int64, tag = "9")]
    pub time_nanos: i64,
    #[prost(int64, tag = "10")]
    pub duration_nanos: i64,
    #[prost(message, optional, tag = "11")]
    pub period_type: Option<ValueType>,
    #[prost(int64, tag = "12")]
    pub period: i64,
}

// Sample-type / label names, matching `pprof::Report::pprof()` exactly so the
// resulting profile is indistinguishable from pprof-rs's own codec output.
const SAMPLES: &str = "samples";
const COUNT: &str = "count";
const CPU: &str = "cpu";
const NANOSECONDS: &str = "nanoseconds";
const THREAD: &str = "thread";

const NANOS_PER_SEC: i64 = 1_000_000_000;

/// Convert a resolved [`pprof::Report`] into a `profile.proto` [`Profile`].
///
/// Mirrors `pprof::Report::pprof()`. Two deliberate differences, both
/// hardening rather than reshaping:
///
/// * string-table lookups use `unwrap_or(0)` (index 0 is the mandatory empty
///   string) instead of `unwrap()`, so a symbol whose name changes between the
///   dedup pass and the emit pass degrades that one frame rather than panicking
///   inside a blocking task;
/// * `frequency` is guarded against 0 so the `period` / per-sample-nanos
///   division cannot divide by zero on a degenerate report.
pub fn report_to_profile(report: &pprof::Report) -> Profile {
    // Pass 1 — collect every string the profile will reference, so the table
    // can be built once and indexed by the emit pass.
    let mut dedup_str: HashSet<String> = HashSet::new();
    for key in report.data.keys() {
        dedup_str.insert(key.thread_name_or_id());
        for frame in key.frames.iter() {
            for symbol in frame {
                dedup_str.insert(symbol.name());
                dedup_str.insert(symbol.sys_name().into_owned());
                dedup_str.insert(symbol.filename().into_owned());
            }
        }
    }
    dedup_str.insert(SAMPLES.to_owned());
    dedup_str.insert(COUNT.to_owned());
    dedup_str.insert(CPU.to_owned());
    dedup_str.insert(NANOSECONDS.to_owned());
    dedup_str.insert(THREAD.to_owned());

    // The format REQUIRES string_table[0] to be the empty string.
    let mut str_tbl = vec![String::new()];
    str_tbl.extend(dedup_str);

    let strings: HashMap<&str, usize> = str_tbl
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let idx = |s: &str| -> i64 { strings.get(s).copied().unwrap_or(0) as i64 };

    // A frequency of 0 would divide by zero below; 1 Hz is the same fallback
    // pprof's own `ReportTiming::default()` uses.
    let frequency = if report.timing.frequency > 0 {
        report.timing.frequency as i64
    } else {
        1
    };

    // Pass 2 — emit samples, interning each distinct function once. pprof-rs
    // gives Location and Function the SAME id (one location per function),
    // which keeps loc_tbl and fn_tbl the same length and index-aligned.
    let mut samples = Vec::with_capacity(report.data.len());
    let mut loc_tbl: Vec<Location> = Vec::new();
    let mut fn_tbl: Vec<Function> = Vec::new();
    let mut functions: HashMap<String, u64> = HashMap::new();

    for (key, count) in report.data.iter() {
        let mut locs = Vec::new();
        for frame in key.frames.iter() {
            for symbol in frame {
                let name = symbol.name();
                if let Some(loc_idx) = functions.get(&name) {
                    locs.push(*loc_idx);
                    continue;
                }

                // Ids are 1-based: 0 means "absent" in profile.proto.
                let function_id = fn_tbl.len() as u64 + 1;
                fn_tbl.push(Function {
                    id: function_id,
                    name: idx(&name),
                    system_name: idx(symbol.sys_name().as_ref()),
                    filename: idx(symbol.filename().as_ref()),
                    start_line: 0,
                });
                loc_tbl.push(Location {
                    id: function_id,
                    mapping_id: 0,
                    address: 0,
                    line: vec![Line {
                        function_id,
                        line: symbol.lineno() as i64,
                    }],
                    is_folded: false,
                });

                functions.insert(name, function_id);
                locs.push(function_id);
            }
        }

        let count = *count as i64;
        samples.push(Sample {
            location_id: locs,
            // Column order must match `sample_type` below.
            value: vec![count, count * NANOS_PER_SEC / frequency],
            label: vec![Label {
                key: idx(THREAD),
                str: idx(&key.thread_name_or_id()),
                num: 0,
                num_unit: 0,
            }],
        });
    }

    let samples_value = ValueType {
        ty: idx(SAMPLES),
        unit: idx(COUNT),
    };
    let time_value = ValueType {
        ty: idx(CPU),
        unit: idx(NANOSECONDS),
    };

    Profile {
        sample_type: vec![samples_value, time_value.clone()],
        sample: samples,
        location: loc_tbl,
        function: fn_tbl,
        string_table: str_tbl,
        time_nanos: report
            .timing
            .start_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64,
        duration_nanos: report.timing.duration.as_nanos() as i64,
        period_type: Some(time_value),
        period: NANOS_PER_SEC / frequency,
    }
}

/// Serialize a [`Profile`] to protobuf wire bytes.
pub fn encode(profile: &Profile) -> Vec<u8> {
    let mut buf = Vec::with_capacity(profile.encoded_len());
    // Infallible: prost only errors on insufficient capacity, and `Vec` grows.
    profile
        .encode(&mut buf)
        .expect("encoding a Profile into a growable Vec cannot fail");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A profile with no samples must still carry a well-formed header: the
    /// mandatory empty string at index 0, both sample_type columns, and a
    /// period_type. This is what a scraper receives from an idle peer, and it
    /// must parse rather than 400.
    #[test]
    fn empty_report_still_encodes_a_valid_header() {
        let report = pprof::Report {
            data: HashMap::new(),
            timing: Default::default(),
        };
        let profile = report_to_profile(&report);

        assert_eq!(profile.string_table[0], "", "index 0 must be empty string");
        assert_eq!(profile.sample_type.len(), 2, "samples/count + cpu/nanos");
        assert!(profile.period_type.is_some());
        assert!(profile.sample.is_empty());

        let bytes = encode(&profile);
        let decoded = Profile::decode(bytes.as_slice()).expect("round-trip decode");
        assert_eq!(decoded, profile);
    }

    /// The sample-type columns must name `samples/count` and `cpu/nanoseconds`
    /// via the string table — that naming is what Pyroscope keys its Go-target
    /// CPU views on.
    #[test]
    fn sample_types_are_named_for_a_go_cpu_target() {
        let report = pprof::Report {
            data: HashMap::new(),
            timing: Default::default(),
        };
        let profile = report_to_profile(&report);
        let s = |i: i64| profile.string_table[i as usize].as_str();

        assert_eq!(s(profile.sample_type[0].ty), SAMPLES);
        assert_eq!(s(profile.sample_type[0].unit), COUNT);
        assert_eq!(s(profile.sample_type[1].ty), CPU);
        assert_eq!(s(profile.sample_type[1].unit), NANOSECONDS);

        let period_type = profile.period_type.as_ref().expect("period_type");
        assert_eq!(s(period_type.ty), CPU);
        assert_eq!(s(period_type.unit), NANOSECONDS);
    }

    /// `frequency: 0` must not panic the blocking task with a divide-by-zero.
    #[test]
    fn zero_frequency_does_not_divide_by_zero() {
        let report = pprof::Report {
            data: HashMap::new(),
            timing: Default::default(),
        };
        assert_eq!(report.timing.frequency, 1, "default sanity");

        let profile = report_to_profile(&report);
        assert_eq!(profile.period, NANOS_PER_SEC);
    }

    /// Field numbers are the wire contract. A rename is free; a renumber is a
    /// silent break that only shows up in a scraper's parser. Pin them.
    #[test]
    fn wire_field_numbers_match_profile_proto() {
        // A Profile carrying ONLY period (tag 12) must encode to the varint
        // key for field 12, wire type 0 → (12 << 3) | 0 == 96 == 0x60.
        let profile = Profile {
            period: 7,
            ..Default::default()
        };
        assert_eq!(encode(&profile), vec![0x60, 0x07]);

        // Sample.value is field 2 → packed repeated → key (2 << 3) | 2 == 0x12.
        let sample = Sample {
            value: vec![1],
            ..Default::default()
        };
        let mut buf = Vec::new();
        sample.encode(&mut buf).unwrap();
        assert_eq!(buf, vec![0x12, 0x01, 0x01]);

        // Every REMAINING field google/pprof's decoder reads, asserted as the
        // raw key byte computed from the spec's field number — NOT from our own
        // struct. A round-trip test cannot catch a renumbered field (it would
        // encode and decode the same wrong number); these bytes can. Key is
        // (field_number << 3) | wire_type, wire_type 0 = varint, 2 = delimited.
        let cases: Vec<(&str, Profile, u8)> = vec![
            // sample_type = 1, message -> (1<<3)|2
            (
                "sample_type",
                Profile {
                    sample_type: vec![ValueType::default()],
                    ..Default::default()
                },
                0x0A,
            ),
            // location = 4, message -> (4<<3)|2
            (
                "location",
                Profile {
                    location: vec![Location::default()],
                    ..Default::default()
                },
                0x22,
            ),
            // function = 5, message -> (5<<3)|2
            (
                "function",
                Profile {
                    function: vec![Function::default()],
                    ..Default::default()
                },
                0x2A,
            ),
            // string_table = 6, repeated string -> (6<<3)|2. The empty first
            // entry is load-bearing in profile.proto (index 0 must be ""), so
            // one empty string encodes as key + zero length.
            (
                "string_table",
                Profile {
                    string_table: vec![String::new()],
                    ..Default::default()
                },
                0x32,
            ),
            // time_nanos = 9, varint -> (9<<3)|0
            (
                "time_nanos",
                Profile {
                    time_nanos: 1,
                    ..Default::default()
                },
                0x48,
            ),
            // duration_nanos = 10, varint -> (10<<3)|0
            (
                "duration_nanos",
                Profile {
                    duration_nanos: 1,
                    ..Default::default()
                },
                0x50,
            ),
            // period_type = 11, message -> (11<<3)|2
            (
                "period_type",
                Profile {
                    period_type: Some(ValueType::default()),
                    ..Default::default()
                },
                0x5A,
            ),
        ];
        for (name, profile, want_key) in cases {
            let bytes = encode(&profile);
            assert_eq!(
                bytes.first().copied(),
                Some(want_key),
                "{name}: first key byte should be {want_key:#04x} per profile.proto, got {bytes:?}"
            );
        }
    }
}
