use super::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use wiremock::{matchers::any, Mock, MockServer, Request, ResponseTemplate};

struct World {
    records: Vec<Value>,
    requests: Vec<String>,
    probe: u16,
    fail_delete: bool,
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
    async fn new() -> Self {
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
                    if w.fail_delete {
                        return ResponseTemplate::new(503);
                    }
                    let id = path.rsplit('/').next().unwrap();
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
        let cfg = Config::parse_from([
            "beacon",
            "--sink",
            "cloudflare",
            "--record-name",
            "diagnostic.test",
            "--shared-record",
            "shared.test=mine",
            "--serving-probe-url",
            &format!("{}/serving", server.uri()),
            "--egress-endpoint",
            &format!("{}/wan", server.uri()),
            "--lan-ip",
            "192.0.2.1",
            "--state-file",
            &format!("/tmp/beacon-membership-{}.json", server.address().port()),
        ]);
        cfg.validate().unwrap();
        let client = build_probe_client(1).unwrap();
        let sink = CloudflareSink::new(
            client.clone(),
            "test".into(),
            "test".into(),
            "diagnostic.test".into(),
            false,
            vec![cloudflare::SharedRecordConfig {
                record_name: "shared.test".into(),
                owner: "mine".into(),
                refresh_secs: 300,
                stale_secs: 900,
            }],
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

    async fn probe(&self, membership: &mut state::Membership) -> Result<()> {
        serving_cycle(&self.cfg, &self.client, &self.sinks, membership).await
    }
    fn mine(&self) -> usize {
        self.world
            .lock()
            .unwrap()
            .records
            .iter()
            .filter(|r| {
                r["name"] == "shared.test"
                    && r["comment"]
                        .as_str()
                        .and_then(cloudflare::parse_owner_comment)
                        .is_some_and(|s| s.owner == "mine")
            })
            .count()
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.cfg.state_file);
    }
}

#[tokio::test]
async fn actual_cycles_join_withdraw_and_rejoin_on_unchanged_wan_preserving_other_owners() {
    let h = Harness::new().await;
    let mut membership = state::Membership::default();
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
    assert_eq!(membership.applied, Some(true));
}

#[tokio::test]
async fn startup_withdrawal_retries_failed_dns_despite_wan_failure() {
    let h = Harness::new().await;
    let mut membership = state::Membership::default();
    {
        let mut w = h.world.lock().unwrap();
        w.fail_delete = true;
        w.fail_wan = true;
        w.probe = 503;
    }
    assert!(cycle(&h.cfg, &h.client, &h.sinks, false).await.is_err());
    assert!(h.probe(&mut membership).await.is_err());
    assert_eq!(membership.applied, None);
    assert_eq!(h.mine(), 3);
    h.world.lock().unwrap().fail_delete = false;
    h.probe(&mut membership).await.unwrap();
    assert_eq!(h.mine(), 0);
    assert_eq!(membership.applied, Some(false));
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
    let mut membership = state::Membership::default();
    h.world.lock().unwrap().probe = 302;
    for _ in 0..2 {
        h.probe(&mut membership).await.unwrap();
    }
    assert!(!membership.serving);
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
    assert!(!membership.serving);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    h.cfg.serving_probe_url = Some(format!("http://{address}/serving"));
    h.probe(&mut membership).await.unwrap();
    assert!(!membership.serving);
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
