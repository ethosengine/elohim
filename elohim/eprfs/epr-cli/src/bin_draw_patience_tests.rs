//! Row 35 B1: an admitted offer whose holder does not answer is waited for, within the bound the
//! pool policy declares; every other gap falls back at once, and a mismatch still refuses.

use super::sibling;
use super::tests::{
    glibc_at_least, host_with_glibc, offer_for, serving, sibling_bins, stating, two_bin_store,
};
use elohim_device_runtime::bin_store::Store;
use serde_json::{json, Value};
use std::path::Path;

/// A fetch gap as `epr artifact fetch` prints it (artifact.rs `gap`), which is what the draw's
/// fetch reads back from `fetch_output`.
fn printed(gap: &eprfs_peer::FetchGap) -> String {
    format!("fetch gap: {}: {gap}", gap.kind())
}

/// Every fetch gap the daemon answers with, one of each kind.
fn every_fetch_gap() -> Vec<eprfs_peer::FetchGap> {
    use eprfs_peer::FetchGap;
    let cid = eprfs_core::BlobCid::compute_bytes(b"an output");
    let device = "did:key:z6MkSibling".to_owned();
    vec![
        FetchGap::NoRecord,
        FetchGap::Mismatch {
            outputs: vec![cid.clone()],
        },
        FetchGap::ExpectedOutputUnavailable {
            output: cid.clone(),
        },
        FetchGap::LocalOnlyCustody {
            outputs: vec![cid.clone()],
        },
        FetchGap::NoRoute {
            device: device.clone(),
        },
        FetchGap::Refused {
            device,
            refusal: "not granted".into(),
        },
        FetchGap::ManifestTooLarge { len: 9, cap: 8 },
        FetchGap::ManifestRejected {
            reason: "names another key".into(),
        },
        FetchGap::DeadlineExceeded {
            fetched: 1,
            total: 2,
        },
        FetchGap::IntegrityFailed { cid },
        FetchGap::DaemonDown,
        FetchGap::TransportCannotDeliver {
            mode: "relay".into(),
            cause: Some("no manifest answer within 10.0 s".into()),
        },
        FetchGap::LocalStore {
            reason: "disk full".into(),
        },
        FetchGap::InstallRefused {
            dest: "/x".into(),
            reason: "outside the scope".into(),
        },
        FetchGap::Cancelled,
    ]
}

/// A clock that moves only when the draw sleeps, and every pause it slept.
struct Clock {
    base: std::time::Instant,
    offset: std::cell::Cell<std::time::Duration>,
    slept: std::cell::RefCell<Vec<u64>>,
}

impl Clock {
    fn new() -> Clock {
        Clock {
            base: std::time::Instant::now(),
            offset: std::cell::Cell::new(std::time::Duration::ZERO),
            slept: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn now(&self) -> std::time::Instant {
        self.base + self.offset.get()
    }

    fn sleep(&self, pause: std::time::Duration) {
        self.slept.borrow_mut().push(pause.as_secs());
        self.offset.set(self.offset.get() + pause);
    }

    fn slept(&self) -> Vec<u64> {
        self.slept.borrow().clone()
    }
}

/// The store's own locks are free: a follow, land or session open on this store could take
/// each of them now.
fn assert_store_unlocked(root: &Path) {
    use rustix::fs::{flock, FlockOperation};
    std::fs::create_dir_all(root).unwrap();
    for name in [".lock", ".follow.lock"] {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join(name))
            .unwrap();
        flock(&file, FlockOperation::NonBlockingLockExclusive)
            .unwrap_or_else(|e| panic!("{name} is held across the wait: {e}"));
    }
}

/// The commit the patience tests draw for.
const DRAWN: &str = "abababababababababababababababababababab";

/// Draw `offer` for [`DRAWN`] through [`sibling::draw_first`] on `host`, with `patience`.
fn draw_with(
    store: &Store,
    repo: &Path,
    offer: &sibling::Offer,
    host: &host_contract::Provides,
    fetch: &dyn Fn(&str, &Path) -> Result<(), String>,
    staged: &sibling::Admit<'_>,
    patience: &sibling::Patience<'_>,
) -> Result<Value, sibling::Draw> {
    sibling::draw_first(
        store,
        repo,
        DRAWN,
        &repo.join("incoming"),
        std::slice::from_ref(offer),
        &sibling::Judges {
            admit: &|_, document| sibling::admit_on(host, document),
            fetch,
            staged,
            patience,
        },
    )
}

/// The kinds a draw waits on: a holder it could not dial or whose direct stream failed
/// (`no-route`), and a holder reached only by relay that did not deliver
/// (`transport-cannot-deliver`; shem's ruling B-b: the two devices hear each other through
/// the relay, so a restarting holder can read either way).
fn waited_on(gap: &eprfs_peer::FetchGap) -> bool {
    matches!(
        gap,
        eprfs_peer::FetchGap::NoRoute { .. } | eprfs_peer::FetchGap::TransportCannotDeliver { .. }
    )
}

/// A relay-only holder that did not deliver, as the daemon reports it.
fn relay_undelivered(device: &str) -> String {
    printed(&eprfs_peer::FetchGap::TransportCannotDeliver {
        mode: "relay".into(),
        cause: Some(format!("no manifest answer from {device} within 10.0 s")),
    })
}

#[test]
fn only_a_holder_that_did_not_answer_is_waited_for() {
    // Row 35 B1: `no-route` is the kind the daemon gives a holder it could not dial (refused,
    // unreachable, the dial's own bound elapsed) or whose direct stream failed;
    // `transport-cannot-deliver` is a holder reached only by relay that did not deliver.
    for gap in every_fetch_gap() {
        let error = printed(&gap);
        assert_eq!(
            sibling::holder_unanswered(&error),
            waited_on(&gap),
            "{error}"
        );
    }
    // This device's own daemon not answering the queue is not a holder's silence.
    for error in [
        "fetch gap: deadline-exceeded: this device's epr peer serve did not answer the \
         queued fetch within 1860 s; it was withdrawn",
        "fetch gap: no-record: the flow store does not read",
        "fetch gap: local-store: the scope does not resolve",
        "invalid arguments: --output must be a CID",
        "no-route: not printed by fetch",
    ] {
        assert!(!sibling::holder_unanswered(error), "{error}");
    }
}

#[test]
fn a_holder_that_refuses_twice_then_serves_is_drawn_after_the_wait() {
    let tmp = tempfile::tempdir().unwrap();
    let store = two_bin_store(tmp.path());
    let landed = tmp.path().join("sibling");
    sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
    let release = store.release_cid_of(&landed).unwrap();
    let offer = stating(offer_for(&release), &glibc_at_least(&[2, 17]));
    let serve = serving(&landed);
    let unanswered = std::cell::Cell::new(0);
    let fetch = |output: &str, dest: &Path| {
        if unanswered.get() < 2 {
            unanswered.set(unanswered.get() + 1);
            return Err(printed(&eprfs_peer::FetchGap::NoRoute {
                device: offer.device.clone(),
            }));
        }
        serve(output, dest)
    };
    let clock = Clock::new();
    let said = std::cell::RefCell::new(Vec::<String>::new());
    let patience = sibling::Patience {
        bound: std::time::Duration::from_secs(90),
        now: &|| clock.now(),
        sleep: &|pause| {
            assert_store_unlocked(store.root());
            clock.sleep(pause);
        },
        say: &|line| said.borrow_mut().push(line.to_owned()),
        then: "building locally",
    };
    let drawn = match draw_with(
        &store,
        tmp.path(),
        &offer,
        &host_with_glibc(&[2, 40]),
        &fetch,
        &|_, _| Ok(()),
        &patience,
    ) {
        Ok(value) => value,
        Err(sibling::Draw::Gap(gap)) => panic!("gap: {gap}"),
        Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
        Err(sibling::Draw::Refused(r)) => panic!("refused {}: {}", r.kind, r.detail),
    };
    assert_eq!(drawn["release"], release);
    assert_eq!(clock.slept(), [2, 4], "two pauses, doubling");
    assert_eq!(
        said.borrow().as_slice(),
        [format!(
            "follow: holder did:key:z6MkSibling not answering for release {release}; \
             waited 6 s; drew it"
        )]
    );
    assert!(store.holds_verified_for(DRAWN));
}

#[test]
fn a_holder_silent_past_the_bound_falls_back_naming_the_holder_and_the_seconds() {
    let tmp = tempfile::tempdir().unwrap();
    let store = two_bin_store(tmp.path());
    let offer = stating(offer_for("R1"), &glibc_at_least(&[2, 17]));
    let asked = std::cell::Cell::new(0);
    let fetch = |_: &str, _: &Path| {
        asked.set(asked.get() + 1);
        Err(printed(&eprfs_peer::FetchGap::NoRoute {
            device: offer.device.clone(),
        }))
    };
    let clock = Clock::new();
    let said = std::cell::RefCell::new(Vec::<String>::new());
    let patience = sibling::Patience {
        bound: sibling::DEFAULT_PATIENCE,
        now: &|| clock.now(),
        sleep: &|pause| clock.sleep(pause),
        say: &|line| said.borrow_mut().push(line.to_owned()),
        then: "building locally",
    };
    match draw_with(
        &store,
        tmp.path(),
        &offer,
        &host_with_glibc(&[2, 40]),
        &fetch,
        &|_, _| Ok(()),
        &patience,
    ) {
        Err(sibling::Draw::Unanswered(gap)) => {
            assert!(gap.starts_with("holder-not-answering: "), "{gap}");
            assert!(
                gap.contains("did:key:z6MkSibling") && gap.contains("R1"),
                "{gap}"
            );
            assert!(gap.contains("waited 90 s"), "{gap}");
        }
        Err(sibling::Draw::Gap(gap)) => panic!("the wait ends unanswered, not a gap: {gap}"),
        Err(sibling::Draw::Refused(r)) => panic!("refused {}", r.detail),
        Ok(value) => panic!("nothing answered, yet {value}"),
    }
    assert_eq!(
        clock.slept(),
        [2, 4, 8, 16, 16, 16, 16, 12],
        "doubling to 16 s, the last cut to the bound"
    );
    assert_eq!(asked.get(), 9, "asked once more after every pause");
    assert_eq!(
        said.borrow().as_slice(),
        [
            "follow: holder did:key:z6MkSibling not answering for release R1; waited 90 s; \
          building locally"
        ]
    );
    assert!(!store.holds_verified_for(DRAWN));
    assert!(!tmp.path().join("incoming/sibling-R1").exists());
}

#[test]
fn a_relay_only_holder_that_cannot_deliver_twice_then_serves_is_drawn_after_the_wait() {
    let tmp = tempfile::tempdir().unwrap();
    let store = two_bin_store(tmp.path());
    let landed = tmp.path().join("sibling");
    sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
    let release = store.release_cid_of(&landed).unwrap();
    let offer = stating(offer_for(&release), &glibc_at_least(&[2, 17]));
    let serve = serving(&landed);
    let undelivered = std::cell::Cell::new(0);
    let fetch = |output: &str, dest: &Path| {
        if undelivered.get() < 2 {
            undelivered.set(undelivered.get() + 1);
            return Err(relay_undelivered(&offer.device));
        }
        serve(output, dest)
    };
    let clock = Clock::new();
    let said = std::cell::RefCell::new(Vec::<String>::new());
    let patience = sibling::Patience {
        bound: sibling::DEFAULT_PATIENCE,
        now: &|| clock.now(),
        sleep: &|pause| {
            assert_store_unlocked(store.root());
            clock.sleep(pause);
        },
        say: &|line| said.borrow_mut().push(line.to_owned()),
        then: "building locally",
    };
    let drawn = match draw_with(
        &store,
        tmp.path(),
        &offer,
        &host_with_glibc(&[2, 40]),
        &fetch,
        &|_, _| Ok(()),
        &patience,
    ) {
        Ok(value) => value,
        Err(sibling::Draw::Gap(gap)) => panic!("gap: {gap}"),
        Err(sibling::Draw::Unanswered(gap)) => panic!("unanswered: {gap}"),
        Err(sibling::Draw::Refused(r)) => panic!("refused {}: {}", r.kind, r.detail),
    };
    assert_eq!(drawn["release"], release);
    assert_eq!(clock.slept(), [2, 4], "two pauses, doubling");
    assert_eq!(
        said.borrow().as_slice(),
        [format!(
            "follow: holder did:key:z6MkSibling not answering for release {release}; \
             waited 6 s; drew it"
        )]
    );
    assert!(store.holds_verified_for(DRAWN));
}

#[test]
fn a_relay_only_holder_that_never_delivers_falls_back_naming_the_holder_and_the_seconds() {
    let tmp = tempfile::tempdir().unwrap();
    let store = two_bin_store(tmp.path());
    let offer = stating(offer_for("R1"), &glibc_at_least(&[2, 17]));
    let asked = std::cell::Cell::new(0);
    let fetch = |_: &str, _: &Path| {
        asked.set(asked.get() + 1);
        Err(relay_undelivered(&offer.device))
    };
    let clock = Clock::new();
    let said = std::cell::RefCell::new(Vec::<String>::new());
    let patience = sibling::Patience {
        bound: sibling::DEFAULT_PATIENCE,
        now: &|| clock.now(),
        sleep: &|pause| clock.sleep(pause),
        say: &|line| said.borrow_mut().push(line.to_owned()),
        then: "building locally",
    };
    match draw_with(
        &store,
        tmp.path(),
        &offer,
        &host_with_glibc(&[2, 40]),
        &fetch,
        &|_, _| Ok(()),
        &patience,
    ) {
        Err(sibling::Draw::Unanswered(gap)) => {
            assert!(gap.starts_with("holder-not-answering: "), "{gap}");
            assert!(gap.contains("did:key:z6MkSibling"), "{gap}");
            assert!(gap.contains("waited 90 s"), "{gap}");
            assert!(
                gap.contains("transport-cannot-deliver"),
                "the last gap: {gap}"
            );
        }
        Err(sibling::Draw::Gap(gap)) => panic!("the wait ends unanswered, not a gap: {gap}"),
        Err(sibling::Draw::Refused(r)) => panic!("refused {}", r.detail),
        Ok(value) => panic!("nothing delivered, yet {value}"),
    }
    assert_eq!(clock.slept(), [2, 4, 8, 16, 16, 16, 16, 12]);
    assert_eq!(asked.get(), 9);
    assert_eq!(
        said.borrow().as_slice(),
        [
            "follow: holder did:key:z6MkSibling not answering for release R1; waited 90 s; \
             building locally"
        ]
    );
    assert!(!store.holds_verified_for(DRAWN));
}

#[test]
fn every_other_gap_falls_back_at_once_with_no_wait() {
    let tmp = tempfile::tempdir().unwrap();
    let store = two_bin_store(tmp.path());
    let landed = tmp.path().join("sibling");
    sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
    let release = store.release_cid_of(&landed).unwrap();
    let host = host_with_glibc(&[2, 31]);
    let clock = Clock::new();
    let said = std::cell::RefCell::new(Vec::<String>::new());
    let patience = sibling::Patience {
        bound: sibling::DEFAULT_PATIENCE,
        now: &|| clock.now(),
        sleep: &|pause| clock.sleep(pause),
        say: &|line| said.borrow_mut().push(line.to_owned()),
        then: "building locally",
    };
    let gap_of = |result: Result<Value, sibling::Draw>| match result {
        Err(sibling::Draw::Gap(gap)) => gap,
        Err(sibling::Draw::Unanswered(gap)) => panic!("waited on: {gap}"),
        Err(sibling::Draw::Refused(r)) => panic!("refused {}", r.detail),
        Ok(value) => panic!("taken {value}"),
    };
    // An offer this host does not admit: nothing fetched, nothing waited.
    let asked = std::cell::Cell::new(0);
    let unmet = stating(offer_for(&release), &glibc_at_least(&[2, 39]));
    let gap = gap_of(draw_with(
        &store,
        tmp.path(),
        &unmet,
        &host,
        &|_, _| {
            asked.set(asked.get() + 1);
            Err("must not fetch".into())
        },
        &|_, _| Ok(()),
        &patience,
    ));
    assert!(gap.starts_with("requirement-unmet: "), "{gap}");
    assert_eq!(asked.get(), 0);
    // An admitted offer whose fetch fails any other way than a silent holder.
    let offer = stating(offer_for(&release), &glibc_at_least(&[2, 17]));
    for failed in every_fetch_gap()
        .iter()
        .filter(|gap| !waited_on(gap))
        .map(printed)
    {
        asked.set(0);
        let gap = gap_of(draw_with(
            &store,
            tmp.path(),
            &offer,
            &host,
            &|_, _| {
                asked.set(asked.get() + 1);
                Err(failed.clone())
            },
            &|_, _| Ok(()),
            &patience,
        ));
        assert_eq!(gap, failed);
        assert_eq!(asked.get(), 1, "{failed}: asked once");
    }
    // Staged bytes this host cannot run: a gap at once, though the holder answered.
    let serve = serving(&landed);
    let gap = gap_of(draw_with(
        &store,
        tmp.path(),
        &offer,
        &host,
        &serve,
        &|_, _| Err("isa.x86 missing-feature avx512f".into()),
        &patience,
    ));
    assert!(gap.starts_with("statement-understated: "), "{gap}");
    assert_eq!(clock.slept(), Vec::<u64>::new(), "no gap above waits");
    assert!(said.borrow().is_empty(), "{:?}", said.borrow());
    // A holder that was silent once and then answers with another gap: that gap, at once.
    let turns = std::cell::Cell::new(0);
    let gap = gap_of(draw_with(
        &store,
        tmp.path(),
        &offer,
        &host,
        &|_, _| {
            turns.set(turns.get() + 1);
            Err(printed(&if turns.get() == 1 {
                eprfs_peer::FetchGap::NoRoute {
                    device: offer.device.clone(),
                }
            } else {
                eprfs_peer::FetchGap::Refused {
                    device: offer.device.clone(),
                    refusal: "not granted".into(),
                }
            }))
        },
        &|_, _| Ok(()),
        &patience,
    ));
    assert!(gap.starts_with("fetch gap: refused: "), "{gap}");
    assert_eq!(clock.slept(), [2], "one pause, then the refusal ends it");
    assert_eq!(turns.get(), 2);
    assert!(!store.holds_verified_for(DRAWN));
}

#[test]
fn a_mismatch_after_a_silent_holder_answers_still_refuses_at_once() {
    let tmp = tempfile::tempdir().unwrap();
    let store = two_bin_store(tmp.path());
    let landed = tmp.path().join("sibling");
    sibling_bins(&landed, "#!/bin/sh\n# sibling launcher\n");
    let promised = store.release_cid_of(&landed).unwrap();
    let tampered = tmp.path().join("tampered");
    sibling_bins(&tampered, "#!/bin/sh\n# not the launcher it landed\n");
    let offer = stating(offer_for(&promised), &glibc_at_least(&[2, 17]));
    let serve = serving(&tampered);
    let silent = std::cell::Cell::new(true);
    let fetch = |output: &str, dest: &Path| {
        if silent.replace(false) {
            return Err(printed(&eprfs_peer::FetchGap::NoRoute {
                device: offer.device.clone(),
            }));
        }
        serve(output, dest)
    };
    let clock = Clock::new();
    let said = std::cell::RefCell::new(Vec::<String>::new());
    let patience = sibling::Patience {
        bound: sibling::DEFAULT_PATIENCE,
        now: &|| clock.now(),
        sleep: &|pause| clock.sleep(pause),
        say: &|line| said.borrow_mut().push(line.to_owned()),
        then: "building locally",
    };
    match draw_with(
        &store,
        tmp.path(),
        &offer,
        &host_with_glibc(&[2, 40]),
        &fetch,
        &|_, _| Ok(()),
        &patience,
    ) {
        Err(sibling::Draw::Refused(refusal)) => {
            assert_eq!(refusal.kind, "sibling-release-mismatch");
            assert!(refusal.detail.contains(&promised), "{}", refusal.detail);
        }
        Err(sibling::Draw::Gap(gap)) => panic!("a mismatch is a refusal, not a gap: {gap}"),
        Err(sibling::Draw::Unanswered(gap)) => panic!("a mismatch is not waited on: {gap}"),
        Ok(value) => panic!("tampered bytes were taken: {value}"),
    }
    assert_eq!(
        clock.slept(),
        [2],
        "waited only while the holder was silent"
    );
    assert!(said.borrow().is_empty(), "{:?}", said.borrow());
    assert!(!store.holds_verified_for(DRAWN), "nothing is ingested");
}

/// A checkout whose HEAD holds `policy`, when given, as genesis/agentic/pool-policy.json.
fn policy_checkout(policy: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let git = |args: &[&str]| {
        let out = crate::git::native_command(root)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
    };
    git(&["init", "-q"]);
    std::fs::write(root.join("README"), "fixture\n").unwrap();
    if let Some(policy) = policy {
        std::fs::create_dir_all(root.join("genesis/agentic")).unwrap();
        std::fs::write(root.join("genesis/agentic/pool-policy.json"), policy).unwrap();
    }
    git(&["add", "."]);
    git(&["commit", "-qm", "fixture", "--no-verify"]);
    dir
}

#[test]
fn the_patience_bound_is_the_pool_policys_at_head() {
    let short = policy_checkout(Some(
        &json!({"device_restart": {"draw_patience_seconds": 5}}).to_string(),
    ));
    assert_eq!(
        sibling::draw_patience(short.path()),
        std::time::Duration::from_secs(5)
    );
    // Absent key, absent file, a value that is not whole seconds, no repository: 90 s.
    for policy in [
        Some(json!({"device_restart": {}}).to_string()),
        Some(json!({"pool_max_gb": 150}).to_string()),
        Some(json!({"device_restart": {"draw_patience_seconds": "soon"}}).to_string()),
        None,
    ] {
        let checkout = policy_checkout(policy.as_deref());
        assert_eq!(
            sibling::draw_patience(checkout.path()),
            sibling::DEFAULT_PATIENCE,
            "{policy:?}"
        );
    }
    let bare = tempfile::tempdir().unwrap();
    assert_eq!(
        sibling::draw_patience(bare.path()),
        sibling::DEFAULT_PATIENCE
    );
    // The bound read there is the one the draw honours.
    let store = two_bin_store(short.path());
    let offer = stating(offer_for("R1"), &glibc_at_least(&[2, 17]));
    let clock = Clock::new();
    let said = std::cell::RefCell::new(Vec::<String>::new());
    let patience = sibling::Patience {
        bound: sibling::draw_patience(short.path()),
        now: &|| clock.now(),
        sleep: &|pause| clock.sleep(pause),
        say: &|line| said.borrow_mut().push(line.to_owned()),
        then: "trying a local build",
    };
    let result = draw_with(
        &store,
        short.path(),
        &offer,
        &host_with_glibc(&[2, 40]),
        &|_, _| {
            Err(printed(&eprfs_peer::FetchGap::NoRoute {
                device: offer.device.clone(),
            }))
        },
        &|_, _| Ok(()),
        &patience,
    );
    assert!(
        matches!(result, Err(sibling::Draw::Unanswered(_))),
        "a short bound still ends unanswered"
    );
    assert_eq!(clock.slept(), [2, 3]);
    assert_eq!(
        said.borrow().as_slice(),
        [
            "follow: holder did:key:z6MkSibling not answering for release R1; waited 5 s; \
          trying a local build"
        ]
    );
}
