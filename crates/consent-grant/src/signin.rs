//! Signing in to one's own node: the rules, in the OAuth model.
//!
//! The node is the authorization server for its own person. A sign-in word
//! and a secret prove the person once; the signed-in device then acts with
//! their authority, with no proof asked per act. Noticing that something is
//! off is a witness's job at a witnessed moment ([`crate::witness`]), which
//! may pause until the person signs in again.
//!
//! What lives here is what decides, with no socket, clock or store of its
//! own (the host passes the time in):
//!
//! - [`may_make_node_sign`]: who may make the node sign as its person.
//! - [`channel`]: how a sign-in arrived ([`Channel`]), and
//!   [`sign_in_channel_verdict`]: whether that channel may carry it. Open to a
//!   plain channel today, by operator ruling; a must-have before the floor is
//!   ready.
//! - [`AttemptLimiter`]: how often a sign-in may be tried, per source and for
//!   the node's one account, with a growing delay.
//! - [`SignInRefusal`]: every way a sign-in is refused, by name. A wrong word
//!   and a wrong secret are one refusal, so a caller cannot tell them apart.
//!
//! The credential itself is private to the node: never on the DHT, a source
//! chain, the declaration, a sync or a projection. Losing the secret is
//! recovered from the node's own machine, which is the deterministic floor:
//! whoever is at the machine can set a new one.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;

/// How long a session proven by sign-in lives: thirty days, unless signed out
/// or ended by a new secret.
pub const SESSION_LIFE_MICROS: i64 = 30 * 24 * 60 * 60 * 1_000_000;

/// Who may make the node sign as its person: a caller on this machine, or a
/// request carrying a session proven by sign-in for this node's own person.
/// Nothing else: a session minted any other way proves nobody.
pub fn may_make_node_sign(caller_is_local: bool, proven_by_signin: bool) -> bool {
    caller_is_local || proven_by_signin
}

/// How a request reached the node, as far as the node can know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// From this machine. A password never left it.
    Loopback,
    /// From elsewhere, through a proxy this node was told to trust, which says
    /// it took the request over TLS.
    Tls,
    /// From elsewhere, in the clear or by a path the node cannot vouch for.
    Plain,
}

/// Classify how a request arrived.
///
/// The node terminates no TLS itself, so the only TLS it can know of is a
/// proxy's: `peer` must be one of `trusted_proxies` (named by the operator of
/// the node) and the last `X-Forwarded-Proto` value that proxy appended must
/// be `https`. A forwarded header from any other peer is ignored: a caller on
/// the same network can send one, and by doing so exposes only the secret it
/// sends itself.
pub fn channel(peer: IpAddr, forwarded_proto: Option<&str>, trusted_proxies: &[IpAddr]) -> Channel {
    if peer.is_loopback() {
        return Channel::Loopback;
    }
    let says_tls = forwarded_proto
        .and_then(|v| v.rsplit(',').next())
        .is_some_and(|last| last.trim().eq_ignore_ascii_case("https"));
    if says_tls && trusted_proxies.contains(&peer) {
        Channel::Tls
    } else {
        Channel::Plain
    }
}

/// Whether a sign-in from another machine is allowed in the clear. Today it
/// is (`true`): operator ruling 2026-10-05, accepting the risk while no real
/// secret or real network is at stake. It is a must-have before the floor is
/// ready: a sign-in secret and a session cookie must not cross a network in
/// the clear (the security backlog entry, device-recognition row 1, and the
/// confidentiality-plane cluster record it). Making it refuse is this one
/// line.
pub const PLAIN_SIGN_IN_ALLOWED: bool = true;

/// The secure-channel verdict for a sign-in: `Ok(in_the_clear)` when it may
/// go on (`true` when a secret and a cookie cross a network unencrypted, which
/// the host logs), or the refusal. Deliberately open today
/// ([`PLAIN_SIGN_IN_ALLOWED`]).
pub fn sign_in_channel_verdict(channel: Channel) -> Result<bool, SignInRefusal> {
    channel_verdict(channel, PLAIN_SIGN_IN_ALLOWED)
}

fn channel_verdict(channel: Channel, plain_allowed: bool) -> Result<bool, SignInRefusal> {
    match channel {
        Channel::Loopback | Channel::Tls => Ok(false),
        Channel::Plain if plain_allowed => Ok(true),
        Channel::Plain => Err(SignInRefusal::NeedsSecureChannel),
    }
}

/// Whether a sign-in must bind its session to a key the browser holds
/// ([`crate::dpop`]). Over TLS it must: a browser on a secure page can always
/// hold one, and then a copied cookie is worth nothing on the routes that make
/// the node sign. Over plain http from another machine it may not (the
/// browser cannot hold one there): allowed unbound, and `Ok(true)` says so for
/// the host to log once. On this machine either is fine: this-machine acts
/// need no session at all.
pub fn sign_in_key_rule(channel: Channel, offers_key: bool) -> Result<bool, SignInRefusal> {
    match (channel, offers_key) {
        (_, true) => Ok(false),
        (Channel::Tls, false) => Err(SignInRefusal::NeedsSessionKey),
        (Channel::Plain, false) => Ok(true),
        (Channel::Loopback, false) => Ok(false),
    }
}

/// Every way a sign-in is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInRefusal {
    /// From another machine, not over TLS. Not produced while
    /// [`PLAIN_SIGN_IN_ALLOWED`] holds.
    NeedsSecureChannel,
    /// Over TLS, with no session key offered.
    NeedsSessionKey,
    /// Too many recent failures; try again after this many seconds.
    Slowed { retry_after_secs: u64 },
    /// No sign-in secret is set on this node.
    SecretUnset,
    /// The word or the secret is wrong; which one is never said.
    InvalidCredentials,
    /// A witness attending the person paused the sign-in.
    Paused { reason: String },
}

impl SignInRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NeedsSecureChannel => "signin_needs_secure_channel",
            Self::NeedsSessionKey => "signin_needs_session_key",
            Self::Slowed { .. } => "signin_slowed",
            Self::SecretUnset => "signin_secret_unset",
            // The doorway's code, so one form reads both.
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::Paused { .. } => "signin_paused",
        }
    }

    /// Plain words for the person.
    pub fn words(&self) -> String {
        match self {
            Self::NeedsSecureChannel => "signing in from another machine needs a secure \
                                         connection (https) to this node; sign in on the node's \
                                         own machine, or reach it through its https address"
                .to_string(),
            Self::NeedsSessionKey => "this browser did not offer a key to bind the sign-in to; \
                                      reload the page and sign in again"
                .to_string(),
            Self::Slowed { retry_after_secs } => {
                format!("too many sign-in attempts; try again in {retry_after_secs} seconds")
            }
            Self::SecretUnset => "no sign-in secret is set on this node; on the node's own \
                                  machine, run: epr identity secret"
                .to_string(),
            Self::InvalidCredentials => "Invalid credentials".to_string(),
            Self::Paused { reason } => {
                format!("a witness attending you paused this sign-in: {reason}")
            }
        }
    }
}

/// Failures a source may make before it is slowed.
pub const FREE_FAILURES_PER_SOURCE: u32 = 3;
/// Failures the account may take, from all sources, before every sign-in is
/// slowed.
pub const FREE_FAILURES_PER_ACCOUNT: u32 = 10;
/// The longest wait a failure can impose.
pub const MAX_DELAY_SECS: u64 = 15 * 60;
/// Sources remembered at once; the oldest is forgotten past this.
pub const MAX_SOURCES: usize = 1024;

#[derive(Debug, Clone, Copy, Default)]
struct Failures {
    count: u32,
    last_micros: i64,
}

impl Failures {
    /// Seconds still to wait at `now`, when over `free`. The delay doubles
    /// with every failure past `free`, up to [`MAX_DELAY_SECS`]; a quiet
    /// [`MAX_DELAY_SECS`] since the last failure forgives them all.
    fn wait(&self, free: u32, now_micros: i64) -> Option<u64> {
        let quiet = now_micros.saturating_sub(self.last_micros);
        if self.count < free || quiet >= (MAX_DELAY_SECS as i64) * 1_000_000 {
            return None;
        }
        let over = (self.count - free).min(20);
        let delay = (1u64 << over).min(MAX_DELAY_SECS);
        let elapsed = (quiet / 1_000_000).max(0) as u64;
        (elapsed < delay).then(|| delay - elapsed)
    }

    fn fresh(&self, now_micros: i64) -> bool {
        now_micros.saturating_sub(self.last_micros) < (MAX_DELAY_SECS as i64) * 1_000_000
    }
}

/// How often a sign-in may be tried. In memory: a restart forgets it, which
/// costs an attacker a restart they cannot cause from outside.
#[derive(Debug, Default)]
pub struct AttemptLimiter {
    inner: Mutex<LimiterInner>,
}

#[derive(Debug, Default)]
struct LimiterInner {
    sources: HashMap<String, Failures>,
    account: Failures,
}

impl AttemptLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, LimiterInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Whether `source` may try now; when not, how long to wait. Checking
    /// counts nothing.
    pub fn check(&self, source: &str, now_micros: i64) -> Result<(), SignInRefusal> {
        let inner = self.lock();
        let by_source = inner
            .sources
            .get(source)
            .and_then(|f| f.wait(FREE_FAILURES_PER_SOURCE, now_micros));
        let by_account = inner.account.wait(FREE_FAILURES_PER_ACCOUNT, now_micros);
        match by_source.max(by_account) {
            Some(retry_after_secs) => Err(SignInRefusal::Slowed { retry_after_secs }),
            None => Ok(()),
        }
    }

    /// Record a failed attempt from `source`.
    pub fn failed(&self, source: &str, now_micros: i64) {
        let mut inner = self.lock();
        // bounded-work: forget stale sources, then the oldest past the cap.
        if inner.sources.len() >= MAX_SOURCES && !inner.sources.contains_key(source) {
            inner.sources.retain(|_, f| f.fresh(now_micros));
            if inner.sources.len() >= MAX_SOURCES {
                if let Some(oldest) = inner
                    .sources
                    .iter()
                    .min_by_key(|(_, f)| f.last_micros)
                    .map(|(k, _)| k.clone())
                {
                    inner.sources.remove(&oldest);
                }
            }
        }
        let bump = |f: &mut Failures| {
            if !f.fresh(now_micros) {
                f.count = 0;
            }
            f.count = f.count.saturating_add(1);
            f.last_micros = now_micros;
        };
        bump(inner.sources.entry(source.to_string()).or_default());
        bump(&mut inner.account);
    }

    /// A sign-in from `source` succeeded: its failures and the account's are
    /// forgiven.
    pub fn succeeded(&self, source: &str) {
        let mut inner = self.lock();
        inner.sources.remove(source);
        inner.account = Failures::default();
    }

    pub fn sources(&self) -> usize {
        self.lock().sources.len()
    }
}

/// What the witness of a sign-in is handed: the word claimed and how the
/// request arrived. The secret is never among the claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInClaims {
    pub identifier: String,
    pub channel: Channel,
}

/// Run the sign-in witnessed moment. The default witness proceeds; a pause
/// refuses the sign-in with its reason. A sign-in witness adds no signature.
pub fn attend_sign_in(
    witness: &dyn crate::witness::WitnessedMoment<SignInClaims, ()>,
    claims: &SignInClaims,
) -> Result<(), SignInRefusal> {
    use crate::witness::{MomentKind, Witnessing};
    match witness.attend(MomentKind::SignIn, claims) {
        Witnessing::Proceed | Witnessing::ProceedWithSignatures(_) => Ok(()),
        Witnessing::PauseForReauthentication { reason } => Err(SignInRefusal::Paused {
            reason: crate::witness::plain_reason(&reason),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: i64 = 1_000_000;

    #[test]
    fn only_this_machine_or_a_proven_sign_in_makes_the_node_sign() {
        assert!(may_make_node_sign(true, false));
        assert!(may_make_node_sign(false, true));
        assert!(!may_make_node_sign(false, false));
    }

    #[test]
    fn tls_is_known_only_from_a_trusted_proxy_that_says_so() {
        let proxy: IpAddr = "10.0.0.2".parse().unwrap();
        let lan: IpAddr = "10.0.0.9".parse().unwrap();
        let local: IpAddr = "127.0.0.1".parse().unwrap();
        let six: IpAddr = "::1".parse().unwrap();
        assert_eq!(channel(local, None, &[]), Channel::Loopback);
        assert_eq!(channel(six, None, &[]), Channel::Loopback);
        assert_eq!(channel(proxy, Some("https"), &[proxy]), Channel::Tls);
        assert_eq!(channel(proxy, Some("http, https"), &[proxy]), Channel::Tls);
        assert_eq!(
            channel(proxy, Some("https, http"), &[proxy]),
            Channel::Plain
        );
        assert_eq!(channel(proxy, None, &[proxy]), Channel::Plain);
        // A header from a peer nobody named is ignored.
        assert_eq!(channel(lan, Some("https"), &[proxy]), Channel::Plain);
        assert_eq!(channel(lan, Some("https"), &[]), Channel::Plain);
    }

    #[test]
    fn plain_sign_in_is_allowed_today_and_flagged_as_in_the_clear() {
        assert_eq!(sign_in_channel_verdict(Channel::Plain), Ok(true));
        assert_eq!(sign_in_channel_verdict(Channel::Tls), Ok(false));
        assert_eq!(sign_in_channel_verdict(Channel::Loopback), Ok(false));
    }

    #[test]
    fn over_tls_a_sign_in_binds_a_key_and_in_the_clear_it_may_not() {
        assert_eq!(sign_in_key_rule(Channel::Tls, true), Ok(false));
        assert_eq!(
            sign_in_key_rule(Channel::Tls, false),
            Err(SignInRefusal::NeedsSessionKey)
        );
        assert_eq!(
            SignInRefusal::NeedsSessionKey.code(),
            "signin_needs_session_key"
        );
        assert_eq!(sign_in_key_rule(Channel::Plain, false), Ok(true));
        assert_eq!(sign_in_key_rule(Channel::Plain, true), Ok(false));
        assert_eq!(sign_in_key_rule(Channel::Loopback, false), Ok(false));
    }

    #[test]
    fn closing_the_plain_channel_refuses_it_by_name() {
        assert_eq!(
            channel_verdict(Channel::Plain, false),
            Err(SignInRefusal::NeedsSecureChannel)
        );
        assert_eq!(channel_verdict(Channel::Tls, false), Ok(false));
        assert_eq!(channel_verdict(Channel::Loopback, false), Ok(false));
        assert_eq!(
            SignInRefusal::NeedsSecureChannel.code(),
            "signin_needs_secure_channel"
        );
    }

    #[test]
    fn failures_slow_a_source_then_the_account_with_a_growing_delay() {
        let l = AttemptLimiter::new();
        let t = 1_000 * S;
        for _ in 0..FREE_FAILURES_PER_SOURCE {
            assert_eq!(l.check("a", t), Ok(()));
            l.failed("a", t);
        }
        // One past the free failures: wait one second, then two, then four.
        assert_eq!(
            l.check("a", t),
            Err(SignInRefusal::Slowed {
                retry_after_secs: 1
            })
        );
        assert_eq!(l.check("a", t + S), Ok(()));
        l.failed("a", t + S);
        assert_eq!(
            l.check("a", t + S),
            Err(SignInRefusal::Slowed {
                retry_after_secs: 2
            })
        );
        // Another source is not slowed by "a" alone.
        assert_eq!(l.check("b", t + S), Ok(()));
        // Many sources together slow the account for everyone.
        for n in 0..FREE_FAILURES_PER_ACCOUNT {
            l.failed(&format!("s{n}"), t + S);
        }
        assert!(matches!(
            l.check("fresh", t + S),
            Err(SignInRefusal::Slowed { .. })
        ));
        // A quiet spell forgives; a success forgives at once.
        assert_eq!(
            l.check("fresh", t + S + (MAX_DELAY_SECS as i64) * S),
            Ok(())
        );
        l.succeeded("a");
        assert_eq!(l.check("a", t + S), Ok(()));
    }

    #[test]
    fn the_delay_never_exceeds_its_ceiling_and_sources_stay_bounded() {
        let l = AttemptLimiter::new();
        for _ in 0..200 {
            l.failed("a", 0);
        }
        let Err(SignInRefusal::Slowed { retry_after_secs }) = l.check("a", 0) else {
            panic!("slowed")
        };
        assert_eq!(retry_after_secs, MAX_DELAY_SECS);
        for n in 0..(MAX_SOURCES + 10) {
            l.failed(&format!("s{n}"), n as i64);
        }
        assert!(l.sources() <= MAX_SOURCES);
    }

    #[test]
    fn a_wrong_word_and_a_wrong_secret_are_one_refusal() {
        let wrong = SignInRefusal::InvalidCredentials;
        assert_eq!(wrong.code(), "INVALID_CREDENTIALS");
        assert_eq!(wrong.words(), "Invalid credentials");
        assert!(SignInRefusal::SecretUnset
            .words()
            .contains("epr identity secret"));
        assert!(SignInRefusal::NeedsSecureChannel.words().contains("https"));
    }

    struct Pauses;
    impl crate::witness::WitnessedMoment<SignInClaims, ()> for Pauses {
        fn attend(
            &self,
            kind: crate::witness::MomentKind,
            claims: &SignInClaims,
        ) -> crate::witness::Witnessing<()> {
            assert_eq!(kind, crate::witness::MomentKind::SignIn);
            assert_eq!(claims.identifier, "matthew");
            crate::witness::Witnessing::PauseForReauthentication {
                reason: "an unusual place to sign in from".into(),
            }
        }
    }

    #[test]
    fn a_sign_in_is_a_witnessed_moment_that_proceeds_by_default() {
        let claims = SignInClaims {
            identifier: "matthew".into(),
            channel: Channel::Tls,
        };
        assert_eq!(attend_sign_in(&crate::witness::Unattended, &claims), Ok(()));
        assert_eq!(
            attend_sign_in(&Pauses, &claims),
            Err(SignInRefusal::Paused {
                reason: "an unusual place to sign in from".into()
            })
        );
    }
}
