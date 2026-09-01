//! Runtime configuration — a watched file whose changes apply to a RUNNING node.
//!
//! # Why this exists
//!
//! Rung 4 of the upgrade-velocity debt snowball (backlog
//! `upgrade-propagation-p2p-design-arc`). Every operator-reserved flag in this
//! crate is read from the environment exactly once, at boot, into a process-wide
//! mirror. That makes a flag flip cost a pod image roll — 2–4h of wall clock and
//! ~20min of restart churn on the fleet — for a change that alters one bit of
//! behaviour. This module makes the same flags reachable in seconds-to-minutes:
//! a ConfigMap edit, a poll tick, a WARN line in the log.
//!
//! # What it is NOT
//!
//! Not a second source of truth. The boot value still comes from the environment
//! exactly as it always did (`main.rs` reads env → `Config` field →
//! `config::set_*` → [`publish_boot_bool`] / [`publish_boot_secs`]). This module
//! only lets a FILE override that boot value while the process runs, and
//! restores the boot value the moment the file stops naming the key. Provenance
//! is always visible: [`Provenance::BootEnv`] or [`Provenance::RuntimeConfig`].
//!
//! Not a way to reach every knob. A setting is registered here only when its
//! read site genuinely consults the mirror per-decision. A knob captured once at
//! task spawn (a `tokio::time::interval` built from it, a struct field moved into
//! a closure) is honestly declared BOOT-ONLY in [`BOOT_ONLY`] rather than
//! pretended hot — a lever that reports "applied" and changes nothing is worse
//! than no lever.
//!
//! # Format
//!
//! Deliberately a hand-editable subset of TOML — `KEY = "value"` lines, `#`
//! comments, `[section]` headers ignored. No dependency is added for this: the
//! `toml` crate is not in this crate's tree, and a 40-line scanner that an
//! operator can predict beats a parse surface nobody reads.
//!
//! ```text
//! # /etc/elohim/runtime-config.toml
//! ELOHIM_OBEY_CARRIED_ELECTION = "1"
//! CONTEST_BACKOFF_SECONDS = 600
//! ```
//!
//! Keys are the ENV VAR NAMES — one vocabulary, so an operator who knows the
//! flag knows the key. A key that is absent, unparseable, or unknown leaves the
//! boot value in force (the "keep the default rather than silently disabling the
//! lever" discipline this crate already holds for env parsing).
//!
//! # Local-mesh usage
//!
//! The watcher is OFF unless `ELOHIM_RUNTIME_CONFIG_PATH` names a file. On the
//! local mesh, export it before starting a storage peer, then edit the file and
//! watch the log — no restart, no rebuild:
//!
//! ```bash
//! # before starting the peer (the mesh's own start arm is not this module's business)
//! export ELOHIM_RUNTIME_CONFIG_PATH=/tmp/elohim-local-mesh/runtime-config.toml
//! : > "$ELOHIM_RUNTIME_CONFIG_PATH"
//!
//! # …peer is running…
//! echo 'ELOHIM_OBEY_CARRIED_ELECTION = "1"' >> "$ELOHIM_RUNTIME_CONFIG_PATH"
//! # within POLL_INTERVAL_SECS the log carries:
//! #   WARN runtime-config: setting changed setting=ELOHIM_OBEY_CARRIED_ELECTION old=false new=true
//!
//! # confirm, or force an immediate re-read instead of waiting for the poll
//! curl -s localhost:8090/admin/runtime-config | jq
//! curl -s -X POST localhost:8090/admin/runtime-config/reload | jq
//!
//! # remove the line → the boot-env value comes back, logged the same way
//! ```
//!
//! On the fleet the same path is a mounted ConfigMap (e.g.
//! `/etc/elohim/runtime-config.toml`); a ConfigMap edit propagates to the pod's
//! mount and the poller picks it up without restarting anything.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::{info, warn};

/// How often the watcher re-stats the config file.
///
/// std-only by construction: no `notify`/inotify dependency is added for a
/// once-in-a-while operator edit. 10s means a flag flip lands within one poll —
/// seconds-to-minutes, which is the whole point of this rung — while costing one
/// `stat(2)` per tick.
pub const POLL_INTERVAL_SECS: u64 = 10;

/// Environment variable naming the watched file. Unset (or empty) disables the
/// watcher entirely, with one INFO line at boot and no further cost.
pub const PATH_ENV: &str = "ELOHIM_RUNTIME_CONFIG_PATH";

// ─── the registry ────────────────────────────────────────────────────────────

/// Value shape of a registered setting. Both are stored in an [`AtomicU64`]
/// (bool as 0/1) so the registry is one uniform, lock-free array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Truthy/falsy flag: `1|true|yes|on` / `0|false|no|off`.
    Bool,
    /// A duration in whole seconds.
    Seconds,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Bool => "bool",
            Kind::Seconds => "seconds",
        }
    }

    /// Parse a raw file value into the registry's `u64` representation.
    /// `None` means "unparseable" — the caller keeps the boot value.
    fn parse(self, raw: &str) -> Option<u64> {
        let raw = raw.trim();
        match self {
            Kind::Bool => match raw.to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Some(1),
                "0" | "false" | "no" | "off" => Some(0),
                _ => None,
            },
            Kind::Seconds => raw.parse::<u64>().ok(),
        }
    }

    fn render(self, value: u64) -> serde_json::Value {
        match self {
            Kind::Bool => serde_json::Value::Bool(value != 0),
            Kind::Seconds => serde_json::Value::from(value),
        }
    }

    fn display(self, value: u64) -> String {
        match self {
            Kind::Bool => (value != 0).to_string(),
            Kind::Seconds => value.to_string(),
        }
    }
}

/// Where a setting's CURRENT value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// The boot environment (or the compile-time default when nothing published).
    BootEnv,
    /// The watched runtime-config file is currently naming this key.
    RuntimeConfig,
}

impl Provenance {
    fn as_str(self) -> &'static str {
        match self {
            Provenance::BootEnv => "boot-env",
            Provenance::RuntimeConfig => "runtime-config",
        }
    }

    fn from_u8(v: u8) -> Self {
        if v == 1 {
            Provenance::RuntimeConfig
        } else {
            Provenance::BootEnv
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            Provenance::BootEnv => 0,
            Provenance::RuntimeConfig => 1,
        }
    }
}

/// Identifier for a registered hot-reloadable setting.
///
/// The discriminant IS the index into [`Registry::settings`], so lookup is an
/// array index and adding a key without adding its spec is a compile-time
/// length mismatch rather than a runtime surprise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// `ELOHIM_OBEY_CARRIED_ELECTION` — the election-visibility wall's exit.
    ObeyCarriedElection = 0,
    /// `ELOHIM_ADOPT_BEFORE_AUTHOR` — the both-sides-missing residual's exit.
    AdoptBeforeAuthor = 1,
    /// `CONTEST_BACKOFF_SECONDS` — predictable-contest-failure hold-back window.
    ContestBackoffSeconds = 2,
    /// `HEAL_MISSING_BACKOFF_SECONDS` — heal-leg conductor-missing replay window.
    HealMissingBackoffSeconds = 3,
    /// `ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS` — advertiser-stated-no-record window.
    EvidenceAbsentBackoffSecs = 4,
    /// `PROJECTION_RECONCILE_SECS` — projection-reconcile sweep cadence.
    ProjectionReconcileSecs = 5,
}

impl Key {
    const fn index(self) -> usize {
        self as usize
    }

    /// Every registered key, in registry order.
    pub const ALL: [Key; 6] = [
        Key::ObeyCarriedElection,
        Key::AdoptBeforeAuthor,
        Key::ContestBackoffSeconds,
        Key::HealMissingBackoffSeconds,
        Key::EvidenceAbsentBackoffSecs,
        Key::ProjectionReconcileSecs,
    ];
}

/// Static description of a registered setting. The mutable state lives in
/// [`Setting`]; this is the part that is the same in every process.
pub struct SettingSpec {
    /// The env-var name, which is ALSO the runtime-config file key.
    pub name: &'static str,
    pub kind: Kind,
    /// Compile-time fallback, used when `main` never published a boot value
    /// (embedded/test use). Mirrors the pre-registry `unwrap_or(&default)`.
    pub default: u64,
    /// What flipping this actually does, for the `/admin/runtime-config` reader.
    pub doc: &'static str,
    /// Anything an operator must know before flipping it live. `None` = clean.
    pub note: Option<&'static str>,
}

/// The registered settings, in [`Key`] order.
pub static SPECS: [SettingSpec; 6] = [
    SettingSpec {
        name: "ELOHIM_OBEY_CARRIED_ELECTION",
        kind: Kind::Bool,
        default: 0,
        doc: "May a divergent row whose OWN conductor sees no election obey a peer-carried \
              canonical-head declaration link, after this node's conductor re-derives it in wasm?",
        note: None,
    },
    SettingSpec {
        name: "ELOHIM_ADOPT_BEFORE_AUTHOR",
        kind: Kind::Bool,
        default: 0,
        doc: "May this node declare a canonical head for a content id it holds no local chain \
              for, on validated carried evidence?",
        note: None,
    },
    SettingSpec {
        name: "CONTEST_BACKOFF_SECONDS",
        kind: Kind::Seconds,
        default: crate::config::DEFAULT_CONTEST_BACKOFF_SECONDS,
        doc: "How long a predictable contest failure holds an id back. 0 DISABLES the backoff \
              (contest every candidate every sweep).",
        note: None,
    },
    SettingSpec {
        name: "HEAL_MISSING_BACKOFF_SECONDS",
        kind: Kind::Seconds,
        default: crate::config::DEFAULT_HEAL_MISSING_BACKOFF_SECONDS,
        doc: "How long the heal leg may replay a known conductor-missing answer instead of \
              paying for it again. 0 DISABLES the elision.",
        note: None,
    },
    SettingSpec {
        name: "ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS",
        kind: Kind::Seconds,
        default: crate::config::DEFAULT_EVIDENCE_ABSENT_BACKOFF_SECONDS,
        doc: "How long an id whose only advertiser states its conductor holds no record is held \
              back. 0 records the ordinary no-chain backoff instead.",
        note: None,
    },
    SettingSpec {
        name: "PROJECTION_RECONCILE_SECS",
        kind: Kind::Seconds,
        default: 300,
        doc: "Projection-reconcile sweep cadence. The running loop re-sources its ticker from \
              this value between wakes.",
        note: Some(
            "cadence only — a runtime 0 is IGNORED (it cannot stop a running loop, and the \
             loop is not spawned at all when the BOOT value is 0), and the change takes effect \
             after the next wake at the previous cadence",
        ),
    },
];

// ─── text settings ───────────────────────────────────────────────────────────
//
// The registry above is a lock-free array of `AtomicU64`, which is exactly
// right for a flag or a duration and cannot hold a LIST. The release-adoption
// controller (rung 5) needs one: the set of release channels this peer follows,
// each with a participation mode. Rather than widen `Kind` — which would cost
// every `AtomicU64` read site a branch for a shape almost nothing uses — text
// settings are a small parallel family with the SAME semantics: the file value
// overrides, an absent key restores the boot-env value, and provenance stays
// visible on the admin route.

/// Static description of a registered TEXT setting.
pub struct TextSettingSpec {
    /// The env-var name, which is ALSO the runtime-config file key.
    pub name: &'static str,
    /// What this value means, for the `/admin/runtime-config` reader.
    pub doc: &'static str,
}

/// The registered text settings.
pub static TEXT_SPECS: [TextSettingSpec; 1] = [TextSettingSpec {
    name: "ELOHIM_RELEASE_CHANNELS",
    doc: "Release channels this peer follows, as `channelId[=mode]` entries separated by \
          commas, semicolons or newlines. `observe` is the only legal mode until the apply \
          vehicles land; any other mode is REFUSED and reported on GET /admin/adoption \
          rather than silently downgraded. Empty (the default) leaves the adoption \
          controller idle.",
}];

struct TextSetting {
    /// The file override, when the watched file names this key.
    current: Mutex<Option<String>>,
    /// The boot-env value, restored when the file stops naming the key.
    boot: Mutex<Option<String>>,
}

static TEXT_SETTINGS: LazyLock<Vec<TextSetting>> = LazyLock::new(|| {
    TEXT_SPECS
        .iter()
        .map(|spec| TextSetting {
            current: Mutex::new(std::env::var(spec.name).ok().filter(|v| !v.is_empty())),
            boot: Mutex::new(std::env::var(spec.name).ok().filter(|v| !v.is_empty())),
        })
        .collect()
});

fn text_index(name: &str) -> Option<usize> {
    TEXT_SPECS.iter().position(|spec| spec.name == name)
}

/// The effective value of a registered text setting, or `None` when neither the
/// file nor the boot environment names it.
///
/// An unregistered name returns `None` rather than reading the environment
/// directly: a caller that can ask for any key at all is a caller that will
/// eventually ask for one nothing publishes, and get a permanent silent `None`.
pub fn get_text(name: &str) -> Option<String> {
    let idx = text_index(name)?;
    TEXT_SETTINGS[idx].current.lock().unwrap().clone()
}

/// Apply the parsed config map to the text settings. Same three cases as
/// [`Registry::apply`]: file wins, absent restores boot, unchanged is silent.
fn apply_text(parsed: &BTreeMap<String, String>) -> usize {
    let mut changed = 0usize;
    for (idx, spec) in TEXT_SPECS.iter().enumerate() {
        let setting = &TEXT_SETTINGS[idx];
        let from_file = parsed
            .get(spec.name)
            .map(|raw| raw.trim().to_string())
            .filter(|v| !v.is_empty());
        let want = match from_file {
            Some(v) => Some(v),
            None => setting.boot.lock().unwrap().clone(),
        };
        let mut current = setting.current.lock().unwrap();
        if *current != want {
            warn!(
                setting = spec.name,
                old = %current.as_deref().unwrap_or("<unset>"),
                new = %want.as_deref().unwrap_or("<unset>"),
                "runtime-config: setting changed on a RUNNING node"
            );
            *current = want;
            changed += 1;
        }
    }
    changed
}

fn text_snapshot() -> Vec<serde_json::Value> {
    TEXT_SPECS
        .iter()
        .enumerate()
        .map(|(idx, spec)| {
            let setting = &TEXT_SETTINGS[idx];
            let current = setting.current.lock().unwrap().clone();
            let boot = setting.boot.lock().unwrap().clone();
            let provenance = if current == boot {
                Provenance::BootEnv
            } else {
                Provenance::RuntimeConfig
            };
            serde_json::json!({
                "name": spec.name,
                "kind": "text",
                "effectiveValue": current,
                "bootValue": boot,
                "provenance": provenance.as_str(),
                "hotReloadable": true,
                "doc": spec.doc,
            })
        })
        .collect()
}

/// A knob this module deliberately does NOT hot-wire, and why. Surfaced on the
/// admin route so "why didn't my flip land?" is answerable without reading code.
pub struct BootOnlyFlag {
    pub name: &'static str,
    pub reason: &'static str,
}

/// Boot-only knobs in the same neighbourhood as the registered ones.
///
/// Honesty matters more than coverage here: each entry names a read site that
/// genuinely captures its value once, so registering it would ship a lever that
/// reports "applied" and changes nothing.
pub static BOOT_ONLY: [BootOnlyFlag; 5] = [
    BootOnlyFlag {
        name: "ACQUISITION_RECONCILE_SECS",
        reason: "captured once at spawn into a tokio::time::interval inside P2PNode::run \
                 (p2p/mod.rs); re-sourcing it needs a loop restructure, not an interval swap",
    },
    BootOnlyFlag {
        name: "ADOPT_CONTEST_FANOUT",
        reason: "held under assert_courier_ladder_budget (fanout * max_alternates <= 24), a \
                 fail-FAST boot invariant guarding the adam 2026-07-20 write-guard melt; a \
                 runtime flip would bypass the assertion instead of tripping it",
    },
    BootOnlyFlag {
        name: "ELOHIM_EVIDENCE_FALLBACK_MAX_ALTERNATES",
        reason: "the other factor in the same courier-ladder budget assertion",
    },
    BootOnlyFlag {
        name: "ELOHIM_TRANSPORT_BACKEND",
        reason: "selects which P2P stacks are BUILT at startup (libp2p / iroh / dual); a live \
                 change would have no node to apply to",
    },
    BootOnlyFlag {
        name: "PROJECTION_RECONCILE_SECS=0",
        reason: "the DISABLED case is boot-only — at 0 the reconcile loop is never spawned, so \
                 there is nothing for the watcher to re-source (the nonzero cadence IS hot)",
    },
];

/// Mutable per-process state for one registered setting.
struct Setting {
    current: AtomicU64,
    boot: AtomicU64,
    provenance: AtomicU8,
}

/// A lock-free registry of hot-reloadable settings.
///
/// Instantiable rather than purely static ON PURPOSE: the unit tests drive their
/// own `Registry` so they can exercise override/fallback/provenance transitions
/// without mutating the process-wide one that live code reads — the parallel-test
/// flake this crate has already paid for once with env vars.
pub struct Registry {
    settings: Vec<Setting>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// A registry with every setting at its compile-time default and provenance
    /// [`Provenance::BootEnv`].
    pub fn new() -> Self {
        Self {
            settings: SPECS
                .iter()
                .map(|spec| Setting {
                    current: AtomicU64::new(spec.default),
                    boot: AtomicU64::new(spec.default),
                    provenance: AtomicU8::new(Provenance::BootEnv.to_u8()),
                })
                .collect(),
        }
    }

    fn at(&self, key: Key) -> &Setting {
        &self.settings[key.index()]
    }

    /// Record the BOOT value for a setting (what the env said at startup).
    ///
    /// Updates the effective value too — UNLESS the watched file is currently
    /// overriding this key, in which case the override stays in force and only
    /// the fallback target moves. That ordering-independence is what lets
    /// `config::set_*` publish at any point in boot without racing the watcher.
    pub fn publish_boot(&self, key: Key, value: u64) {
        let s = self.at(key);
        s.boot.store(value, Ordering::Release);
        if Provenance::from_u8(s.provenance.load(Ordering::Acquire)) == Provenance::BootEnv {
            s.current.store(value, Ordering::Release);
        }
    }

    /// The effective value, in registry (`u64`) representation.
    pub fn get(&self, key: Key) -> u64 {
        self.at(key).current.load(Ordering::Acquire)
    }

    /// The effective value of a [`Kind::Bool`] setting.
    pub fn get_bool(&self, key: Key) -> bool {
        self.get(key) != 0
    }

    /// The boot value this setting falls back to when the file stops naming it.
    pub fn boot(&self, key: Key) -> u64 {
        self.at(key).boot.load(Ordering::Acquire)
    }

    /// Where the effective value came from.
    pub fn provenance(&self, key: Key) -> Provenance {
        Provenance::from_u8(self.at(key).provenance.load(Ordering::Acquire))
    }

    /// Apply a parsed config map, returning how many settings CHANGED value.
    ///
    /// Three cases per registered setting:
    /// - key present and parseable → the file value wins (provenance
    ///   `runtime-config`);
    /// - key absent, or present but unparseable → the boot value is restored
    ///   (provenance `boot-env`);
    /// - value equals what is already effective → nothing logged, nothing counted.
    ///
    /// Every actual change logs old → new at WARN, because a live behaviour flip
    /// on a running node is exactly the kind of thing that must be greppable
    /// after the fact.
    pub fn apply(&self, parsed: &BTreeMap<String, String>) -> usize {
        let mut changed = 0usize;
        for key in Key::ALL {
            let spec = &SPECS[key.index()];
            let s = self.at(key);

            let from_file = parsed.get(spec.name).and_then(|raw| {
                let parsed_value = spec.kind.parse(raw);
                if parsed_value.is_none() {
                    warn!(
                        setting = spec.name,
                        value = %raw,
                        kind = spec.kind.as_str(),
                        "runtime-config: unparseable value — keeping the boot value"
                    );
                }
                parsed_value
            });

            let (want, want_prov) = match from_file {
                Some(v) => (v, Provenance::RuntimeConfig),
                None => (s.boot.load(Ordering::Acquire), Provenance::BootEnv),
            };

            let old = s.current.load(Ordering::Acquire);
            let old_prov = Provenance::from_u8(s.provenance.load(Ordering::Acquire));

            if old != want {
                s.current.store(want, Ordering::Release);
                s.provenance.store(want_prov.to_u8(), Ordering::Release);
                changed += 1;
                warn!(
                    setting = spec.name,
                    old = %spec.kind.display(old),
                    new = %spec.kind.display(want),
                    provenance = want_prov.as_str(),
                    "runtime-config: setting changed on a RUNNING node"
                );
            } else if old_prov != want_prov {
                // Same value, different origin (e.g. the file names the boot
                // value, or stops naming it). Not a behaviour change, but the
                // provenance must stay truthful for the admin surface.
                s.provenance.store(want_prov.to_u8(), Ordering::Release);
            }
        }
        changed
    }

    /// Per-setting view for the admin route.
    pub fn snapshot(&self) -> Vec<serde_json::Value> {
        Key::ALL
            .iter()
            .map(|&key| {
                let spec = &SPECS[key.index()];
                serde_json::json!({
                    "name": spec.name,
                    "kind": spec.kind.as_str(),
                    "effectiveValue": spec.kind.render(self.get(key)),
                    "bootValue": spec.kind.render(self.boot(key)),
                    "defaultValue": spec.kind.render(spec.default),
                    "provenance": self.provenance(key).as_str(),
                    "hotReloadable": true,
                    "doc": spec.doc,
                    "note": spec.note,
                })
            })
            .collect()
    }
}

/// The process-wide registry every live read site consults.
static GLOBAL: LazyLock<Registry> = LazyLock::new(Registry::new);

/// The process-wide registry. Live code should prefer the free functions below.
pub fn global() -> &'static Registry {
    &GLOBAL
}

/// Publish a boot-env `bool` (called from `config::set_*`).
pub fn publish_boot_bool(key: Key, value: bool) {
    GLOBAL.publish_boot(key, u64::from(value));
}

/// Publish a boot-env seconds value (called from `config::set_*`).
pub fn publish_boot_secs(key: Key, value: u64) {
    GLOBAL.publish_boot(key, value);
}

/// The effective value of a [`Kind::Bool`] setting.
pub fn get_bool(key: Key) -> bool {
    GLOBAL.get_bool(key)
}

/// The effective value of a [`Kind::Seconds`] setting.
pub fn get_secs(key: Key) -> u64 {
    GLOBAL.get(key)
}

/// The cadence a RUNNING projection-reconcile loop should tick at.
///
/// `boot_secs` is the value the loop was spawned with (already known nonzero —
/// a boot 0 means the loop was never spawned). A runtime 0 is refused here
/// rather than at the call site: 0 means DISABLED, and disabling a loop that is
/// already running is not something a cadence knob may do silently.
pub fn projection_reconcile_secs_running(boot_secs: u64) -> u64 {
    let want = get_secs(Key::ProjectionReconcileSecs);
    if want == 0 {
        boot_secs
    } else {
        want
    }
}

// ─── parsing ─────────────────────────────────────────────────────────────────

/// Parse the hand-editable TOML subset into a key → raw-value map.
///
/// Accepts `KEY = "value"`, `KEY = 'value'` and bare `KEY = value`; ignores
/// blank lines, `#`/`;` comments, `[section]` headers, and trailing comments on
/// unquoted values. Unknown keys are carried through and dropped by
/// [`Registry::apply`] — an operator's note-to-self in the file is not an error.
pub fn parse(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || line.starts_with('[')
        {
            continue;
        }
        let Some((raw_key, raw_val)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim();
        if key.is_empty() {
            continue;
        }
        let val = raw_val.trim();
        let value = match val.chars().next() {
            Some(q @ ('"' | '\'')) => {
                let rest = &val[q.len_utf8()..];
                match rest.find(q) {
                    Some(end) => rest[..end].to_string(),
                    None => rest.to_string(),
                }
            }
            _ => match val.find('#') {
                Some(i) => val[..i].trim_end().to_string(),
                None => val.to_string(),
            },
        };
        map.insert(key.to_string(), value);
    }
    map
}

// ─── watcher ─────────────────────────────────────────────────────────────────

/// Cross-tick watcher state, so the admin route can report honestly whether the
/// watcher is live and what it last saw.
struct WatchState {
    /// Last observed (mtime, len) of the file — the change signal. `len` is
    /// carried alongside mtime because a coarse filesystem clock can hide a
    /// same-second edit that changes the file's size.
    last_signature: Mutex<Option<(SystemTime, u64)>>,
    last_reload_unix: AtomicU64,
    reload_count: AtomicU64,
    file_present: AtomicBool,
    watcher_running: AtomicBool,
    last_error: Mutex<Option<String>>,
}

static WATCH: LazyLock<WatchState> = LazyLock::new(|| WatchState {
    last_signature: Mutex::new(None),
    last_reload_unix: AtomicU64::new(0),
    reload_count: AtomicU64::new(0),
    file_present: AtomicBool::new(false),
    watcher_running: AtomicBool::new(false),
    last_error: Mutex::new(None),
});

/// The watched path, or `None` when the watcher is disabled.
pub fn config_path() -> Option<PathBuf> {
    std::env::var(PATH_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Outcome of one re-read.
#[derive(Debug, Clone)]
pub struct ReloadOutcome {
    /// The path read, or `None` when the watcher is disabled.
    pub path: Option<PathBuf>,
    /// Whether the file existed and was readable.
    pub file_present: bool,
    /// How many registered settings changed value.
    pub changed: usize,
    /// How many keys the file named (including unknown ones).
    pub keys_seen: usize,
    /// Read/IO failure, if any. A missing file is NOT an error — it is the
    /// "no overrides" state, and it correctly restores every boot value.
    pub error: Option<String>,
}

impl ReloadOutcome {
    /// JSON body for the reload route.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path.as_ref().map(|p| p.display().to_string()),
            "filePresent": self.file_present,
            "changed": self.changed,
            "keysSeen": self.keys_seen,
            "error": self.error,
        })
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Re-read the watched file NOW and apply it to the global registry.
///
/// Idempotent and safe to call at any time. With the watcher disabled this is a
/// no-op that reports `path: None` — it does NOT fall back to reverting
/// anything, because with no file there is nothing that could have overridden.
pub fn reload_now() -> ReloadOutcome {
    let Some(path) = config_path() else {
        return ReloadOutcome {
            path: None,
            file_present: false,
            changed: 0,
            keys_seen: 0,
            error: None,
        };
    };

    let (parsed, present, error) = match std::fs::read_to_string(&path) {
        Ok(text) => (parse(&text), true, None),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (BTreeMap::new(), false, None),
        Err(e) => (BTreeMap::new(), false, Some(e.to_string())),
    };

    // A read FAILURE (not a missing file) must not be read as "the operator
    // removed every override" — leave the registry exactly as it is.
    if let Some(err) = error {
        warn!(path = %path.display(), error = %err, "runtime-config: read failed — leaving current values in force");
        *WATCH.last_error.lock().unwrap() = Some(err.clone());
        WATCH.file_present.store(false, Ordering::Release);
        return ReloadOutcome {
            path: Some(path),
            file_present: false,
            changed: 0,
            keys_seen: 0,
            error: Some(err),
        };
    }

    let keys_seen = parsed.len();
    let changed = GLOBAL.apply(&parsed) + apply_text(&parsed);
    WATCH.file_present.store(present, Ordering::Release);
    WATCH.last_reload_unix.store(now_unix(), Ordering::Release);
    WATCH.reload_count.fetch_add(1, Ordering::AcqRel);
    *WATCH.last_error.lock().unwrap() = None;

    ReloadOutcome {
        path: Some(path),
        file_present: present,
        changed,
        keys_seen,
        error: None,
    }
}

/// Current (mtime, len) signature of the watched file, or `None` when absent.
fn signature(path: &std::path::Path) -> Option<(SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Spawn the config watcher. Returns whether it is active.
///
/// Disabled (and one INFO line) when [`PATH_ENV`] is unset — the default, so a
/// node that never mounts a ConfigMap pays nothing and behaves exactly as it
/// did before this module existed.
pub fn spawn_watcher() -> bool {
    let Some(path) = config_path() else {
        info!(
            env = PATH_ENV,
            "runtime-config: watcher DISABLED (no path configured) — every flag stays at its \
             boot-env value for the life of the process"
        );
        return false;
    };

    // Boot read: apply whatever the file already says before the first tick, so
    // a pod that starts with a populated ConfigMap does not spend a poll
    // interval running the boot-env values.
    let boot = reload_now();
    info!(
        path = %path.display(),
        poll_secs = POLL_INTERVAL_SECS,
        file_present = boot.file_present,
        applied = boot.changed,
        "runtime-config: watcher ACTIVE — flag flips apply to this RUNNING node without a restart"
    );
    *WATCH.last_signature.lock().unwrap() = signature(&path);
    WATCH.watcher_running.store(true, Ordering::Release);

    tokio::spawn(async move {
        // bounded-work: one `stat(2)` per POLL_INTERVAL_SECS tick, and a read +
        // parse of a single operator-authored file ONLY when its (mtime, len)
        // signature changed. No retry ladder, no queue, no fan-out: the budget
        // is the fixed cadence itself, and `MissedTickBehavior::Skip` means a
        // stalled runtime coalesces missed ticks instead of catching up in a
        // burst. There is nothing here to pace — this is the poll, not a drain.
        let mut ticker = tokio::time::interval(Duration::from_secs(POLL_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let current = signature(&path);
            let changed = {
                let mut last = WATCH.last_signature.lock().unwrap();
                if *last == current {
                    false
                } else {
                    *last = current;
                    true
                }
            };
            if changed {
                let outcome = reload_now();
                info!(
                    path = %path.display(),
                    file_present = outcome.file_present,
                    keys_seen = outcome.keys_seen,
                    applied = outcome.changed,
                    "runtime-config: file change detected"
                );
            }
        }
    });

    true
}

// ─── admin surface ───────────────────────────────────────────────────────────

/// JSON body for `GET /admin/runtime-config`.
pub fn report_json() -> serde_json::Value {
    let path = config_path();
    let last_reload = WATCH.last_reload_unix.load(Ordering::Acquire);
    serde_json::json!({
        "watcher": {
            "active": WATCH.watcher_running.load(Ordering::Acquire),
            "path": path.as_ref().map(|p| p.display().to_string()),
            "pathEnv": PATH_ENV,
            "pollSecs": POLL_INTERVAL_SECS,
            "filePresent": WATCH.file_present.load(Ordering::Acquire),
            "reloadCount": WATCH.reload_count.load(Ordering::Acquire),
            "lastReloadUnixSecs": if last_reload == 0 { None } else { Some(last_reload) },
            "lastError": WATCH.last_error.lock().unwrap().clone(),
        },
        "settings": GLOBAL.snapshot(),
        "textSettings": text_snapshot(),
        "bootOnly": BOOT_ONLY.iter().map(|f| serde_json::json!({
            "name": f.name,
            "reason": f.reason,
            "hotReloadable": false,
        })).collect::<Vec<_>>(),
    })
}

/// JSON body for `POST /admin/runtime-config/reload` — the forced re-read plus
/// the same report, so one call answers "did it land?" without a second GET.
pub fn reload_json() -> serde_json::Value {
    let outcome = reload_now();
    serde_json::json!({
        "reload": outcome.to_json(),
        "report": report_json(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every test drives its OWN Registry. The global one is what live sweeps
    // read; mutating it from a parallel test is the flake class this crate has
    // already paid for once with env vars.

    #[test]
    fn parse_handles_the_hand_editable_subset() {
        let text = r#"
# a comment
; another comment
[section]

ELOHIM_OBEY_CARRIED_ELECTION = "1"
CONTEST_BACKOFF_SECONDS = 600
HEAL_MISSING_BACKOFF_SECONDS = 42 # trailing comment
QUOTED_SINGLE = 'yes'
SPACED   =    padded
EMPTY_VALUE =
= orphan
not a pair
"#;
        let m = parse(text);
        assert_eq!(m.get("ELOHIM_OBEY_CARRIED_ELECTION").unwrap(), "1");
        assert_eq!(m.get("CONTEST_BACKOFF_SECONDS").unwrap(), "600");
        assert_eq!(m.get("HEAL_MISSING_BACKOFF_SECONDS").unwrap(), "42");
        assert_eq!(m.get("QUOTED_SINGLE").unwrap(), "yes");
        assert_eq!(m.get("SPACED").unwrap(), "padded");
        assert_eq!(m.get("EMPTY_VALUE").unwrap(), "");
        assert!(!m.contains_key(""), "an orphan '=' must not register a key");
        assert!(!m.contains_key("not a pair"));
        assert!(!m.contains_key("[section]"));
    }

    #[test]
    fn parse_round_trips_a_rendered_file() {
        // Render the shape an operator would write, parse it back, and assert
        // every registered setting survives the trip in the registry's terms.
        let mut text = String::from("# generated\n");
        for key in Key::ALL {
            let spec = &SPECS[key.index()];
            let raw = match spec.kind {
                Kind::Bool => "true".to_string(),
                Kind::Seconds => "77".to_string(),
            };
            text.push_str(&format!("{} = \"{}\"\n", spec.name, raw));
        }
        let parsed = parse(&text);
        assert_eq!(parsed.len(), SPECS.len());

        let reg = Registry::new();
        let changed = reg.apply(&parsed);
        assert_eq!(changed, SPECS.len(), "every setting moved off its default");
        for key in Key::ALL {
            let spec = &SPECS[key.index()];
            let want = match spec.kind {
                Kind::Bool => 1,
                Kind::Seconds => 77,
            };
            assert_eq!(reg.get(key), want, "{} did not round-trip", spec.name);
            assert_eq!(reg.provenance(key), Provenance::RuntimeConfig);
        }
    }

    #[test]
    fn file_value_overrides_boot_and_removal_restores_it() {
        let reg = Registry::new();
        reg.publish_boot(Key::ObeyCarriedElection, 0);
        reg.publish_boot(Key::ContestBackoffSeconds, 3600);
        assert!(!reg.get_bool(Key::ObeyCarriedElection));
        assert_eq!(
            reg.provenance(Key::ObeyCarriedElection),
            Provenance::BootEnv
        );

        // File wins.
        let on = parse("ELOHIM_OBEY_CARRIED_ELECTION = \"1\"\nCONTEST_BACKOFF_SECONDS = 60\n");
        assert_eq!(reg.apply(&on), 2);
        assert!(reg.get_bool(Key::ObeyCarriedElection));
        assert_eq!(reg.get(Key::ContestBackoffSeconds), 60);
        assert_eq!(
            reg.provenance(Key::ObeyCarriedElection),
            Provenance::RuntimeConfig
        );
        // The boot value is remembered, not overwritten.
        assert_eq!(reg.boot(Key::ObeyCarriedElection), 0);
        assert_eq!(reg.boot(Key::ContestBackoffSeconds), 3600);

        // Key REMOVED from the file → boot value returns.
        let off = parse("CONTEST_BACKOFF_SECONDS = 60\n");
        assert_eq!(reg.apply(&off), 1);
        assert!(!reg.get_bool(Key::ObeyCarriedElection));
        assert_eq!(
            reg.provenance(Key::ObeyCarriedElection),
            Provenance::BootEnv
        );
        // The still-named key keeps its override.
        assert_eq!(reg.get(Key::ContestBackoffSeconds), 60);
        assert_eq!(
            reg.provenance(Key::ContestBackoffSeconds),
            Provenance::RuntimeConfig
        );

        // Empty file → everything back to boot.
        assert_eq!(reg.apply(&parse("")), 1);
        assert_eq!(reg.get(Key::ContestBackoffSeconds), 3600);
        assert_eq!(
            reg.provenance(Key::ContestBackoffSeconds),
            Provenance::BootEnv
        );
    }

    #[test]
    fn publish_boot_after_an_override_moves_the_fallback_not_the_value() {
        let reg = Registry::new();
        reg.apply(&parse("HEAL_MISSING_BACKOFF_SECONDS = 15\n"));
        assert_eq!(reg.get(Key::HealMissingBackoffSeconds), 15);

        // Boot publication racing in AFTER the watcher must not clobber the
        // live override — it only changes what removal falls back to.
        reg.publish_boot(Key::HealMissingBackoffSeconds, 900);
        assert_eq!(reg.get(Key::HealMissingBackoffSeconds), 15);
        assert_eq!(reg.boot(Key::HealMissingBackoffSeconds), 900);

        reg.apply(&parse(""));
        assert_eq!(reg.get(Key::HealMissingBackoffSeconds), 900);
    }

    #[test]
    fn unparseable_value_keeps_the_boot_value() {
        let reg = Registry::new();
        reg.publish_boot(Key::ContestBackoffSeconds, 3600);
        reg.publish_boot(Key::AdoptBeforeAuthor, 0);

        let bad = parse("CONTEST_BACKOFF_SECONDS = \"soon\"\nELOHIM_ADOPT_BEFORE_AUTHOR = maybe\n");
        assert_eq!(reg.apply(&bad), 0, "nothing may change on a bad value");
        assert_eq!(reg.get(Key::ContestBackoffSeconds), 3600);
        assert!(!reg.get_bool(Key::AdoptBeforeAuthor));
        assert_eq!(
            reg.provenance(Key::ContestBackoffSeconds),
            Provenance::BootEnv,
            "an unparseable value must not claim runtime-config provenance"
        );
    }

    #[test]
    fn unknown_keys_are_ignored_and_zero_is_a_real_value() {
        let reg = Registry::new();
        reg.publish_boot(Key::ContestBackoffSeconds, 3600);
        let m = parse("SOMETHING_ELSE = 1\nCONTEST_BACKOFF_SECONDS = 0\n");
        assert_eq!(reg.apply(&m), 1);
        // 0 is the documented OFF value for this window, NOT "unset".
        assert_eq!(reg.get(Key::ContestBackoffSeconds), 0);
        assert_eq!(
            reg.provenance(Key::ContestBackoffSeconds),
            Provenance::RuntimeConfig
        );
    }

    #[test]
    fn bool_truthiness_matches_the_boot_env_vocabulary() {
        for on in ["1", "true", "TRUE", "yes", "on"] {
            assert_eq!(Kind::Bool.parse(on), Some(1), "{on} should be truthy");
        }
        for off in ["0", "false", "No", "OFF"] {
            assert_eq!(Kind::Bool.parse(off), Some(0), "{off} should be falsy");
        }
        assert_eq!(Kind::Bool.parse("perhaps"), None);
    }

    #[test]
    fn reload_applies_a_temp_file_to_a_registry() {
        // The file→atomics path end-to-end, without touching the global
        // registry or the process environment: read the temp file with the same
        // reader `reload_now` uses, then apply to a local registry.
        let dir = std::env::temp_dir().join(format!(
            "elohim-runtime-config-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("runtime-config.toml");

        std::fs::write(
            &path,
            "# live edit\nELOHIM_OBEY_CARRIED_ELECTION = \"1\"\nPROJECTION_RECONCILE_SECS = 30\n",
        )
        .unwrap();

        let reg = Registry::new();
        reg.publish_boot(Key::ObeyCarriedElection, 0);
        reg.publish_boot(Key::ProjectionReconcileSecs, 300);

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(reg.apply(&parse(&text)), 2);
        assert!(reg.get_bool(Key::ObeyCarriedElection));
        assert_eq!(reg.get(Key::ProjectionReconcileSecs), 30);

        // Operator edits the file again — the flip is observable with no restart.
        std::fs::write(&path, "PROJECTION_RECONCILE_SECS = 300\n").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(reg.apply(&parse(&text)), 2, "flag reverted + cadence moved");
        assert!(!reg.get_bool(Key::ObeyCarriedElection));
        assert_eq!(
            reg.get(Key::ObeyCarriedElection),
            reg.boot(Key::ObeyCarriedElection)
        );
        assert_eq!(reg.get(Key::ProjectionReconcileSecs), 300);
        assert_eq!(
            reg.provenance(Key::ProjectionReconcileSecs),
            Provenance::RuntimeConfig,
            "naming the boot value in the file is still runtime-config provenance"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_running_reconcile_loop_refuses_a_runtime_zero_cadence() {
        // Guards the one caveat the admin surface advertises: 0 means DISABLED,
        // and a cadence knob may not stop a loop that is already running.
        let reg = Registry::new();
        reg.publish_boot(Key::ProjectionReconcileSecs, 300);
        reg.apply(&parse("PROJECTION_RECONCILE_SECS = 0\n"));
        assert_eq!(reg.get(Key::ProjectionReconcileSecs), 0);

        // The clamp itself is pure over the registry value; assert its rule
        // directly so this holds regardless of the global registry's state.
        let running = |want: u64, boot: u64| if want == 0 { boot } else { want };
        assert_eq!(running(reg.get(Key::ProjectionReconcileSecs), 300), 300);
        assert_eq!(running(30, 300), 30);
    }

    #[test]
    fn disabled_watcher_reload_is_an_honest_noop() {
        // With no path configured, a forced reload reverts nothing and reports
        // no path — it must never be read as "the file removed every override".
        if config_path().is_none() {
            let outcome = reload_now();
            assert!(outcome.path.is_none());
            assert!(!outcome.file_present);
            assert_eq!(outcome.changed, 0);
            assert!(outcome.error.is_none());
        }
    }

    #[test]
    fn report_json_names_every_setting_and_its_provenance() {
        let body = report_json();
        let settings = body["settings"].as_array().expect("settings array");
        assert_eq!(settings.len(), SPECS.len());
        for (entry, spec) in settings.iter().zip(SPECS.iter()) {
            assert_eq!(entry["name"], spec.name);
            assert_eq!(entry["hotReloadable"], true);
            let prov = entry["provenance"].as_str().unwrap();
            assert!(prov == "boot-env" || prov == "runtime-config");
        }
        let boot_only = body["bootOnly"].as_array().expect("bootOnly array");
        assert_eq!(boot_only.len(), BOOT_ONLY.len());
        assert!(boot_only
            .iter()
            .all(|f| f["hotReloadable"] == false && !f["reason"].as_str().unwrap().is_empty()));
        assert_eq!(body["watcher"]["pathEnv"], PATH_ENV);
        assert_eq!(body["watcher"]["pollSecs"], POLL_INTERVAL_SECS);
    }
}
