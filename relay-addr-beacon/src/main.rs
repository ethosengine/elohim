//! relay-addr-beacon — keeps a sovereign relay's dynamic residential WAN IP
//! fresh in DNS (Tier-1: Cloudflare) and pkarr (Tier-2), and feeds coturn its
//! `external-ip` mapping. N-generalizable: run one per relay node.

mod config;
mod detect;
mod sinks;
mod state;

use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use tracing::{error, info, warn};

use config::{Config, SinkName};
use sinks::{
    cloudflare, cloudflare::CloudflareSink, coturn::CoturnSink, pkarr::PkarrSink, ActiveSink,
    AddrUpdate, Sink,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::parse();
    init_tracing();
    run(cfg).await
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn build_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("relay-addr-beacon/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(15))
        .build()
        .context("building HTTP client")
}

/// Resolve the Cloudflare token from flag/env, else the token file.
fn resolve_cf_token(cfg: &Config) -> Result<String> {
    if let Some(token) = &cfg.cf_token {
        return Ok(token.clone());
    }
    if let Some(path) = &cfg.cf_token_file {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading cloudflare token file {}", path.display()))?;
        return Ok(raw.trim().to_string());
    }
    Err(anyhow!(
        "cloudflare sink requires --cf-token / CF_API_TOKEN or --cf-token-file"
    ))
}

/// Construct the enabled sinks, validating that each has the config it needs.
fn build_sinks(cfg: &Config, client: &reqwest::Client) -> Result<Vec<ActiveSink>> {
    if cfg.sinks.is_empty() {
        return Err(anyhow!("no sink enabled — pass at least one --sink"));
    }
    let mut sinks = Vec::new();
    for sink in &cfg.sinks {
        match sink {
            SinkName::Cloudflare => {
                let zone = cfg
                    .cf_zone
                    .clone()
                    .ok_or_else(|| anyhow!("cloudflare sink requires --cf-zone / CF_ZONE"))?;
                let record_name = cfg.record_name.clone().ok_or_else(|| {
                    anyhow!("cloudflare sink requires --record-name / BEACON_RECORD_NAME")
                })?;
                let token = resolve_cf_token(cfg)?;
                // `Config::validate` (called before `build_sinks`) already
                // guarantees shared_record_name.is_some() => record_owner.is_some();
                // the match stays defensive rather than asserting/unwrapping.
                let shared = match (&cfg.shared_record_name, &cfg.record_owner) {
                    (Some(name), Some(owner)) => Some(cloudflare::SharedRecordConfig {
                        record_name: name.clone(),
                        owner: owner.clone(),
                        refresh_secs: cfg.shared_refresh_secs,
                        stale_secs: cfg.shared_stale_secs,
                    }),
                    _ => None,
                };
                sinks.push(ActiveSink::Cloudflare(CloudflareSink::new(
                    client.clone(),
                    token,
                    zone,
                    record_name,
                    cfg.enable_v6,
                    shared,
                )));
            }
            SinkName::Pkarr => {
                sinks.push(ActiveSink::Pkarr(PkarrSink::new(
                    client.clone(),
                    cfg.pkarr_key_file.clone(),
                    cfg.pkarr_relay.clone(),
                    cfg.enable_v6,
                )));
            }
            SinkName::Coturn => {
                let base_conf = cfg.coturn_base_conf.clone().ok_or_else(|| {
                    anyhow!("coturn sink requires --coturn-base-conf / COTURN_BASE_CONF")
                })?;
                let out_conf = cfg.coturn_out_conf.clone().ok_or_else(|| {
                    anyhow!("coturn sink requires --coturn-out-conf / COTURN_OUT_CONF")
                })?;
                sinks.push(ActiveSink::Coturn(CoturnSink::new(
                    base_conf,
                    out_conf,
                    cfg.on_change_exec.clone(),
                )?));
            }
        }
    }
    Ok(sinks)
}

/// Detect the current address snapshot. `prev_lan` is the last-published LAN
/// IPv4 (if any): when the LAN probe transiently fails we carry it forward
/// rather than dropping to `None`, so a flaky probe doesn't thrash change
/// detection (and coturn's `<wan>/<lan>` mapping).
async fn detect_snapshot(
    cfg: &Config,
    client: &reqwest::Client,
    prev_lan: Option<Ipv4Addr>,
) -> Result<AddrUpdate> {
    let endpoints = detect::resolved_endpoints(&cfg.egress_endpoints);

    let wan_v4 = detect::detect_public_ipv4(client, &endpoints)
        .await
        .context("detecting public IPv4")?;

    let wan_v6 = if cfg.enable_v6 {
        match detect::detect_public_ipv6(client, &endpoints).await {
            Ok(v6) => Some(v6),
            Err(e) => {
                warn!(error = %e, "could not detect public IPv6 (continuing without AAAA)");
                None
            }
        }
    } else {
        None
    };

    let lan_v4 = match cfg.lan_ip {
        Some(ip) => Some(ip),
        None => match detect::detect_lan_ipv4() {
            Ok(ip) => Some(ip),
            Err(e) => match prev_lan {
                Some(prev) => {
                    warn!(error = %e, previous_lan = %prev, "LAN IPv4 probe failed — carrying previous LAN forward (avoids change-detection thrash)");
                    Some(prev)
                }
                None => {
                    warn!(error = %e, "could not detect LAN IPv4 (coturn will advertise bare WAN)");
                    None
                }
            },
        },
    };

    Ok(AddrUpdate {
        wan_v4,
        wan_v6,
        lan_v4,
    })
}

/// Run all enabled sinks. Returns true if every sink succeeded. A failing sink
/// never aborts the others — each outcome is logged independently.
async fn run_sinks(sinks: &[ActiveSink], update: &AddrUpdate) -> bool {
    let mut all_ok = true;
    for sink in sinks {
        match sink.publish(update).await {
            Ok(()) => info!(sink = sink.name(), "sink publish ok"),
            Err(e) => {
                all_ok = false;
                error!(sink = sink.name(), error = %format!("{e:#}"), "sink publish failed");
            }
        }
    }
    all_ok
}

/// Run ONLY the Cloudflare sink's shared "logical anycast" lane (skips its
/// exclusive `--record-name` lane and every other sink entirely). Used on an
/// unchanged-address cycle when shared mode is configured, so the periodic
/// freshness-stamp refresh (and stale-sibling reap) doesn't starve — without
/// re-touching sinks that have nothing to do. See `cycle`'s doc comment for
/// why this is the least-churn structure.
async fn run_shared_refresh_only(sinks: &[ActiveSink], update: &AddrUpdate) -> bool {
    let mut all_ok = true;
    for sink in sinks {
        if let ActiveSink::Cloudflare(cf) = sink {
            match cf.publish_shared_only(update).await {
                Ok(()) => info!(sink = cf.name(), "shared-lane freshness refresh ok"),
                Err(e) => {
                    all_ok = false;
                    error!(
                        sink = cf.name(),
                        error = %format!("{e:#}"),
                        "shared-lane freshness refresh failed"
                    );
                }
            }
        }
    }
    all_ok
}

/// One detect->(maybe publish)->persist cycle. `force` runs sinks regardless of
/// change (used by `--once`). Returns true if the cycle is fully healthy.
///
/// Unchanged-address branching: with NO shared mode configured, this is
/// unchanged legacy behavior — a no-op cycle skips every sink. With shared
/// mode configured, a global skip would starve the shared lane's periodic
/// freshness-stamp refresh and stale-sibling reap (a beacon whose WAN IP
/// never changes must still periodically re-stamp, or a sibling would
/// eventually reap it as abandoned). The alternative — running every sink
/// unconditionally every cycle — was rejected: the pkarr sink re-signs and
/// PUTs on every call with no idempotency check, and the Cloudflare exclusive
/// lane's `upsert` unconditionally PATCHes/POSTs with no content-diff guard,
/// so both would needlessly hammer their APIs every poll interval forever
/// (coturn's sink *does* already no-op on an identical rendered config, but
/// that alone doesn't make "run everything" churn-free). So on an unchanged
/// cycle with shared mode configured, only the Cloudflare shared lane runs;
/// state is not persisted (the address itself is unchanged, so there is
/// nothing new to persist) and no other sink is touched.
async fn cycle(
    cfg: &Config,
    client: &reqwest::Client,
    sinks: &[ActiveSink],
    force: bool,
) -> Result<bool> {
    // Load the previous snapshot FIRST so its LAN IP can be carried forward when
    // this cycle's LAN probe transiently fails.
    let previous = state::load(&cfg.state_file)
        .with_context(|| format!("loading state {}", cfg.state_file.display()))?;
    let prev_lan = previous.as_ref().and_then(|p| p.lan_v4);
    let update = detect_snapshot(cfg, client, prev_lan).await?;
    let changed = state::has_changed(previous.as_ref(), &update);

    info!(
        wan_v4 = %update.wan_v4,
        wan_v6 = ?update.wan_v6,
        lan_v4 = ?update.lan_v4,
        changed,
        force,
        "detected address snapshot"
    );

    if !changed && !force {
        if cfg.shared_record_name.is_some() {
            info!(
                "no change since last publish — running cloudflare shared-lane freshness refresh only"
            );
            return Ok(run_shared_refresh_only(sinks, &update).await);
        }
        info!("no change since last publish — skipping sinks");
        return Ok(true);
    }

    let all_ok = run_sinks(sinks, &update).await;

    // Persist only on full success so a transient sink failure retries next
    // cycle rather than being suppressed by a stale "already published" record.
    if all_ok {
        state::save(&cfg.state_file, &update)
            .with_context(|| format!("saving state {}", cfg.state_file.display()))?;
        info!(state = %cfg.state_file.display(), "persisted published snapshot");
    } else {
        warn!("not persisting state — at least one sink failed; will retry next cycle");
    }

    Ok(all_ok)
}

async fn run(cfg: Config) -> Result<()> {
    cfg.validate()?;
    let client = build_http_client()?;
    let sinks = build_sinks(&cfg, &client)?;
    let enabled: Vec<&str> = sinks.iter().map(ActiveSink::name).collect();
    info!(
        ?enabled,
        once = cfg.once,
        enable_v6 = cfg.enable_v6,
        "relay-addr-beacon starting"
    );

    if cfg.once {
        let ok = cycle(&cfg, &client, &sinks, true).await?;
        if ok {
            Ok(())
        } else {
            Err(anyhow!("--once: at least one sink failed"))
        }
    } else {
        let interval = Duration::from_secs(cfg.interval_secs);
        loop {
            if let Err(e) = cycle(&cfg, &client, &sinks, false).await {
                // A whole-cycle error (e.g. no endpoint reachable) is logged and
                // retried — a transient outage should not kill the daemon.
                error!(error = %format!("{e:#}"), "cycle failed — retrying next interval");
            }
            tokio::time::sleep(interval).await;
        }
    }
}
