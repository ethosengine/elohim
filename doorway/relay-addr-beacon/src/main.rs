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
    cloudflare, cloudflare::CloudflareSink, coturn::CoturnSink, file::FileMembershipSink,
    pkarr::PkarrSink, ActiveSink, AddrUpdate, Sink,
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
                // guarantees each preferred or legacy lane has an owner and
                // that record names are unique. Resolve again defensively so
                // this constructor remains sound if called independently.
                let shared = cfg
                    .shared_record_lanes()?
                    .into_iter()
                    .map(|lane| cloudflare::SharedRecordConfig {
                        record_name: lane.record_name,
                        owner: lane.owner,
                        refresh_secs: cfg.shared_refresh_secs,
                        stale_secs: cfg.shared_stale_secs,
                    })
                    .collect();
                sinks.push(ActiveSink::Cloudflare(
                    CloudflareSink::new(
                        client.clone(),
                        token,
                        zone,
                        record_name,
                        cfg.enable_v6,
                        shared,
                    )
                    .with_serving_probe(cfg.serving_probe_url.is_some()),
                ));
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
            SinkName::File => {
                // `Config::validate` already proved each of these; resolve
                // defensively so this constructor stays sound if called alone.
                let origin = cfg.member_origin.clone().ok_or_else(|| {
                    anyhow!("file sink requires --member-origin / BEACON_MEMBER_ORIGIN")
                })?;
                // ONE SINK PER LANE, each with its own derived document: one
                // membership document names one public name, so two lanes are
                // two documents — never one document whose `name` field means
                // whichever lane was parsed last.
                for (lane, path) in cfg.membership_lanes()? {
                    info!(
                        lane = %lane.record_name,
                        owner = %lane.owner,
                        document = %path.display(),
                        "file membership lane"
                    );
                    sinks.push(ActiveSink::File(FileMembershipSink::new(
                        path,
                        lane.record_name,
                        lane.owner,
                        origin.clone(),
                        cfg.shared_refresh_secs,
                    )));
                }
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

/// Run ONLY the Cloudflare sink's freshness verification (every other sink is
/// untouched). Used on an unchanged-address cycle: verifies the exclusive
/// `--record-name` record still holds what we published (re-asserting on
/// external clobber/deletion — the 2026-07-28 apex incident: a forgotten
/// ddclient PATCHed the beacon-owned apex back 25s after publish and the
/// drift stood undetected), and runs the shared lane's periodic
/// freshness-stamp refresh + stale-sibling reap. Read-only when nothing
/// drifted. See `cycle`'s doc comment for why this is the least-churn
/// structure.
async fn run_freshness_verify(sinks: &[ActiveSink], update: &AddrUpdate) -> bool {
    let mut all_ok = true;
    for sink in sinks {
        if let ActiveSink::Cloudflare(cf) = sink {
            match cf.verify_freshness(update).await {
                Ok(()) => info!(sink = cf.name(), "cloudflare freshness verify ok"),
                Err(e) => {
                    all_ok = false;
                    error!(
                        sink = cf.name(),
                        error = %format!("{e:#}"),
                        "cloudflare freshness verify failed"
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
/// Unchanged-address branching: with a Cloudflare sink enabled, an unchanged
/// cycle runs its FRESHNESS VERIFICATION — a read-mostly pass that (a)
/// re-reads the exclusive `--record-name` record and re-asserts it only if an
/// external writer clobbered or deleted it (2026-07-28: a forgotten ddclient
/// reverted the beacon-owned apex 25s after publish, and the old
/// publish-only-on-address-change design left the drift standing
/// indefinitely), and (b) runs the shared lane's periodic freshness-stamp
/// refresh and stale-sibling reap (a beacon whose WAN IP never changes must
/// still re-stamp, or a sibling would eventually reap it as abandoned). The
/// alternative — running every sink unconditionally every cycle — remains
/// rejected: the pkarr sink re-signs and PUTs on every call with no
/// idempotency check, and the Cloudflare `upsert` unconditionally
/// PATCHes/POSTs, so both would hammer their APIs every poll interval
/// (verification only WRITES on observed drift). State is not persisted on
/// these cycles (the address itself is unchanged) and no other sink is
/// touched; with no Cloudflare sink enabled, an unchanged cycle skips every
/// sink as before.
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
        let has_cloudflare = sinks.iter().any(|s| matches!(s, ActiveSink::Cloudflare(_)));
        if has_cloudflare {
            info!(
                "no change since last publish — running cloudflare freshness verification (exclusive + shared lanes)"
            );
            return Ok(run_freshness_verify(sinks, &update).await);
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

/// One health cycle: status is the contract; a malformed 200 is still serving.
/// Uses its own no-redirect bounded client, independent of WAN discovery.
async fn serving_cycle(
    cfg: &Config,
    client: &reqwest::Client,
    sinks: &[ActiveSink],
    membership: &mut state::MembershipSet,
) -> Result<()> {
    let url = cfg
        .serving_probe_url
        .as_deref()
        .context("serving probe URL unset")?;
    let serving = client
        .get(url)
        .send()
        .await
        .is_ok_and(|response| response.status() == reqwest::StatusCode::OK);
    membership.observe(serving, cfg.serving_leave_after, cfg.serving_join_after);
    apply_membership(cfg, sinks, membership).await
}

/// Project each lane's already-decided verdict through every sink.
///
/// Lanes are reconciled INDEPENDENTLY: one lane's failing projection never
/// suppresses another's, and a lane is marked applied only on its own success.
/// The address snapshot is loaded at most once per pass and shared by every
/// joining lane — the verdicts differ per lane, the detected address does not.
async fn apply_membership(
    cfg: &Config,
    sinks: &[ActiveSink],
    membership: &mut state::MembershipSet,
) -> Result<()> {
    let any_serving = membership
        .lanes()
        .iter()
        .any(|held| held.membership.serving);
    let previous = if any_serving {
        state::load(&cfg.state_file)?
    } else {
        None
    };
    let mut failures = 0_usize;
    for held in membership.lanes_mut() {
        let serving = held.membership.serving;
        let mut lane_ok = true;
        for sink in sinks {
            // Every sink holding this lane is handed the SAME already-decided
            // verdict. Cloudflare projects it as A records for the fleet; a
            // file sink projects it into the one document it owns.
            if let Err(error) = sink
                .reconcile_lane(&held.lane.record_name, serving, previous.as_ref())
                .await
            {
                lane_ok = false;
                warn!(
                    sink = sink.name(),
                    lane = %held.lane.record_name,
                    owner = %held.lane.owner,
                    error = %error,
                    "shared membership projection failed; retrying next probe"
                );
            }
        }
        if !lane_ok {
            failures += 1;
            continue;
        }
        if held.membership.applied != Some(serving) {
            info!(
                lane = %held.lane.record_name,
                owner = %held.lane.owner,
                serving,
                "shared membership projection applied"
            );
            held.membership.applied = Some(serving);
        }
    }
    if failures > 0 {
        return Err(anyhow!(
            "shared membership projection incomplete for {failures} lane(s)"
        ));
    }
    Ok(())
}

fn build_probe_client(interval: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(interval.min(15)))
        .build()
        .context("building serving probe client")
}

async fn address_loop(cfg: &Config, client: &reqwest::Client, sinks: &[ActiveSink]) {
    loop {
        if let Err(e) = cycle(cfg, client, sinks, false).await {
            error!(error = %format!("{e:#}"), "cycle failed — retrying next interval");
        }
        tokio::time::sleep(Duration::from_secs(cfg.interval_secs)).await;
    }
}

async fn serving_loop(
    cfg: &Config,
    client: &reqwest::Client,
    sinks: &[ActiveSink],
    mut membership: state::MembershipSet,
) {
    loop {
        if let Err(e) = serving_cycle(cfg, client, sinks, &mut membership).await {
            warn!(error = %e, "serving cycle failed — retrying next interval");
        }
        tokio::time::sleep(Duration::from_secs(cfg.serving_probe_interval_secs)).await;
    }
}

async fn run(cfg: Config) -> Result<()> {
    cfg.validate()?;
    let client = build_http_client()?;
    let sinks = build_sinks(&cfg, &client)?;
    let enabled: Vec<&str> = sinks.iter().map(ActiveSink::name).collect();
    // A membership-only leg (the household `file` sink alone) projects ORIGINS,
    // not the detected address, so it must not require an egress echo endpoint:
    // a loopback household mesh has no public IP to discover and may have no
    // internet at all. Skipping the address loop entirely is the difference
    // between "runs at home" and "logs a detection error every interval".
    let needs_address = sinks.iter().any(ActiveSink::needs_address);
    info!(
        ?enabled,
        once = cfg.once,
        enable_v6 = cfg.enable_v6,
        needs_address,
        "relay-addr-beacon starting"
    );

    // Per-lane serving evidence, built from the SAME validated lane list the
    // sinks were constructed from.
    let lanes = cfg.shared_record_lanes()?;

    if cfg.once {
        let mut membership = state::MembershipSet::new(&lanes);
        if cfg.serving_probe_url.is_some() {
            // Withdraw stale contributions before detection, even if WAN is down.
            apply_membership(&cfg, &sinks, &mut membership).await?;
        }
        let ok = if needs_address {
            cycle(&cfg, &client, &sinks, true).await?
        } else {
            info!("no address-dependent sink enabled — skipping address detection");
            true
        };
        if cfg.serving_probe_url.is_some() {
            serving_cycle(
                &cfg,
                &build_probe_client(cfg.serving_probe_interval_secs)?,
                &sinks,
                &mut membership,
            )
            .await?;
        }
        if ok {
            Ok(())
        } else {
            Err(anyhow!("--once: at least one sink failed"))
        }
    } else {
        match (needs_address, cfg.serving_probe_url.is_some()) {
            (true, true) => {
                let probe_client = build_probe_client(cfg.serving_probe_interval_secs)?;
                tokio::join!(
                    address_loop(&cfg, &client, &sinks),
                    serving_loop(
                        &cfg,
                        &probe_client,
                        &sinks,
                        state::MembershipSet::new(&lanes)
                    )
                );
            }
            (true, false) => address_loop(&cfg, &client, &sinks).await,
            (false, true) => {
                let probe_client = build_probe_client(cfg.serving_probe_interval_secs)?;
                serving_loop(
                    &cfg,
                    &probe_client,
                    &sinks,
                    state::MembershipSet::new(&lanes),
                )
                .await;
            }
            (false, false) => {
                return Err(anyhow!(
                    "nothing to run: no address-dependent sink and no --serving-probe-url"
                ))
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod file_membership_tests;
#[cfg(test)]
mod membership_tests;
