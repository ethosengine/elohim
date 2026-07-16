//! Publish sinks. Each sink is independently enabled (via `--sink`) and they
//! compose: every enabled sink runs on each publish.

pub mod cloudflare;
pub mod coturn;
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
}

impl ActiveSink {
    pub fn name(&self) -> &'static str {
        match self {
            ActiveSink::Cloudflare(s) => s.name(),
            ActiveSink::Pkarr(s) => s.name(),
            ActiveSink::Coturn(s) => s.name(),
        }
    }

    pub async fn publish(&self, update: &AddrUpdate) -> Result<()> {
        match self {
            ActiveSink::Cloudflare(s) => s.publish(update).await,
            ActiveSink::Pkarr(s) => s.publish(update).await,
            ActiveSink::Coturn(s) => s.publish(update).await,
        }
    }
}
