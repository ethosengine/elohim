//! ark-core — the pure half of the elohim compute envelope (tevah).
//!
//! Spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §3, §5.1, §6, §8.
//! No I/O and no async runtime live here; `ark-supervisor` implements the traits in `sink` and
//! is the only ARK construction site for sidecar stores.

pub mod berth;
pub mod candidate;
pub mod declaration;
pub mod exit;
pub mod intent;
pub mod lifecycle;
pub mod manifest;
pub mod passport;
pub mod rea;
pub mod ring;
pub mod sample;
pub mod sink;
pub mod tally;
pub mod task;
pub mod verdict;
pub mod witness;

// Re-exports are uncommented by the task that creates each type.
pub use berth::{Berth, LogLevel, PassphraseSource};
pub use elohim_compute::{LimitOwner, Refusal};
pub use exit::{classify_readiness_outcome, ExitClass, ReadinessOutcome};
pub use intent::{Intent, IntentAction};
pub use lifecycle::{step, Action, ChildState, Event, IncidentCloseKind};
pub use manifest::{ArtifactRef, ChildPolicy, ChildSpec, Probe, ProcessKind, RuntimeManifest};
pub use passport::{EffectiveTier, Passport, ProcessPassport, PASSPORT_KIND};
pub use rea::{
    ReaProjectionError, RuntimeScope, INCIDENT_SPEC_ID, INCIDENT_SPEC_VERSION,
    RUNTIME_TAG_BERTH_OFFER, RUNTIME_TAG_DEATH, RUNTIME_TAG_GIVE_UP, RUNTIME_TAG_INCIDENT_CLOSED,
    RUNTIME_TAG_KILL, RUNTIME_TAG_RESTART, RUNTIME_TAG_SPAWN, RUNTIME_TAG_STOP, UNIDENTIFIED_AGENT,
    UNIT_CPU_MILLIS, UNIT_DEATHS, UNIT_DISK_BYTES, UNIT_MEMORY_BYTES, UNIT_PIDS, UNIT_PROCESS_MS,
};
pub use ring::RingBuffer;
pub use sample::ProcessSample;
pub use sink::{Clock, SinkError, WitnessSink};
pub use tally::{DeathRecord, DeathTally};
pub use task::{
    eprfs_depth, placement_depth, BerthOffer, TaskChild, TaskKind, TaskOutcome, TaskSpec,
    MAX_OFFER_HOLDS, TASK_CHILD_NAME, TASK_TERMS,
};
pub use verdict::{
    BoundedBy, GiveUpReason, RestartContext, RestartGovernor, RestartGrant, RestartRequest,
    RestartVerdict,
};
pub use witness::{DeathWitness, Incident, IncidentClose, WitnessError, WITNESS_KIND};

#[cfg(test)]
mod boundary {
    /// Every module's source, read at compile time so the scan below needs no reader.
    ///
    /// `boundary_scan_covers_every_module` keeps this list honest as modules are added.
    const SOURCES: &[(&str, &str)] = &[
        ("berth.rs", include_str!("berth.rs")),
        ("candidate.rs", include_str!("candidate.rs")),
        ("declaration.rs", include_str!("declaration.rs")),
        ("exit.rs", include_str!("exit.rs")),
        ("intent.rs", include_str!("intent.rs")),
        ("lib.rs", include_str!("lib.rs")),
        ("lifecycle.rs", include_str!("lifecycle.rs")),
        ("manifest.rs", include_str!("manifest.rs")),
        ("passport.rs", include_str!("passport.rs")),
        ("rea.rs", include_str!("rea.rs")),
        ("ring.rs", include_str!("ring.rs")),
        ("sample.rs", include_str!("sample.rs")),
        ("sink.rs", include_str!("sink.rs")),
        ("tally.rs", include_str!("tally.rs")),
        ("task.rs", include_str!("task.rs")),
        ("verdict.rs", include_str!("verdict.rs")),
        ("witness.rs", include_str!("witness.rs")),
    ];

    /// The denied packages, each with the reason it is denied.
    const DENIED: &[(&str, &str)] = &[
        (
            "tokio",
            "ark-core is plain data + decisions; the supervisor owns the runtime",
        ),
        ("nix", "syscalls are the supervisor's"),
        ("libc", "syscalls are the supervisor's"),
        ("diesel", "persistence is a storage concern"),
        ("rusqlite", "persistence is a storage concern"),
        (
            "libp2p",
            "the envelope has no swarm in v1 (spec §12 item 11)",
        ),
        ("iroh", "the envelope has no swarm in v1 (spec §12 item 11)"),
        ("reqwest", "no network in the envelope"),
        ("hyper", "no network in the envelope"),
        ("axum", "no network in the envelope"),
        ("hdk", "the envelope is below the DNA line"),
        ("hdi", "the envelope is below the DNA line"),
    ];

    /// One dependency a manifest's non-dev tables declare, read structurally.
    #[derive(Debug, PartialEq)]
    struct Declared {
        /// The package it resolves to: a `package = …` rename's target, else its key.
        package: String,
        /// `default-features = false` (or `default_features = false`) is written on it.
        default_off: bool,
        /// The features it asks for by name.
        features: Vec<String>,
        /// `optional = true` is written on it.
        optional: bool,
    }

    impl Declared {
        fn read(key: &str, spec: &toml_edit::Item) -> Self {
            let Some(table) = spec.as_table_like() else {
                // `name = "1"`: a version string, the package its own key, every default on.
                return Self {
                    package: key.to_string(),
                    default_off: false,
                    features: Vec::new(),
                    optional: false,
                };
            };
            let flag = |name: &str| table.get(name).and_then(toml_edit::Item::as_bool);
            Self {
                package: table
                    .get("package")
                    .and_then(toml_edit::Item::as_str)
                    .unwrap_or(key)
                    .to_string(),
                default_off: ["default-features", "default_features"]
                    .iter()
                    .any(|name| flag(name) == Some(false)),
                features: table
                    .get("features")
                    .and_then(toml_edit::Item::as_array)
                    .map(|features| {
                        features
                            .iter()
                            .filter_map(|feature| feature.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                optional: flag("optional") == Some(true),
            }
        }
    }

    /// The dependency tables Cargo builds a consumer with: `[dependencies]` and
    /// `[build-dependencies]` (either spelling), at the top level and under every `[target.*]`.
    /// `[dev-dependencies]` in any form is not among them: a test-only dependency never reaches a
    /// consumer's build.
    const BUILD_TABLES: [&str; 3] = ["dependencies", "build-dependencies", "build_dependencies"];

    /// Every dependency a manifest's non-dev tables declare, parsed as TOML (Codex review 2: the
    /// earlier line scanner missed a header followed by a comment and a single-quoted rename),
    /// with each `package` rename resolved to the package it names.
    fn declared_dependencies(toml: &str) -> Vec<Declared> {
        let doc: toml_edit::DocumentMut = toml
            .parse()
            .unwrap_or_else(|error| panic!("a manifest the boundary reads is not TOML: {error}"));
        let mut tables: Vec<&toml_edit::Item> = BUILD_TABLES
            .iter()
            .filter_map(|kind| doc.get(kind))
            .collect();
        if let Some(targets) = doc.get("target").and_then(toml_edit::Item::as_table_like) {
            for (_, target) in targets.iter() {
                tables.extend(BUILD_TABLES.iter().filter_map(|kind| target.get(kind)));
            }
        }
        tables
            .into_iter()
            .flat_map(|table| {
                let table = table
                    .as_table_like()
                    .expect("a dependency table is a table");
                table
                    .iter()
                    .map(|(key, spec)| Declared::read(key, spec))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The packages [`declared_dependencies`] names.
    fn declared_packages(toml: &str) -> Vec<String> {
        declared_dependencies(toml)
            .into_iter()
            .map(|declared| declared.package)
            .collect()
    }

    /// The purity boundary, read from this crate's own manifest and, one hop down, from the
    /// manifests of the two pure cores §2.F added (rail R-A1): a runtime or I/O dependency
    /// arriving here, directly or through eprfs-core or placement-core, would let a pure
    /// decision do I/O. Those two crates keep their own exact-set rails (R-E1, R-P1); this one
    /// fails ark-core's build if either admits a denied package.
    #[test]
    fn no_runtime_or_io_deps() {
        let manifests = [
            ("elohim-ark-core", include_str!("../Cargo.toml")),
            (
                "eprfs-core",
                include_str!("../../../eprfs/eprfs-core/Cargo.toml"),
            ),
            (
                "elohim-placement-core",
                include_str!("../../../placement/core/Cargo.toml"),
            ),
        ];
        for (crate_name, toml) in manifests {
            let declared = declared_packages(toml);
            for (pkg, why) in DENIED {
                assert!(
                    !declared.iter().any(|name| name == pkg),
                    "{crate_name} declares `{pkg}` — {why}"
                );
            }
        }
        // ark-core reaches the two cores it names (the hop above is not reading stale manifests).
        let own = declared_packages(include_str!("../Cargo.toml"));
        for core in ["eprfs-core", "elohim-placement-core"] {
            assert!(
                own.iter().any(|name| name == core),
                "ark-core no longer names {core}"
            );
        }
    }

    /// §1's no-clock boundary through elohim-compute (Codex review 1): compute's default-on
    /// `runtime` feature carries its reporting modules and chrono's clock, and a pure consumer
    /// takes the crate with `default-features = false`. Placement-core's own `default-features =
    /// false` does not cancel a default ark-core turns on, so every manifest in the pure graph
    /// that names compute must disable the default and never ask for `runtime` by name. The second
    /// half pins what the first relies on: in compute's own manifest the clock is optional, behind
    /// `runtime`, so disabling the default really does leave it out.
    #[test]
    fn compute_runtime_is_unreachable() {
        let manifests = [
            ("elohim-ark-core", include_str!("../Cargo.toml")),
            (
                "eprfs-core",
                include_str!("../../../eprfs/eprfs-core/Cargo.toml"),
            ),
            (
                "elohim-placement-core",
                include_str!("../../../placement/core/Cargo.toml"),
            ),
        ];
        let mut named = 0;
        for (crate_name, toml) in manifests {
            for declared in declared_dependencies(toml)
                .into_iter()
                .filter(|declared| declared.package == "elohim-compute")
            {
                named += 1;
                assert!(
                    runtime_is_off(&declared),
                    "{crate_name} takes elohim-compute with its runtime feature reachable: {declared:?}"
                );
            }
        }
        assert!(named >= 1, "ark-core no longer names elohim-compute");

        let compute = include_str!("../../../elohim-compute/Cargo.toml");
        let chrono = declared_dependencies(compute)
            .into_iter()
            .find(|declared| declared.package == "chrono");
        assert!(
            chrono.is_some_and(|declared| declared.optional),
            "elohim-compute's chrono is no longer optional behind `runtime`"
        );
    }

    /// A compute dependency with the default off and no `runtime` asked for by name.
    fn runtime_is_off(declared: &Declared) -> bool {
        declared.default_off && !declared.features.iter().any(|feature| feature == "runtime")
    }

    /// The compute dependency a one-table manifest declares.
    fn compute_in(toml: &str) -> Declared {
        declared_dependencies(toml)
            .into_iter()
            .find(|declared| declared.package == "elohim-compute")
            .unwrap_or_else(|| panic!("no elohim-compute in {toml:?}"))
    }

    #[test]
    fn the_runtime_check_refuses_every_form_that_reaches_it() {
        for toml in [
            "[dependencies]\nelohim-compute = { path = \"../c\" }\n",
            "[dependencies]\nelohim-compute = { path = \"../c\", default-features = true }\n",
            "[dependencies]\nelohim-compute = { path = \"../c\", default-features = false, features = [\"runtime\"] }\n",
            "[dependencies]\nelohim-compute = { path = '../c', default-features = false, features = ['runtime'] }\n",
            "[dependencies.elohim-compute]\npath = \"../c\"\n",
            "[dependencies]\nelohim-compute = \"1\"\n",
        ] {
            assert!(
                !runtime_is_off(&compute_in(toml)),
                "{toml:?} was read as runtime-free"
            );
        }
        for toml in [
            "[dependencies]\nelohim-compute = { path = \"../c\", default-features = false }\n",
            "[dependencies]\nelohim-compute = { path = \"../c\", default_features = false }\n",
            "[dependencies.elohim-compute]\npath = \"../c\"\ndefault-features = false\n",
            "[dependencies]\nc = { package = 'elohim-compute', path = '../c', default-features = false }\n",
        ] {
            assert!(
                runtime_is_off(&compute_in(toml)),
                "{toml:?} was read as reaching runtime"
            );
        }
    }

    /// The parser sees every form that adds a dependency, and skips only dev-dependencies. The old
    /// needle scan (`"\n{pkg} "`) missed `[dependencies.tokio]` and `tokio.workspace = true`; the
    /// line scanner that replaced it missed a commented header and a single-quoted rename (Codex
    /// review 2), the last two groups below.
    #[test]
    fn the_dependency_parser_sees_every_form_that_adds_a_dependency() {
        for toml in [
            "[dependencies] # runtime dependencies\ntokio = \"1\"\n",
            "[dependencies]\nrt = { package = 'tokio', version = '1' }\n",
            "[dependencies.rt]\npackage = 'tokio'\nversion = '1'\n",
            "[dependencies]\n\"tokio\" = \"1\"\n",
            "[dependencies]\nrt = {package=\"tokio\",version=\"1\"}\n",
            "[build_dependencies]\ntokio = \"1\"\n",
            "[target.\"cfg(unix)\".dependencies] # unix only\ntokio = \"1\"\n",
            "[dependencies]\ntokio = \"1\"\n",
            "[dependencies]\ntokio.workspace = true\n",
            "[dependencies]\nrt = { package = \"tokio\", version = \"1\" }\n",
            "[dependencies.tokio]\nversion = \"1\"\n",
            "[dependencies.rt]\npackage = \"tokio\"\nversion = \"1\"\n",
            "[target.'cfg(unix)'.dependencies]\ntokio = \"1\"\n",
            "[target.'cfg(unix)'.dependencies.tokio]\nversion = \"1\"\n",
            "[build-dependencies]\ntokio = \"1\"\n",
        ] {
            assert!(
                declared_packages(toml).iter().any(|name| name == "tokio"),
                "the parser missed tokio in {toml:?}"
            );
        }
        for toml in [
            "[dev-dependencies]\ntokio = \"1\"\n",
            "[dev-dependencies.tokio]\nversion = \"1\"\n",
            "[target.'cfg(unix)'.dev-dependencies]\ntokio = \"1\"\n",
            "[dev_dependencies]\ntokio = \"1\"\n",
            "[dependencies] # tokio = \"1\"\nserde = \"1\"\n",
            "[features]\ntokio = []\n",
        ] {
            assert!(
                !declared_packages(toml).iter().any(|name| name == "tokio"),
                "the parser read a non-dependency in {toml:?}"
            );
        }
    }

    /// The invariant-bearing fields stay private (Codex review 2). `TaskSpec`'s fields carry the
    /// exec binding, `TaskOutcome`'s the clean-exit rule for a derivation record, and
    /// `BerthOffer.holds` the bound and one-reach-per-root rule; each is checked only where the
    /// value is built or decoded, so a `pub` field would let a caller break it afterwards. The
    /// `compile_fail` doctests in `task.rs` pin the same thing, but the gate's `cargo test
    /// --all-targets` does not run doctests, and this scan does run there.
    #[test]
    fn invariant_bearing_fields_stay_private() {
        let task = include_str!("task.rs");
        for (name, guarded) in [
            ("TaskSpec", None),
            ("TaskOutcome", None),
            ("BerthOffer", Some(["holds"].as_slice())),
        ] {
            let open = format!("pub struct {name} {{");
            let at = task
                .find(&open)
                .unwrap_or_else(|| panic!("task.rs no longer declares `{open}`"));
            let body = &task[at + open.len()..];
            let body = &body[..body.find("\n}").expect("the struct body closes")];
            let mut fields = 0;
            for line in body.lines().map(str::trim) {
                if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                    continue;
                }
                let field = line
                    .trim_start_matches("pub ")
                    .split(':')
                    .next()
                    .unwrap_or("");
                if guarded.is_none_or(|guarded| guarded.contains(&field.trim())) {
                    fields += 1;
                    assert!(
                        !line.starts_with("pub "),
                        "`{name}.{}` is public; it carries an invariant checked only at construction",
                        field.trim()
                    );
                }
            }
            assert!(fields >= 1, "the scan read no field of `{name}`");
        }
    }

    /// No source reads a clock (§1; Codex review 1). chrono stays a dependency for its
    /// `DateTime` types, which a caller fills; reading "now" is the supervisor's, through
    /// [`crate::Clock`]. Needles are split for the same reason as below.
    #[test]
    fn no_clock_in_pure_core() {
        let denied: Vec<String> = ["Utc", "Local", "SystemTime", "Instant"]
            .iter()
            .map(|clock| format!("{clock}{}now", "::"))
            .collect();
        for (file, source) in SOURCES {
            for needle in &denied {
                assert!(
                    !source.contains(needle.as_str()),
                    "ark-core src/{file} reads a clock (`{needle}`); time is the caller's"
                );
            }
        }
    }

    /// The pure core touches no filesystem — not directly, and not through the substrate
    /// crate it now depends on.
    ///
    /// `elohim-epr-rea` is taken `default-features = false`, which drops its `sidecar`
    /// feature, so its stores are not compiled here at all. This test refuses the reach at
    /// SOURCE level too, because a dependency-graph fact is only a boundary while nobody
    /// re-enables the feature — and because a source-level scan names the offending line.
    ///
    /// The needles are assembled at run time from split literals: this test's own module is
    /// one of the files it reads, so a contiguous literal here would make the guard fail on
    /// itself.
    #[test]
    fn no_fs_in_pure_core() {
        let denied: [(String, &str); 4] = [
            (
                format!("std{}fs", "::"),
                "the supervisor owns every byte that touches a disk",
            ),
            (
                format!("File{}", "::"),
                "opening a handle is I/O, and I/O is the supervisor's",
            ),
            (
                format!("{}{}", "Sidecar", "FlowStore"),
                "the spool is constructed in ark-supervisor and nowhere else",
            ),
            (
                format!("{}{}", "Sidecar", "ActorStore"),
                "the spool is constructed in ark-supervisor and nowhere else",
            ),
        ];

        for (file, source) in SOURCES {
            for (needle, why) in &denied {
                assert!(
                    !source.contains(needle.as_str()),
                    "ark-core src/{file} names `{needle}` — {why}"
                );
            }
        }
    }

    /// A module added without an entry in [`SOURCES`] would be invisible to the scan above,
    /// which is how a purity boundary quietly stops covering the code it exists for.
    #[test]
    fn boundary_scan_covers_every_module() {
        for line in include_str!("lib.rs")
            .lines()
            .filter(|line| line.starts_with("pub mod "))
        {
            let module = line
                .trim_start_matches("pub mod ")
                .trim_end_matches(';')
                .trim();
            let expected = format!("{module}.rs");
            assert!(
                SOURCES.iter().any(|(name, _)| *name == expected),
                "src/{expected} is not scanned by the purity boundary"
            );
        }
        assert!(SOURCES.iter().any(|(name, _)| *name == "lib.rs"));
    }
}
