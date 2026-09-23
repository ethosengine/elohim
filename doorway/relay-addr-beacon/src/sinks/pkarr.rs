//! Tier-2 sink: pkarr signed-packet publish to a relay.
//!
//! We build a `SignedPacket` with an A (and optionally AAAA) record at the
//! signer's apex (`.`) and PUT the relay payload to `{relay}/{z32}`.
//!
//! IMPORTANT — ICE cannot yet CONSUME pkarr names. tx5 / the conductor ICE
//! config resolve ordinary DNS (Tier-1) today; consuming a pkarr key as a TURN
//! host needs the Tier-2 resolution bridge that is not built yet. These records
//! are published so that bridge has something to resolve when it lands — they
//! are NOT consumed by any relay client today. This sink is OFF by default.
//!
//! The `signed_packet` feature set does NOT compile `pkarr::Client`, so the PUT
//! is done directly with reqwest.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use pkarr::dns::Name;
use pkarr::{Keypair, SignedPacket};
use tracing::info;

use super::{AddrUpdate, Sink};

const PKARR_TTL: u32 = 60;

/// A configured pkarr sink.
pub struct PkarrSink {
    client: reqwest::Client,
    key_file: PathBuf,
    relay: String,
    enable_v6: bool,
}

impl PkarrSink {
    pub fn new(client: reqwest::Client, key_file: PathBuf, relay: String, enable_v6: bool) -> Self {
        Self {
            client,
            key_file,
            relay,
            enable_v6,
        }
    }
}

/// Load a dedicated pkarr keypair from `path`, generating and persisting a new
/// one (hex, mode 0600) if the file does not exist.
fn load_or_create_keypair(path: &Path) -> Result<Keypair> {
    if path.exists() {
        Keypair::from_secret_key_file(path)
            .map_err(|e| anyhow!("pkarr: reading key file {}: {e}", path.display()))
    } else {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("pkarr: creating key dir {}", parent.display()))?;
            }
        }
        let keypair = Keypair::random();
        keypair
            .write_secret_key_file(path)
            .map_err(|e| anyhow!("pkarr: writing key file {}: {e}", path.display()))?;
        // Enforce 0600 regardless of the writer's default umask.
        let mut perms = std::fs::metadata(path)
            .with_context(|| format!("pkarr: stat key file {}", path.display()))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("pkarr: chmod 0600 {}", path.display()))?;
        info!(path = %path.display(), "pkarr: generated dedicated keypair");
        Ok(keypair)
    }
}

impl Sink for PkarrSink {
    fn name(&self) -> &'static str {
        "pkarr"
    }

    async fn publish(&self, update: &AddrUpdate) -> Result<()> {
        let keypair = load_or_create_keypair(&self.key_file)?;

        // Apex name (".") -> the signer's own key.
        let mut builder = SignedPacket::builder().a(
            Name::new(".").context("pkarr: apex name")?,
            update.wan_v4,
            PKARR_TTL,
        );
        if self.enable_v6 {
            if let Some(v6) = update.wan_v6 {
                builder = builder.aaaa(Name::new(".").context("pkarr: apex name")?, v6, PKARR_TTL);
            }
        }
        let signed = builder
            .sign(&keypair)
            .map_err(|e| anyhow!("pkarr: signing packet: {e}"))?;

        let payload = signed.to_relay_payload();
        let z32 = keypair.public_key().to_z32();
        let url = format!("{}/{}", self.relay.trim_end_matches('/'), z32);

        self.client
            .put(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(payload)
            .send()
            .await
            .with_context(|| format!("pkarr: PUT {url}"))?
            .error_for_status()
            .with_context(|| format!("pkarr: PUT {url} status"))?;

        info!(relay = %self.relay, z32 = %z32, "pkarr: published signed packet");
        Ok(())
    }
}
