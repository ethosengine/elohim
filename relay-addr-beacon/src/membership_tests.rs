use super::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use wiremock::{matchers::any, Mock, MockServer, Request, ResponseTemplate};

struct World {
    records: Vec<Value>,
    requests: Vec<String>,
    probe: u16,
    fail_delete: bool,
    /// DELETE fails only for records at this name — one lane's projection
    /// failing while its sibling lane's succeeds.
    fail_delete_name: Option<String>,
    fail_wan: bool,
}

struct Harness {
    server: MockServer,
    world: Arc<Mutex<World>>,
    cfg: Config,
    sinks: Vec<ActiveSink>,
    client: reqwest::Client,
}

impl Harness {
    /// The single-lane harness every pre-existing scenario uses.
    async fn new() -> Self {
        Self::with_lanes(&["shared.test=mine"]).await
    }

    /// `lanes` are `<name>=<owner>` exactly as the flag spells them, so a
    /// scenario configures two lanes the way a manifest would.
    async fn with_lanes(lanes: &[&str]) -> Self {
        let server = MockServer::start().await;
        let stamp = cloudflare::format_owner_comment("mine", 1);
        let world = Arc::new(Mutex::new(World {
            records: vec![
                json!({"id":"exclusive","name":"diagnostic.test","type":"A","content":"203.0.113.7"}),
                json!({"id":"mine-a","name":"shared.test","type":"A","content":"203.0.113.7","comment":stamp}),
                json!({"id":"mine-duplicate","name":"shared.test","type":"A","content":"203.0.113.7","comment":stamp}),
                json!({"id":"mine-v6","name":"shared.test","type":"AAAA","content":"2001:db8::1","comment":stamp}),
                // Equal addresses never establish ownership.
                json!({"id":"sibling","name":"shared.test","type":"A","content":"203.0.113.7","comment":cloudflare::format_owner_comment("sibling", 1)}),
                json!({"id":"unowned","name":"shared.test","type":"A","content":"203.0.113.7"}),
            ],
            requests: vec![],
            probe: 200,
            fail_delete: false,
            fail_delete_name: None,
            fail_wan: false,
        }));
        let state = world.clone();
        Mock::given(any())
            .respond_with(move |request: &Request| {
                let mut w = state.lock().unwrap();
                let path = request.url.path();
                let method = request.method.as_str();
                w.requests.push(format!("{method} {path}"));
                if path == "/serving" {
                    return ResponseTemplate::new(w.probe)
                        .insert_header("location", "/healthy")
                        .set_body_string("not JSON");
                }
                if path == "/healthy" {
                    return ResponseTemplate::new(200);
                }
                if path == "/wan" {
                    return ResponseTemplate::new(if w.fail_wan { 503 } else { 200 })
                        .set_body_string("203.0.113.7");
                }
                let result = if path == "/zones" {
                    json!([{"id":"zone"}])
                } else if method == "GET" {
                    let query: std::collections::HashMap<_, _> =
                        request.url.query_pairs().collect();
                    json!(w
                        .records
                        .iter()
                        .filter(|r| r["name"] == query["name"].as_ref()
                            && r["type"] == query["type"].as_ref())
                        .cloned()
                        .collect::<Vec<_>>())
                } else if method == "DELETE" {
                    let id = path.rsplit('/').next().unwrap();
                    let lane_blocked = w.fail_delete_name.as_deref().is_some_and(|name| {
                        w.records.iter().any(|r| r["id"] == id && r["name"] == name)
                    });
                    if w.fail_delete || lane_blocked {
                        return ResponseTemplate::new(503);
                    }
                    w.records.retain(|r| r["id"] != id);
                    json!({"id":id})
                } else {
                    let mut body: Value = serde_json::from_slice(&request.body).unwrap();
                    let id = if method == "POST" {
                        format!("new-{}", w.requests.len())
                    } else {
                        path.rsplit('/').next().unwrap().to_string()
                    };
                    body["id"] = json!(id);
                    w.records.retain(|r| r["id"] != id);
                    w.records.push(body.clone());
                    body
                };
                ResponseTemplate::new(200).set_body_json(json!({"success":true,"result":result}))
            })
            .mount(&server)
            .await;
        let mut args: Vec<String> = vec![
            "beacon".into(),
            "--sink".into(),
            "cloudflare".into(),
            "--record-name".into(),
            "diagnostic.test".into(),
            "--serving-probe-url".into(),
            format!("{}/serving", server.uri()),
            "--egress-endpoint".into(),
            format!("{}/wan", server.uri()),
            "--lan-ip".into(),
            "192.0.2.1".into(),
            "--state-file".into(),
            format!("/tmp/beacon-membership-{}.json", server.address().port()),
        ];
        for lane in lanes {
            args.push("--shared-record".into());
            args.push((*lane).into());
        }
        let cfg = Config::parse_from(args);
        cfg.validate().unwrap();
        let client = build_probe_client(1).unwrap();
        let sink = CloudflareSink::new(
            client.clone(),
            "test".into(),
            "test".into(),
            "diagnostic.test".into(),
            false,
            cfg.shared_record_lanes()
                .unwrap()
                .into_iter()
                .map(|lane| cloudflare::SharedRecordConfig {
                    record_name: lane.record_name,
                    owner: lane.owner,
                    refresh_secs: 300,
                    stale_secs: 900,
                })
                .collect(),
        )
        .with_api_base(server.uri())
        .with_serving_probe(true);
        Self {
            server,
            world,
            cfg,
            sinks: vec![ActiveSink::Cloudflare(sink)],
            client,
        }
    }

    async fn probe(&self, membership: &mut state::MembershipSet) -> Result<()> {
        serving_cycle(&self.cfg, &self.client, &self.sinks, membership).await
    }

    /// Per-lane serving evidence for exactly the lanes this harness configured.
    fn membership(&self) -> state::MembershipSet {
        state::MembershipSet::new(&self.cfg.shared_record_lanes().unwrap())
    }

    fn mine(&self) -> usize {
        self.mine_at("shared.test")
    }

    /// Records at `name` whose ownership comment resolves to owner `mine`.
    fn mine_at(&self, name: &str) -> usize {
        self.world
            .lock()
            .unwrap()
            .records
            .iter()
            .filter(|r| {
                r["name"] == name
                    && r["comment"]
                        .as_str()
                        .and_then(cloudflare::parse_owner_comment)
                        .is_some_and(|s| s.owner == "mine")
            })
            .count()
    }

    fn record(&self, id: &str) -> Option<Value> {
        self.world
            .lock()
            .unwrap()
            .records
            .iter()
            .find(|r| r["id"] == id)
            .cloned()
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.cfg.state_file);
    }
}

/// Read one lane's decided verdict out of the per-lane evidence.
fn lane_serving(membership: &state::MembershipSet, name: &str) -> bool {
    membership
        .lane(name)
        .expect("lane is configured")
        .membership
        .serving
}

/// Read one lane's applied-projection marker.
fn lane_applied(membership: &state::MembershipSet, name: &str) -> Option<bool> {
    membership
        .lane(name)
        .expect("lane is configured")
        .membership
        .applied
}

#[tokio::test]
async fn actual_cycles_join_withdraw_and_rejoin_on_unchanged_wan_preserving_other_owners() {
    let h = Harness::new().await;
    let mut membership = h.membership();
    assert!(cycle(&h.cfg, &h.client, &h.sinks, false).await.unwrap());
    // Address publication before first health tick cannot re-advertise us.
    h.probe(&mut membership).await.unwrap();
    assert_eq!(h.mine(), 0);
    h.probe(&mut membership).await.unwrap();
    assert_eq!(h.mine(), 1);
    h.world.lock().unwrap().probe = 503;
    for _ in 0..2 {
        h.probe(&mut membership).await.unwrap();
        assert_eq!(h.mine(), 1);
    }
    let before = h
        .world
        .lock()
        .unwrap()
        .records
        .iter()
        .filter(|r| r["id"] == "exclusive" || r["id"] == "sibling" || r["id"] == "unowned")
        .cloned()
        .collect::<Vec<_>>();
    h.probe(&mut membership).await.unwrap();
    assert_eq!(h.mine(), 0);
    assert!(cycle(&h.cfg, &h.client, &h.sinks, false).await.unwrap());
    assert_eq!(h.mine(), 0);
    let after = h
        .world
        .lock()
        .unwrap()
        .records
        .iter()
        .filter(|r| r["id"] == "exclusive" || r["id"] == "sibling" || r["id"] == "unowned")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(before, after);
    h.world.lock().unwrap().probe = 200;
    h.probe(&mut membership).await.unwrap();
    assert_eq!(h.mine(), 0);
    h.probe(&mut membership).await.unwrap();
    assert_eq!(h.mine(), 1);
    assert_eq!(lane_applied(&membership, "shared.test"), Some(true));
}

#[tokio::test]
async fn startup_withdrawal_retries_failed_dns_despite_wan_failure() {
    let h = Harness::new().await;
    let mut membership = h.membership();
    {
        let mut w = h.world.lock().unwrap();
        w.fail_delete = true;
        w.fail_wan = true;
        w.probe = 503;
    }
    assert!(cycle(&h.cfg, &h.client, &h.sinks, false).await.is_err());
    assert!(h.probe(&mut membership).await.is_err());
    assert_eq!(lane_applied(&membership, "shared.test"), None);
    assert_eq!(h.mine(), 3);
    h.world.lock().unwrap().fail_delete = false;
    h.probe(&mut membership).await.unwrap();
    assert_eq!(h.mine(), 0);
    assert_eq!(lane_applied(&membership, "shared.test"), Some(false));
    let w = h.world.lock().unwrap();
    assert!(!w
        .requests
        .iter()
        .any(|s| s == "DELETE /zones/zone/dns_records/sibling"
            || s == "DELETE /zones/zone/dns_records/exclusive"
            || s == "DELETE /zones/zone/dns_records/unowned"));
}

#[tokio::test]
async fn redirects_timeout_and_refusal_do_not_supply_join_evidence() {
    let mut h = Harness::new().await;
    cycle(&h.cfg, &h.client, &h.sinks, false).await.unwrap();
    let mut membership = h.membership();
    h.world.lock().unwrap().probe = 302;
    for _ in 0..2 {
        h.probe(&mut membership).await.unwrap();
    }
    assert!(!lane_serving(&membership, "shared.test"));
    assert!(!h
        .world
        .lock()
        .unwrap()
        .requests
        .iter()
        .any(|r| r == "GET /healthy"));
    Mock::given(wiremock::matchers::path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .with_priority(1)
        .mount(&h.server)
        .await;
    h.cfg.serving_probe_url = Some(format!("{}/slow", h.server.uri()));
    h.probe(&mut membership).await.unwrap();
    assert!(!lane_serving(&membership, "shared.test"));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    h.cfg.serving_probe_url = Some(format!("http://{address}/serving"));
    h.probe(&mut membership).await.unwrap();
    assert!(!lane_serving(&membership, "shared.test"));
}

#[test]
fn positive_counts_and_hysteresis_reset_are_required() {
    for exclusive in ["shared.test", "SHARED.TEST."] {
        let cfg = Config::parse_from([
            "beacon",
            "--sink",
            "cloudflare",
            "--record-name",
            exclusive,
            "--shared-record",
            "shared.test=mine",
            "--serving-probe-url",
            "http://localhost/serving",
        ]);
        assert!(cfg.validate().is_err());
    }
    for flag in [
        "--serving-probe-interval-secs",
        "--serving-leave-after",
        "--serving-join-after",
    ] {
        let cfg = Config::parse_from(["beacon", flag, "0"]);
        assert!(cfg.validate().is_err());
    }
    let mut membership = state::Membership::default();
    for value in [true, false, true] {
        membership.observe(value, 3, 2);
    }
    assert!(!membership.serving);
    membership.observe(true, 3, 2);
    assert!(membership.serving);
    for value in [false, false, true, false, false] {
        membership.observe(value, 3, 2);
    }
    assert!(membership.serving);
    membership.observe(false, 3, 2);
    assert!(!membership.serving);
}

/// TWO Cloudflare lanes on one beacon leg, driven through the production
/// serving cycle: the apex name and the doorway-set name are separate lanes
/// with separate evidence, and one probe result reconciles both.
#[tokio::test]
async fn two_cloudflare_lanes_join_and_withdraw_independently_through_the_production_cycle() {
    let h = Harness::with_lanes(&["shared.test=mine", "apex.test=mine"]).await;
    // A sibling already contributes to the apex lane; it must survive
    // everything this leg does (its modified_on is absent, so it is
    // fail-safe not reapable).
    h.world.lock().unwrap().records.push(json!({
        "id": "apex-sibling",
        "name": "apex.test",
        "type": "A",
        "content": "198.51.100.9",
        "comment": cloudflare::format_owner_comment("sibling", 1),
    }));
    let sibling_before = h.record("apex-sibling").unwrap();

    let mut membership = h.membership();
    assert_eq!(membership.lanes().len(), 2);
    assert!(cycle(&h.cfg, &h.client, &h.sinks, false).await.unwrap());

    // Both lanes start withdrawn: one serving probe is evidence, not membership.
    h.probe(&mut membership).await.unwrap();
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (0, 0));

    // The second consecutive serving probe earns BOTH lanes (join_after = 2).
    h.probe(&mut membership).await.unwrap();
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (1, 1));
    assert_eq!(lane_applied(&membership, "shared.test"), Some(true));
    assert_eq!(lane_applied(&membership, "apex.test"), Some(true));

    // The doorway sheds. leave_after = 3, so two non-serving probes hold both
    // lanes, and the third withdraws both — this leg's contribution to each
    // name, and nothing else.
    h.world.lock().unwrap().probe = 503;
    for _ in 0..2 {
        h.probe(&mut membership).await.unwrap();
        assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (1, 1));
    }
    h.probe(&mut membership).await.unwrap();
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (0, 0));
    assert_eq!(lane_applied(&membership, "apex.test"), Some(false));

    // An address cycle after withdrawal must not re-advertise either lane.
    assert!(cycle(&h.cfg, &h.client, &h.sinks, false).await.unwrap());
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (0, 0));

    // Rejoin: again both lanes, again only on the second serving probe.
    h.world.lock().unwrap().probe = 200;
    h.probe(&mut membership).await.unwrap();
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (0, 0));
    h.probe(&mut membership).await.unwrap();
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (1, 1));

    // The sibling's apex record was never patched, reordered or reaped.
    assert_eq!(h.record("apex-sibling"), Some(sibling_before));
}

/// A lane whose projection FAILS retries on its own and is not recorded as
/// applied, while its sibling lane's successful projection stands. This is the
/// whole reason the counters and the applied marker are held per lane rather
/// than one verdict written twice.
#[tokio::test]
async fn one_lane_failing_neither_marks_itself_applied_nor_suppresses_the_other() {
    let h = Harness::with_lanes(&["shared.test=mine", "apex.test=mine"]).await;
    h.world.lock().unwrap().records.push(json!({
        "id": "apex-mine",
        "name": "apex.test",
        "type": "A",
        "content": "203.0.113.7",
        "comment": cloudflare::format_owner_comment("mine", 1),
    }));
    let mut membership = h.membership();
    assert!(cycle(&h.cfg, &h.client, &h.sinks, false).await.unwrap());
    for _ in 0..2 {
        h.probe(&mut membership).await.unwrap();
    }
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (1, 1));

    // Withdrawal DELETEs. Break it for the apex lane ONLY, then shed.
    {
        let mut w = h.world.lock().unwrap();
        w.fail_delete_name = Some("apex.test".into());
        w.probe = 503;
    }
    // leave_after = 3: the first two probes still carry a JOIN verdict, which
    // writes no DELETE and so cannot fail.
    for _ in 0..2 {
        h.probe(&mut membership).await.unwrap();
    }
    // The third flips both lanes to withdrawn. One lane projects it; the other
    // cannot — and the cycle reports the incomplete pass.
    assert!(h.probe(&mut membership).await.is_err());
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (0, 1));
    // The lane that succeeded recorded its withdrawal...
    assert_eq!(lane_applied(&membership, "shared.test"), Some(false));
    // ...the lane that failed did NOT, so its next tick retries...
    assert_eq!(lane_applied(&membership, "apex.test"), Some(true));
    // ...and both still hold the decided verdict, so that retry is a withdrawal.
    assert!(!lane_serving(&membership, "shared.test"));
    assert!(!lane_serving(&membership, "apex.test"));

    h.world.lock().unwrap().fail_delete_name = None;
    h.probe(&mut membership).await.unwrap();
    assert_eq!((h.mine_at("shared.test"), h.mine_at("apex.test")), (0, 0));
    assert_eq!(lane_applied(&membership, "apex.test"), Some(false));
}
