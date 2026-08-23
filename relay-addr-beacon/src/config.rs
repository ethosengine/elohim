//! Command-line / environment configuration.
//!
//! Precedence is `flag > env > default`, which is exactly clap's native
//! behaviour: an explicit flag wins, otherwise the `env` fallback is consulted,
//! otherwise the declared default is used.

use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};

/// One sibling-safe shared DNS record lane contributed by this beacon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedRecordLane {
    pub record_name: String,
    pub owner: String,
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
            let key = lane.record_name.trim_end_matches('.').to_ascii_lowercase();
            if !names.insert(key) {
                return Err(anyhow!(
                    "duplicate shared record lane name: {}",
                    lane.record_name
                ));
            }
        }

        Ok(lanes)
    }

    /// Cross-field validation clap's declarative attributes can't express.
    /// Called once at startup, independent of which sinks are enabled — these
    /// are shared-mode invariants, not per-sink construction concerns.
    pub fn validate(&self) -> Result<()> {
        self.shared_record_lanes()?;
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
