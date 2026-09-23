//! Local sink: render coturn's `external-ip` mapping and optionally reload.
//!
//! coturn behind residential NAT must advertise `external-ip=<wan>/<lan>` (or a
//! bare `<wan>` when the LAN address is unknown) or it hands out the useless
//! private candidate. coturn does NOT hot-reload `external-ip` on SIGHUP, so the
//! configurable `on_change_exec` should RESTART coturn (see the crate README).

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use tracing::{debug, info, warn};

use super::{AddrUpdate, Sink};

/// Render the coturn config: the base config verbatim, then an appended
/// `external-ip=<wan>[/<lan>]` line. Pure and unit-tested.
pub fn render_conf(base: &str, wan: Ipv4Addr, lan: Option<Ipv4Addr>) -> String {
    let mapping = match lan {
        Some(l) => format!("{wan}/{l}"),
        None => wan.to_string(),
    };
    let mut out = String::with_capacity(base.len() + mapping.len() + 16);
    out.push_str(base);
    if !base.is_empty() && !base.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("external-ip=");
    out.push_str(&mapping);
    out.push('\n');
    out
}

/// Canonicalize a path best-effort. `out_conf` may not exist yet on first run,
/// so fall back to canonicalizing the parent dir and re-attaching the file name
/// (still resolves symlinks / `..` in the directory), and finally to the path
/// as-given.
fn canonical_best_effort(p: &Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    if let (Some(parent), Some(name)) = (p.parent(), p.file_name()) {
        if !parent.as_os_str().is_empty() {
            if let Ok(cp) = parent.canonicalize() {
                return cp.join(name);
            }
        }
    }
    p.to_path_buf()
}

/// A configured coturn sink.
pub struct CoturnSink {
    base_conf: PathBuf,
    out_conf: PathBuf,
    on_change_exec: Option<String>,
}

impl CoturnSink {
    pub fn new(
        base_conf: PathBuf,
        out_conf: PathBuf,
        on_change_exec: Option<String>,
    ) -> Result<Self> {
        // If base and out resolve to the SAME file, each cycle would append an
        // `external-ip=` line onto a file that already has one — compounding the
        // config indefinitely. Refuse to construct.
        if canonical_best_effort(&base_conf) == canonical_best_effort(&out_conf) {
            return Err(anyhow!(
                "coturn: base conf ({}) and out conf ({}) resolve to the same path — \
                 rendering would compound external-ip lines every cycle; they must differ",
                base_conf.display(),
                out_conf.display()
            ));
        }
        Ok(Self {
            base_conf,
            out_conf,
            on_change_exec,
        })
    }
}

impl Sink for CoturnSink {
    fn name(&self) -> &'static str {
        "coturn"
    }

    async fn publish(&self, update: &AddrUpdate) -> Result<()> {
        let base = std::fs::read_to_string(&self.base_conf)
            .with_context(|| format!("coturn: reading base conf {}", self.base_conf.display()))?;
        let rendered = render_conf(&base, update.wan_v4, update.lan_v4);

        // If the freshly-rendered conf is byte-identical to what's already on
        // disk, skip both the write and the on-change (restart) exec — running
        // the restart on a no-op cycle is pointless thrash.
        if let Ok(existing) = std::fs::read_to_string(&self.out_conf) {
            if existing == rendered {
                debug!(
                    out = %self.out_conf.display(),
                    "coturn: rendered conf unchanged — skipping write and on-change exec"
                );
                return Ok(());
            }
        }

        std::fs::write(&self.out_conf, &rendered)
            .with_context(|| format!("coturn: writing {}", self.out_conf.display()))?;
        info!(
            out = %self.out_conf.display(),
            wan = %update.wan_v4,
            lan = ?update.lan_v4,
            "coturn: rendered external-ip config"
        );

        if let Some(cmd) = &self.on_change_exec {
            info!(cmd, "coturn: running on-change exec");
            let status = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .status()
                .await
                .with_context(|| format!("coturn: spawning on-change exec {cmd:?}"))?;
            if status.success() {
                info!(cmd, "coturn: on-change exec succeeded");
            } else {
                warn!(cmd, ?status, "coturn: on-change exec returned non-zero");
                return Err(anyhow::anyhow!(
                    "coturn: on-change exec {cmd:?} exited with {status}"
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_wan_lan_mapping() {
        let base = "listening-port=3478\nrealm=elohim.host\n";
        let out = render_conf(
            base,
            Ipv4Addr::new(203, 0, 113, 7),
            Some(Ipv4Addr::new(192, 168, 1, 100)),
        );
        assert!(out.starts_with(base));
        assert!(out.ends_with("external-ip=203.0.113.7/192.168.1.100\n"));
    }

    #[test]
    fn renders_bare_wan_when_no_lan() {
        let base = "realm=elohim.host\n";
        let out = render_conf(base, Ipv4Addr::new(203, 0, 113, 7), None);
        assert!(out.ends_with("external-ip=203.0.113.7\n"));
        assert!(!out.contains('/'));
    }

    #[test]
    fn inserts_newline_when_base_lacks_trailing_newline() {
        let base = "realm=elohim.host"; // no trailing newline
        let out = render_conf(base, Ipv4Addr::new(203, 0, 113, 7), None);
        assert_eq!(out, "realm=elohim.host\nexternal-ip=203.0.113.7\n");
    }

    #[test]
    fn handles_empty_base() {
        let out = render_conf("", Ipv4Addr::new(203, 0, 113, 7), None);
        assert_eq!(out, "external-ip=203.0.113.7\n");
    }
}
