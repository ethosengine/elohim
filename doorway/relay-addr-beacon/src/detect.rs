//! Address detection: public egress IP (via HTTP echo endpoints) and the
//! primary LAN IPv4 (via a connect-only UDP socket, no packets sent).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket};

use anyhow::{anyhow, Context, Result};

/// Default ordered list of public-IP echo endpoints. First success wins.
pub const DEFAULT_EGRESS_ENDPOINTS: &[&str] = &[
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
];

/// Parse and validate an IP echoed by an HTTP endpoint (tolerates surrounding
/// whitespace/newlines that plain-text echo services append).
pub fn parse_echoed_ip(raw: &str) -> Result<IpAddr> {
    let trimmed = raw.trim();
    trimmed
        .parse::<IpAddr>()
        .map_err(|e| anyhow!("echoed value {trimmed:?} is not a valid IP: {e}"))
}

/// Resolve the ordered endpoint list, falling back to the built-in defaults.
pub fn resolved_endpoints(overrides: &[String]) -> Vec<String> {
    if overrides.is_empty() {
        DEFAULT_EGRESS_ENDPOINTS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        overrides.to_vec()
    }
}

async fn fetch_echoed_ip(client: &reqwest::Client, endpoint: &str) -> Result<IpAddr> {
    let body = client
        .get(endpoint)
        .send()
        .await
        .with_context(|| format!("GET {endpoint}"))?
        .error_for_status()
        .with_context(|| format!("non-success status from {endpoint}"))?
        .text()
        .await
        .with_context(|| format!("reading body from {endpoint}"))?;
    parse_echoed_ip(&body).with_context(|| format!("from {endpoint}"))
}

/// Detect the public egress IPv4 by trying each endpoint in order; the first
/// that returns a valid IPv4 wins. Endpoints that answer with IPv6 are skipped.
pub async fn detect_public_ipv4(
    client: &reqwest::Client,
    endpoints: &[String],
) -> Result<Ipv4Addr> {
    let mut last_err: Option<anyhow::Error> = None;
    for endpoint in endpoints {
        match fetch_echoed_ip(client, endpoint).await {
            Ok(IpAddr::V4(v4)) => return Ok(v4),
            Ok(IpAddr::V6(v6)) => {
                last_err = Some(anyhow!("{endpoint} returned IPv6 {v6}, wanted IPv4"));
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("no egress endpoints configured")))
}

/// Detect the public egress IPv6 (only used when `--enable-v6`). Operators
/// enabling v6 should ensure their endpoint list is reachable over IPv6.
pub async fn detect_public_ipv6(
    client: &reqwest::Client,
    endpoints: &[String],
) -> Result<Ipv6Addr> {
    let mut last_err: Option<anyhow::Error> = None;
    for endpoint in endpoints {
        match fetch_echoed_ip(client, endpoint).await {
            Ok(IpAddr::V6(v6)) => return Ok(v6),
            Ok(IpAddr::V4(v4)) => {
                last_err = Some(anyhow!("{endpoint} returned IPv4 {v4}, wanted IPv6"));
            }
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("no egress endpoints configured")))
}

/// Detect the primary non-loopback LAN IPv4 by opening a UDP socket and
/// `connect`-ing it toward a public address. No datagram is sent; the kernel
/// simply picks the egress interface, whose local address we then read.
pub fn detect_lan_ipv4() -> Result<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").context("bind UDP probe socket")?;
    socket
        .connect("8.8.8.8:80")
        .context("connect UDP probe socket (no packet sent)")?;
    match socket.local_addr().context("read local_addr")?.ip() {
        IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_unspecified() => Ok(v4),
        other => Err(anyhow!(
            "no usable non-loopback LAN IPv4 (probe gave {other})"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_ipv4() {
        assert_eq!(
            parse_echoed_ip("203.0.113.7").unwrap(),
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))
        );
    }

    #[test]
    fn tolerates_trailing_newline_and_spaces() {
        assert_eq!(
            parse_echoed_ip("  203.0.113.7\n").unwrap(),
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))
        );
    }

    #[test]
    fn parses_ipv6() {
        let ip = parse_echoed_ip("2001:db8::1").unwrap();
        assert!(matches!(ip, IpAddr::V6(_)));
    }

    #[test]
    fn rejects_non_ip() {
        assert!(parse_echoed_ip("not-an-ip").is_err());
        assert!(parse_echoed_ip("999.999.999.999").is_err());
        assert!(parse_echoed_ip("").is_err());
    }

    #[test]
    fn defaults_used_when_no_overrides() {
        let eps = resolved_endpoints(&[]);
        assert_eq!(eps.len(), DEFAULT_EGRESS_ENDPOINTS.len());
        assert_eq!(eps[0], "https://api.ipify.org");
    }

    #[test]
    fn overrides_take_precedence() {
        let eps = resolved_endpoints(&["https://example.test/ip".to_string()]);
        assert_eq!(eps, vec!["https://example.test/ip".to_string()]);
    }
}
