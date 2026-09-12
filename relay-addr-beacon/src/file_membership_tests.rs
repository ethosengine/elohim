//! The household membership leg, driven through the PRODUCTION cycle.
//!
//! These tests never call `FileMembershipSink::reconcile_membership` directly
//! (`sinks/file.rs` owns that unit surface). They drive `serving_cycle` — the
//! same function the daemon's `serving_loop` calls — so what is under test is
//! the whole decision: a real HTTP serving probe, `state::Membership`'s
//! join2/leave3 hysteresis, and the file projection beneath it. Two legs share
//! one document exactly as two doorways on one household host do.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use wiremock::{matchers::any, Mock, MockServer, Request, ResponseTemplate};

use super::*;
use sinks::file::MembershipDoc;

/// One beacon leg: its own probe endpoint, its own ephemeral membership
/// evidence, contributing to the shared document at `path`.
struct Leg {
    cfg: Config,
    sinks: Vec<ActiveSink>,
    client: reqwest::Client,
    membership: state::Membership,
    status: Arc<Mutex<u16>>,
    _server: MockServer,
}

impl Leg {
    async fn new(path: &Path, owner: &str, origin: &str) -> Self {
        let server = MockServer::start().await;
        let status = Arc::new(Mutex::new(200_u16));
        let observed = status.clone();
        Mock::given(any())
            .respond_with(move |_: &Request| {
                ResponseTemplate::new(*observed.lock().unwrap()).set_body_string("doorway")
            })
            .mount(&server)
            .await;

        let args: Vec<String> = vec![
            "beacon".into(),
            "--sink".into(),
            "file".into(),
            "--shared-record".into(),
            format!("elohim.local={owner}"),
            "--membership-file".into(),
            path.to_string_lossy().into_owned(),
            "--member-origin".into(),
            origin.into(),
            "--serving-probe-url".into(),
            format!("{}/health", server.uri()),
            "--serving-probe-interval-secs".into(),
            "1".into(),
        ];
        let cfg = Config::parse_from(args);
        cfg.validate().expect("household file leg should validate");
        let client = build_probe_client(cfg.serving_probe_interval_secs).unwrap();
        let sinks = build_sinks(&cfg, &client).unwrap();
        Self {
            cfg,
            sinks,
            client,
            membership: state::Membership::default(),
            status,
            _server: server,
        }
    }

    /// One health tick through the production cycle.
    async fn probe(&mut self) {
        serving_cycle(&self.cfg, &self.client, &self.sinks, &mut self.membership)
            .await
            .expect("serving cycle");
    }

    /// What this leg's doorway answers on `/health` from now on.
    fn answers(&self, status: u16) {
        *self.status.lock().unwrap() = status;
    }

    /// A process restart: serving evidence is ephemeral and does not survive
    /// it, so membership must be earned again.
    fn restart(&mut self) {
        self.membership = state::Membership::default();
    }
}

fn doc_path(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("beacon-household-{tag}-{nanos}/elohim.local.json"))
}

fn read(path: &Path) -> MembershipDoc {
    serde_json::from_str(&std::fs::read_to_string(path).expect("membership document")).unwrap()
}

fn owners(path: &Path) -> Vec<String> {
    read(path).members.into_iter().map(|m| m.owner).collect()
}

#[tokio::test]
async fn a_membership_only_leg_needs_no_address_detection() {
    let path = doc_path("no-address");
    let leg = Leg::new(&path, "alpha", "http://localhost:8888").await;
    assert!(!leg.sinks.iter().any(ActiveSink::needs_address));
}

#[tokio::test]
async fn join_two_leave_three_across_two_legs_sharing_one_document() {
    let path = doc_path("hysteresis");
    let mut alpha = Leg::new(&path, "alpha", "http://localhost:8888").await;
    let mut apex = Leg::new(&path, "apex", "http://localhost:8889").await;

    // One serving probe is evidence, not membership: a leg starts withdrawn.
    alpha.probe().await;
    apex.probe().await;
    assert!(owners(&path).is_empty());

    // The second consecutive serving probe earns the join (join_after = 2).
    alpha.probe().await;
    apex.probe().await;
    assert_eq!(owners(&path), vec!["alpha", "apex"]);

    let sibling_before = read(&path).member("alpha").cloned().unwrap();

    // The apex leg's doorway sheds. Two non-serving probes are NOT enough —
    // that is the whole point of leave_after = 3.
    apex.answers(503);
    apex.probe().await;
    assert!(read(&path).member("apex").is_some());
    apex.probe().await;
    assert!(read(&path).member("apex").is_some());

    // The third withdraws it — and only it. The sibling's entry is
    // byte-identical, stamp included: leg B never wrote leg A's record.
    apex.probe().await;
    let shed = read(&path);
    assert!(shed.member("apex").is_none());
    assert_eq!(shed.member("alpha").cloned().unwrap(), sibling_before);

    // Recovery: one serving probe is not yet a rejoin.
    apex.answers(200);
    apex.probe().await;
    assert!(read(&path).member("apex").is_none());

    // The second rejoins, without duplicating either owner.
    apex.probe().await;
    assert_eq!(owners(&path), vec!["alpha", "apex"]);
    assert_eq!(read(&path).members.len(), 2);
}

#[tokio::test]
async fn a_restarted_leg_withdraws_then_re_adopts_exactly_one_entry() {
    let path = doc_path("restart");
    let mut alpha = Leg::new(&path, "alpha", "http://localhost:8888").await;
    let mut apex = Leg::new(&path, "apex", "http://localhost:8889").await;
    for _ in 0..2 {
        alpha.probe().await;
        apex.probe().await;
    }
    assert_eq!(owners(&path), vec!["alpha", "apex"]);

    apex.restart();
    // A restarted leg's first act is to withdraw the entry it cannot yet
    // vouch for — the document must never outlive the evidence behind it.
    apex.probe().await;
    let after_restart = read(&path);
    assert!(after_restart.member("apex").is_none());
    assert!(after_restart.member("alpha").is_some());

    apex.probe().await;
    assert_eq!(owners(&path), vec!["alpha", "apex"]);
    assert_eq!(read(&path).members.len(), 2);
}

#[tokio::test]
async fn a_dead_probe_endpoint_withdraws_the_same_way_a_shed_one_does() {
    let path = doc_path("dead");
    let mut alpha = Leg::new(&path, "alpha", "http://localhost:8888").await;
    for _ in 0..2 {
        alpha.probe().await;
    }
    assert_eq!(owners(&path), vec!["alpha"]);

    // Silence is not serving: point the probe at a closed port.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    alpha.cfg.serving_probe_url = Some(format!("http://{address}/health"));
    for _ in 0..3 {
        alpha.probe().await;
    }
    assert!(owners(&path).is_empty());
}
