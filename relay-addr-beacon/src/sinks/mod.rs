//! Publish sinks. Each sink is independently enabled (via `--sink`) and they
//! compose: every enabled sink runs on each publish.

pub mod cloudflare;
pub mod coturn;
pub mod file;
pub mod pkarr;

use std::net::{Ipv4Addr, Ipv6Addr};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A detected address snapshot handed to every sink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddrUpdate {
    /// Public egress IPv4 (always present).
    pub wan_v4: Ipv4Addr,
    /// Public egress IPv6 (only when `--enable-v6`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wan_v6: Option<Ipv6Addr>,
    /// Primary LAN IPv4, used for coturn's `external-ip=<wan>/<lan>` mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lan_v4: Option<Ipv4Addr>,
}

/// A publish target. Implemented per concrete sink; dispatched through
/// [`ActiveSink`] so a heterogeneous, composed set can be driven uniformly.
#[allow(async_fn_in_trait)]
pub trait Sink {
    /// Stable short name for logging.
    fn name(&self) -> &'static str;
    /// Publish the current address snapshot.
    async fn publish(&self, update: &AddrUpdate) -> Result<()>;
}

/// Enum dispatch over the concrete sinks so multiple sinks compose in one loop.
pub enum ActiveSink {
    Cloudflare(cloudflare::CloudflareSink),
    Pkarr(pkarr::PkarrSink),
    Coturn(coturn::CoturnSink),
    File(file::FileMembershipSink),
}

impl ActiveSink {
    pub fn name(&self) -> &'static str {
        match self {
            ActiveSink::Cloudflare(s) => s.name(),
            ActiveSink::Pkarr(s) => s.name(),
            ActiveSink::Coturn(s) => s.name(),
            ActiveSink::File(s) => s.name(),
        }
    }

    pub async fn publish(&self, update: &AddrUpdate) -> Result<()> {
        match self {
            ActiveSink::Cloudflare(s) => s.publish(update).await,
            ActiveSink::Pkarr(s) => s.publish(update).await,
            ActiveSink::Coturn(s) => s.publish(update).await,
            ActiveSink::File(s) => s.publish(update).await,
        }
    }

    /// Reconcile ONE shared lane's membership through this sink.
    ///
    /// Every membership projection is handed the SAME already-decided verdict
    /// for that lane. A sink that holds no lane by this name is a no-op, which
    /// is what lets one beacon process run two lanes with different sink
    /// targets: the Cloudflare sink reconciles the lanes whose records it
    /// owns, and each file sink reconciles the one document it writes.
    pub async fn reconcile_lane(
        &self,
        record_name: &str,
        serving: bool,
        update: Option<&AddrUpdate>,
    ) -> Result<()> {
        match self {
            ActiveSink::Cloudflare(s) => {
                s.reconcile_membership_lane(record_name, serving, update)
                    .await
            }
            ActiveSink::File(s) => {
                if s.owns_lane(record_name) {
                    s.reconcile_membership(serving).await
                } else {
                    Ok(())
                }
            }
            ActiveSink::Pkarr(_) | ActiveSink::Coturn(_) => Ok(()),
        }
    }

    /// Does this sink's projection depend on the detected WAN/LAN address?
    ///
    /// Every DNS/coturn projection does — they publish the address itself. The
    /// file membership sink does NOT: it projects ORIGINS, which are declared
    /// configuration. A beacon leg running only the file sink therefore needs
    /// no egress echo endpoint at all, which is what lets a household run one
    /// per doorway on a loopback mesh with no public IP and no internet.
    pub fn needs_address(&self) -> bool {
        match self {
            ActiveSink::Cloudflare(_) | ActiveSink::Pkarr(_) | ActiveSink::Coturn(_) => true,
            ActiveSink::File(_) => false,
        }
    }
}
