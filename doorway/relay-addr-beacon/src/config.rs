//! Command-line / environment configuration.
//!
//! Precedence is `flag > env > default`, which is exactly clap's native
//! behaviour: an explicit flag wins, otherwise the `env` fallback is consulted,
//! otherwise the declared default is used.

use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};

/// One sibling-safe shared DNS record lane contributed by this beacon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedRecordLane {
    pub record_name: String,
    pub owner: String,
}

impl SharedRecordLane {
    /// The lane's identity for lookup and duplicate detection. DNS names are
    /// case-insensitive and a terminal dot is equivalent, so both aliases
    /// normalize to one key — a lane cannot be configured twice under two
    /// spellings, and a per-lane reconcile can find its lane by either.
    pub fn key(&self) -> String {
        lane_key(&self.record_name)
    }
}

/// Normalize a shared record name to its lane key (see
/// [`SharedRecordLane::key`]).
pub fn lane_key(record_name: &str) -> String {
    record_name.trim_end_matches('.').to_ascii_lowercase()
}

/// The `{name}` placeholder an operator may put anywhere inside
/// `--membership-file` to say where each lane's public name lands.
pub const MEMBERSHIP_NAME_PLACEHOLDER: &str = "{name}";

/// Derive ONE lane's membership document path from the configured
/// `--membership-file`. One document names one public name, so a beacon leg
/// contributing to several lanes writes several documents. Three shapes, in
/// order:
///
/// 1. the configured path contains `{name}` — substituted with the lane's
///    public name. Explicit and operator-controlled; works at any lane count.
/// 2. the configured path is directory-shaped (a trailing `/`, or it already
///    exists as a directory) — the lane lands at `<dir>/<public-name>.json`.
/// 3. otherwise it is a plain file path: used VERBATIM for a single lane (so
///    every deployed single-lane invocation stays byte-identical), and
///    REFUSED for two or more — deriving sibling documents beside a file the
///    operator already named would leave that name meaning nothing, and
///    silently writing one lane there and inventing paths for the rest is
///    exactly the last-value-wins class this lane work exists to remove.
pub fn membership_path_for(
    base: &std::path::Path,
    record_name: &str,
    lane_count: usize,
) -> Result<PathBuf> {
    // A terminal dot is DNS-equivalent but not wanted in a file name.
    let public_name = record_name.trim_end_matches('.');
    let raw = base.to_string_lossy();
    if raw.contains(MEMBERSHIP_NAME_PLACEHOLDER) {
        return Ok(PathBuf::from(
            raw.replace(MEMBERSHIP_NAME_PLACEHOLDER, public_name),
        ));
    }
    if raw.ends_with('/') || base.is_dir() {
        return Ok(base.join(format!("{public_name}.json")));
    }
    if lane_count <= 1 {
        return Ok(base.to_path_buf());
    }
    Err(anyhow!(
        "--membership-file {} names ONE document but {lane_count} shared lanes are configured — \
         one membership document names one public name. Pass a directory (give the path a \
         trailing `/`) or put the `{MEMBERSHIP_NAME_PLACEHOLDER}` placeholder in the path, so \
         every lane gets its own document.",
        base.display(),
    ))
}

/// The sinks a single beacon process can drive. `--sink` is repeatable and the
/// enabled sinks compose (all run on every publish).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum SinkName {
    /// Tier-1: Cloudflare A/AAAA upsert (proxied=false — UDP can't be proxied).
    Cloudflare,
    /// Tier-2: pkarr signed-packet PUT to a relay (published for a future
    /// resolution bridge; ICE cannot consume pkarr names yet).
    Pkarr,
    /// Local: render coturn's `external-ip=<wan>/<lan>` and optionally reload.
    Coturn,
    /// Household-ownable: project shared membership (the set of origins
    /// eligible to serve the public name) into a JSON document this household
    /// owns, instead of into a DNS zone it does not. Same reconcile loop, same
    /// join/leave hysteresis, same serving probe as the Cloudflare shared lane.
    File,
}

/// relay-addr-beacon — publish a relay's dynamic WAN IP to DNS / pkarr / coturn.
///
/// `Debug` is implemented manually (below) so the Cloudflare token fields render
/// as `<redacted>` — a derived `Debug` would leak the bearer token into any
/// `debug!("{cfg:?}")`. All non-secret fields stay visible.
#[derive(Parser)]
#[command(name = "relay-addr-beacon", version, about, long_about = None)]
pub struct Config {
    /// Poll interval in seconds for the detect->publish loop.
    #[arg(long, env = "BEACON_INTERVAL_SECS", default_value_t = 30)]
    pub interval_secs: u64,

    /// Detect + run every enabled sink exactly once, then exit. Ideal for use
    /// as a Kubernetes initContainer. Runs sinks regardless of change state.
    #[arg(long, default_value_t = false)]
    pub once: bool,

    /// Also detect and publish the public IPv6 address (AAAA / pkarr aaaa).
    /// OFF by default — this phase is v4-first.
    #[arg(long, env = "BEACON_ENABLE_V6", default_value_t = false)]
    pub enable_v6: bool,

    /// DNS record name published by the Cloudflare sink (e.g. `turn.elohim.host`).
    /// Not required by the pkarr sink (it publishes at the signer's apex).
    #[arg(long, env = "BEACON_RECORD_NAME")]
    pub record_name: Option<String>,

    /// Override the detected primary LAN IPv4 used for coturn's external-ip
    /// `<wan>/<lan>` mapping. If unset, the beacon auto-detects it.
    #[arg(long, env = "BEACON_LAN_IP")]
    pub lan_ip: Option<Ipv4Addr>,

    /// File where the last-published address snapshot is persisted so the
    /// beacon can detect a change across restarts.
    #[arg(
        long,
        env = "BEACON_STATE_FILE",
        default_value = "/var/lib/relay-addr-beacon/state.json"
    )]
    pub state_file: PathBuf,

    /// Enable a sink (repeatable). At least one is required.
    #[arg(long = "sink", value_enum)]
    pub sinks: Vec<SinkName>,

    /// Override the ordered list of HTTP echo endpoints used to discover the
    /// public egress IP. First success wins. Comma-separated or repeated.
    #[arg(
        long = "egress-endpoint",
        env = "BEACON_EGRESS_ENDPOINTS",
        value_delimiter = ','
    )]
    pub egress_endpoints: Vec<String>,

    // ---- Cloudflare sink ------------------------------------------------
    /// Cloudflare zone name that owns the record (e.g. `elohim.host`).
    #[arg(long, env = "CF_ZONE")]
    pub cf_zone: Option<String>,

    /// Cloudflare API token (Bearer). Prefer `CF_API_TOKEN` env or a file.
    #[arg(long, env = "CF_API_TOKEN")]
    pub cf_token: Option<String>,

    /// Path to a file containing the Cloudflare API token (trailing whitespace
    /// trimmed). Used if `--cf-token`/`CF_API_TOKEN` is not set.
    #[arg(long, env = "CF_API_TOKEN_FILE")]
    pub cf_token_file: Option<PathBuf>,

    /// Shared "logical anycast" DNS lane as `<record-name>=<owner>`. Repeat the
    /// flag to contribute this beacon's address to more than one shared name.
    /// The environment form is comma-separated.
    #[arg(
        long = "shared-record",
        env = "BEACON_SHARED_RECORDS",
        value_delimiter = ',',
        action = clap::ArgAction::Append,
        value_name = "NAME=OWNER"
    )]
    pub shared_records: Vec<String>,

    /// Legacy single shared-record name. Prefer repeatable `--shared-record
    /// <name>=<owner>` for new configuration. Retained so deployed manifests
    /// continue to parse unchanged.
    #[arg(long, env = "BEACON_SHARED_RECORD_NAME")]
    pub shared_record_name: Option<String>,

    /// Legacy owner paired with `--shared-record-name`. Prefer the repeatable
    /// `--shared-record <name>=<owner>` spelling for new configuration.
    #[arg(long, env = "BEACON_RECORD_OWNER")]
    pub record_owner: Option<String>,

    /// Max age (seconds) of our OWN freshness stamp on the shared record before
    /// we re-PATCH it even though our IP is unchanged. Keeps our record from
    /// ever looking abandoned to a sibling.
    #[arg(long, env = "BEACON_SHARED_REFRESH_SECS", default_value_t = 300)]
    pub shared_refresh_secs: u64,

    /// Age (seconds) beyond which a SIBLING's shared record is considered
    /// abandoned and reaped (DELETEd). Must be greater than
    /// `--shared-refresh-secs`, else a live sibling could be reaped before its
    /// own refresh cycle runs.
    #[arg(long, env = "BEACON_SHARED_STALE_SECS", default_value_t = 900)]
    pub shared_stale_secs: u64,

    /// Opt-in doorway serving contract. Only HTTP 200 serves; redirects,
    /// errors and silence do not. Body parsing is deliberately unnecessary.
    /// Shared lanes start withdrawn; exclusive diagnostic names are unaffected.
    #[arg(long, env = "BEACON_SERVING_PROBE_URL")]
    pub serving_probe_url: Option<String>,

    /// Independent health polling interval (WAN discovery cannot block it).
    #[arg(long, env = "BEACON_SERVING_PROBE_INTERVAL_SECS", default_value_t = 15)]
    pub serving_probe_interval_secs: u64,

    /// Consecutive non-serving probes required to withdraw shared membership.
    #[arg(long, env = "BEACON_SERVING_LEAVE_AFTER", default_value_t = 3)]
    pub serving_leave_after: u64,

    /// Consecutive serving probes required to join, including after restart.
    #[arg(long, env = "BEACON_SERVING_JOIN_AFTER", default_value_t = 2)]
    pub serving_join_after: u64,

    // ---- file membership sink -------------------------------------------
    /// Membership document this beacon leg contributes its own entry to
    /// (`--sink file`). Sibling legs share the path and each owns exactly one
    /// entry, keyed by `--record-owner` / the owner half of `--shared-record`.
    #[arg(long, env = "BEACON_MEMBERSHIP_FILE")]
    pub membership_file: Option<PathBuf>,

    /// The origin (`scheme://host:port`) this leg advertises as eligible to
    /// serve the public name. Required by `--sink file`, because a household's
    /// doorways can differ by PORT on one host rather than by IP — an address
    /// snapshot cannot tell them apart, and a membership entry must.
    #[arg(long, env = "BEACON_MEMBER_ORIGIN")]
    pub member_origin: Option<String>,

    // ---- pkarr sink -----------------------------------------------------
    /// Dedicated pkarr secret-key file (hex, 0600). Generated if absent. Do NOT
    /// reuse an iroh/libp2p key here.
    #[arg(
        long,
        env = "PKARR_KEY_FILE",
        default_value = "/var/lib/relay-addr-beacon/pkarr.key"
    )]
    pub pkarr_key_file: PathBuf,

    /// pkarr relay endpoint including the `/pkarr` path segment. The signer's
    /// z-base-32 public key is appended: `{relay}/{z32}`.
    #[arg(long, env = "PKARR_RELAY", default_value = "https://elohim.host/pkarr")]
    pub pkarr_relay: String,

    // ---- coturn sink ----------------------------------------------------
    /// Base coturn config that the rendered config is built from (copied
    /// verbatim, then an `external-ip=` line is appended).
    #[arg(long, env = "COTURN_BASE_CONF")]
    pub coturn_base_conf: Option<PathBuf>,

    /// Output path for the rendered coturn config (base + external-ip line).
    #[arg(long, env = "COTURN_OUT_CONF")]
    pub coturn_out_conf: Option<PathBuf>,

    /// Command run (via `sh -c`) after the coturn config changes. coturn does
    /// NOT hot-reload external-ip on SIGHUP, so this should RESTART coturn.
    /// Fully operator-configurable — no reload mechanism is hardcoded.
    #[arg(long, env = "COTURN_ON_CHANGE_EXEC")]
    pub on_change_exec: Option<String>,
}

impl Config {
    /// Resolve the preferred repeatable spelling and the legacy single pair
    /// into one ordered lane list. Parsing is deliberately deferred until
    /// validation so a missing owner produces the same cross-field error class
    /// as the legacy spelling.
    pub fn shared_record_lanes(&self) -> Result<Vec<SharedRecordLane>> {
        let mut lanes = Vec::with_capacity(
            self.shared_records.len() + usize::from(self.shared_record_name.is_some()),
        );

        for raw in &self.shared_records {
            let (record_name, owner) = raw.split_once('=').unwrap_or((raw.as_str(), ""));
            let record_name = record_name.trim();
            let owner = owner.trim();
            if record_name.is_empty() {
                return Err(anyhow!(
                    "--shared-record requires a non-empty record name; expected <name>=<owner>"
                ));
            }
            if owner.is_empty() {
                return Err(anyhow!(
                    "--shared-record {raw:?} requires an owner; expected <name>=<owner>"
                ));
            }
            // A lane's public name becomes a file name under the `file` sink's
            // per-lane document derivation, so a path separator in it is
            // refused here rather than allowed to escape the configured
            // directory. No DNS name contains one.
            if record_name.contains('/') || record_name.contains('\\') {
                return Err(anyhow!(
                    "--shared-record name {record_name:?} must be a DNS name (no path separators)"
                ));
            }
            lanes.push(SharedRecordLane {
                record_name: record_name.to_string(),
                owner: owner.to_string(),
            });
        }

        if let Some(record_name) = self.shared_record_name.as_deref() {
            let owner = self.record_owner.as_deref().ok_or_else(|| {
                anyhow!("--shared-record-name requires --record-owner / BEACON_RECORD_OWNER")
            })?;
            if record_name.trim().is_empty() || owner.trim().is_empty() {
                return Err(anyhow!(
                    "--shared-record-name and --record-owner must both be non-empty"
                ));
            }
            lanes.push(SharedRecordLane {
                record_name: record_name.trim().to_string(),
                owner: owner.trim().to_string(),
            });
        }

        let mut names = HashSet::with_capacity(lanes.len());
        for lane in &lanes {
            // DNS names are case-insensitive and a terminal dot is equivalent,
            // so reject those aliases as duplicates too.
            if !names.insert(lane.key()) {
                return Err(anyhow!(
                    "duplicate shared record lane name: {}",
                    lane.record_name
                ));
            }
        }

        Ok(lanes)
    }

    /// Every shared lane the file membership sink projects, each paired with
    /// its OWN document path (see [`membership_path_for`]). One document names
    /// ONE public name, so a leg contributing to two lanes writes two
    /// documents — never one document that silently means whichever lane was
    /// parsed last.
    pub fn membership_lanes(&self) -> Result<Vec<(SharedRecordLane, PathBuf)>> {
        let base = self.membership_file.as_deref().ok_or_else(|| {
            anyhow!("file sink requires --membership-file / BEACON_MEMBERSHIP_FILE")
        })?;
        let lanes = self.shared_record_lanes()?;
        if lanes.is_empty() {
            return Err(anyhow!(
                "file membership sink requires at least one shared lane \
                 (--shared-record <public-name>=<owner>, repeatable, or the legacy \
                 --shared-record-name/--record-owner pair); none configured"
            ));
        }
        let count = lanes.len();
        lanes
            .into_iter()
            .map(|lane| {
                let path = membership_path_for(base, &lane.record_name, count)?;
                Ok((lane, path))
            })
            .collect()
    }

    /// Cross-field validation clap's declarative attributes can't express.
    /// Called once at startup, independent of which sinks are enabled — these
    /// are shared-mode invariants, not per-sink construction concerns.
    pub fn validate(&self) -> Result<()> {
        self.shared_record_lanes()?;
        if self.serving_probe_interval_secs == 0
            || self.serving_leave_after == 0
            || self.serving_join_after == 0
        {
            return Err(anyhow!(
                "serving probe interval and join/leave counts must be positive"
            ));
        }
        if self.sinks.contains(&SinkName::File) {
            // Resolves every lane's document path too, so an ambiguous
            // multi-lane `--membership-file` is refused before launch rather
            // than at the first health tick.
            self.membership_lanes()?;
            let origin = self.member_origin.as_deref().ok_or_else(|| {
                anyhow!("file sink requires --member-origin / BEACON_MEMBER_ORIGIN")
            })?;
            let parsed = reqwest::Url::parse(origin).context("invalid member origin")?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err(anyhow!("member origin must use http or https"));
            }
            if self.serving_probe_url.is_none() {
                // Membership is EARNED from the serving probe. A file leg with
                // no probe would publish an entry it never re-verified — the
                // exact "DNS as evidence of serving" inversion the shared lane
                // exists to refuse.
                return Err(anyhow!(
                    "file sink requires --serving-probe-url / BEACON_SERVING_PROBE_URL — \
                     membership is earned from the serving probe, never assumed"
                ));
            }
        }
        if let Some(url) = &self.serving_probe_url {
            let lanes = self.shared_record_lanes()?;
            let projects_membership =
                self.sinks.contains(&SinkName::Cloudflare) || self.sinks.contains(&SinkName::File);
            if !projects_membership || lanes.is_empty() {
                return Err(anyhow!(
                    "serving probe requires a shared record lane projected by the cloudflare or file sink"
                ));
            }
            if lanes.iter().any(|lane| {
                self.record_name.as_deref().is_some_and(|name| {
                    name.trim_end_matches('.')
                        .eq_ignore_ascii_case(lane.record_name.trim_end_matches('.'))
                })
            }) {
                return Err(anyhow!(
                    "serving shared lanes must not own the exclusive diagnostic name"
                ));
            }

            let parsed = reqwest::Url::parse(url).context("invalid serving probe URL")?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err(anyhow!("serving probe URL must use http or https"));
            }
        }

        if self.shared_stale_secs <= self.shared_refresh_secs {
            return Err(anyhow!(
                "--shared-stale-secs ({}) must be greater than --shared-refresh-secs ({}) — \
                 otherwise a live sibling's stamp could go stale-reaped before its own refresh cycle runs",
                self.shared_stale_secs,
                self.shared_refresh_secs
            ));
        }
        Ok(())
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Preserve whether each token field is set (Some/None) while never
        // rendering the secret value itself.
        fn redact<T>(v: &Option<T>) -> Option<&'static str> {
            v.as_ref().map(|_| "<redacted>")
        }
        f.debug_struct("Config")
            .field("interval_secs", &self.interval_secs)
            .field("once", &self.once)
            .field("enable_v6", &self.enable_v6)
            .field("record_name", &self.record_name)
            .field("lan_ip", &self.lan_ip)
            .field("state_file", &self.state_file)
            .field("sinks", &self.sinks)
            .field("egress_endpoints", &self.egress_endpoints)
            .field("cf_zone", &self.cf_zone)
            .field("cf_token", &redact(&self.cf_token))
            .field("cf_token_file", &redact(&self.cf_token_file))
            .field("shared_records", &self.shared_records)
            .field("shared_record_name", &self.shared_record_name)
            .field("record_owner", &self.record_owner)
            .field("shared_refresh_secs", &self.shared_refresh_secs)
            .field("shared_stale_secs", &self.shared_stale_secs)
            .field("serving_probe_url", &self.serving_probe_url)
            .field(
                "serving_probe_interval_secs",
                &self.serving_probe_interval_secs,
            )
            .field("serving_leave_after", &self.serving_leave_after)
            .field("serving_join_after", &self.serving_join_after)
            .field("membership_file", &self.membership_file)
            .field("member_origin", &self.member_origin)
            .field("pkarr_key_file", &self.pkarr_key_file)
            .field("pkarr_relay", &self.pkarr_relay)
            .field("coturn_base_conf", &self.coturn_base_conf)
            .field("coturn_out_conf", &self.coturn_out_conf)
            .field("on_change_exec", &self.on_change_exec)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse(args: &[&str]) -> Config {
        Config::try_parse_from(std::iter::once("relay-addr-beacon").chain(args.iter().copied()))
            .expect("configuration should parse")
    }

    #[test]
    fn repeatable_shared_records_preserve_order() {
        let cfg = parse(&[
            "--shared-record",
            "doorways.elohim.host=operations",
            "--shared-record",
            "apex-canary.elohim.host=operations",
        ]);

        let lanes = cfg.shared_record_lanes().unwrap();
        assert_eq!(
            lanes,
            vec![
                SharedRecordLane {
                    record_name: "doorways.elohim.host".to_string(),
                    owner: "operations".to_string(),
                },
                SharedRecordLane {
                    record_name: "apex-canary.elohim.host".to_string(),
                    owner: "operations".to_string(),
                },
            ]
        );
    }

    #[test]
    fn legacy_shared_record_pair_resolves_to_one_lane() {
        let cfg = parse(&[
            "--shared-record-name",
            "doorways.elohim.host",
            "--record-owner",
            "shem",
        ]);

        assert_eq!(
            cfg.shared_record_lanes().unwrap(),
            vec![SharedRecordLane {
                record_name: "doorways.elohim.host".to_string(),
                owner: "shem".to_string(),
            }]
        );
    }

    #[test]
    fn shared_record_without_owner_fails_validation() {
        let cfg = parse(&["--shared-record", "doorways.elohim.host"]);

        let error = cfg.validate().unwrap_err().to_string();
        assert!(
            error.contains("requires an owner"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn file_sink_requires_a_document_an_origin_a_probe_and_a_lane() {
        let complete = [
            "--sink",
            "file",
            "--shared-record",
            "elohim.local=alpha",
            "--membership-file",
            "/tmp/beacon-config-test/elohim.local.json",
            "--member-origin",
            "http://localhost:8888",
            "--serving-probe-url",
            "http://localhost:8888/health",
        ];
        parse(&complete).validate().expect("complete file leg");

        for (drop_flag, expected) in [
            ("--membership-file", "requires --membership-file"),
            ("--member-origin", "requires --member-origin"),
            ("--serving-probe-url", "requires --serving-probe-url"),
        ] {
            let mut args: Vec<&str> = Vec::new();
            let mut skip = false;
            for arg in complete {
                if skip {
                    skip = false;
                    continue;
                }
                if arg == drop_flag {
                    skip = true;
                    continue;
                }
                args.push(arg);
            }
            let error = parse(&args).validate().unwrap_err().to_string();
            assert!(error.contains(expected), "unexpected error: {error}");
        }

        // A lane is required: membership needs a public name to be a member of.
        let no_lane = parse(&[
            "--sink",
            "file",
            "--membership-file",
            "/tmp/beacon-config-test/elohim.local.json",
            "--member-origin",
            "http://localhost:8888",
            "--serving-probe-url",
            "http://localhost:8888/health",
        ]);
        let error = no_lane.validate().unwrap_err().to_string();
        assert!(
            error.contains("at least one shared lane"),
            "unexpected error: {error}"
        );

        // A non-HTTP origin is not an origin a client can be told to try.
        let bad_origin = parse(&[
            "--sink",
            "file",
            "--shared-record",
            "elohim.local=alpha",
            "--membership-file",
            "/tmp/beacon-config-test/elohim.local.json",
            "--member-origin",
            "ftp://localhost:8888",
            "--serving-probe-url",
            "http://localhost:8888/health",
        ]);
        let error = bad_origin.validate().unwrap_err().to_string();
        assert!(
            error.contains("member origin must use http or https"),
            "unexpected error: {error}"
        );
    }

    /// One document names ONE public name, so two lanes pointed at one named
    /// file is refused before launch — with both unambiguous spellings named.
    #[test]
    fn two_lanes_into_one_named_document_are_refused_with_the_fix_named() {
        let cfg = parse(&[
            "--sink",
            "file",
            "--shared-record",
            "elohim.local=alpha",
            "--shared-record",
            "doorways.elohim.local=alpha",
            "--membership-file",
            "/tmp/beacon-config-test/elohim.local.json",
            "--member-origin",
            "http://localhost:8888",
            "--serving-probe-url",
            "http://localhost:8888/health",
        ]);
        let error = cfg.validate().unwrap_err().to_string();
        assert!(
            error.contains("names ONE document but 2 shared lanes are configured"),
            "unexpected error: {error}"
        );
        assert!(error.contains("trailing `/`"), "unexpected error: {error}");
        assert!(error.contains("{name}"), "unexpected error: {error}");
    }

    #[test]
    fn membership_documents_are_derived_per_lane() {
        let two = [
            "--sink",
            "file",
            "--shared-record",
            "elohim.host=shem",
            "--shared-record",
            "doorways.elohim.host=shem",
            "--member-origin",
            "https://doorway.elohim.host",
            "--serving-probe-url",
            "https://doorway.elohim.host/health",
        ];

        // Directory-shaped (trailing `/`): one document per public name.
        let mut args = two.to_vec();
        args.extend(["--membership-file", "/var/lib/beacon/membership/"]);
        let cfg = parse(&args);
        cfg.validate()
            .expect("directory-shaped path is unambiguous");
        let paths: Vec<PathBuf> = cfg
            .membership_lanes()
            .unwrap()
            .into_iter()
            .map(|(_, path)| path)
            .collect();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("/var/lib/beacon/membership/elohim.host.json"),
                PathBuf::from("/var/lib/beacon/membership/doorways.elohim.host.json"),
            ]
        );

        // `{name}` placeholder: substituted wherever it appears.
        let mut args = two.to_vec();
        args.extend(["--membership-file", "/var/lib/beacon/{name}/members.json"]);
        let cfg = parse(&args);
        cfg.validate().expect("placeholder path is unambiguous");
        let paths: Vec<PathBuf> = cfg
            .membership_lanes()
            .unwrap()
            .into_iter()
            .map(|(_, path)| path)
            .collect();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("/var/lib/beacon/elohim.host/members.json"),
                PathBuf::from("/var/lib/beacon/doorways.elohim.host/members.json"),
            ]
        );

        // BACKWARD COMPATIBILITY: one lane, a plain file path — used verbatim,
        // exactly as every deployed single-lane invocation already does.
        let cfg = parse(&[
            "--sink",
            "file",
            "--shared-record",
            "elohim.local=alpha",
            "--membership-file",
            "/tmp/elohim-local-mesh/membership/elohim.local.json",
            "--member-origin",
            "http://localhost:8888",
            "--serving-probe-url",
            "http://localhost:8888/health",
        ]);
        cfg.validate().expect("single-lane file leg");
        let lanes = cfg.membership_lanes().unwrap();
        assert_eq!(lanes.len(), 1);
        assert_eq!(
            lanes[0].1,
            PathBuf::from("/tmp/elohim-local-mesh/membership/elohim.local.json")
        );
        // A terminal dot is DNS-equivalent but never reaches a file name.
        assert_eq!(
            membership_path_for(Path::new("/var/lib/beacon/"), "elohim.host.", 2).unwrap(),
            PathBuf::from("/var/lib/beacon/elohim.host.json")
        );
    }

    #[test]
    fn a_lane_name_with_a_path_separator_is_refused() {
        let cfg = parse(&["--shared-record", "../../etc/passwd=alpha"]);
        let error = cfg.validate().unwrap_err().to_string();
        assert!(
            error.contains("must be a DNS name"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn duplicate_shared_record_names_fail_validation() {
        let cfg = parse(&[
            "--shared-record",
            "doorways.elohim.host=operations",
            "--shared-record",
            "DOORWAYS.ELOHIM.HOST.=shem",
        ]);

        let error = cfg.validate().unwrap_err().to_string();
        assert!(
            error.contains("duplicate shared record lane name"),
            "unexpected error: {error}"
        );
    }
}
