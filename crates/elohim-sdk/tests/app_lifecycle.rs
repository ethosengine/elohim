use async_trait::async_trait;
use elohim_sdk::{contracts::Answer, lifecycle::*, SdkError};

fn release() -> AppReleaseView {
    AppReleaseView {
        app_id: "orchard".into(),
        release_cid: "fixture-release".into(),
        target: "browser".into(),
        artifact_cids: vec!["fixture-artifact".into()],
        required_capabilities: vec![],
    }
}
fn demand(urgency: AppSyncUrgency, end: Option<&str>) -> AppSyncDemandView {
    AppSyncDemandView {
        scope: "own:observations".into(),
        urgency,
        targets: vec![AppSyncTargetView {
            device_id: "own-laptop".into(),
            stream_id: "observations".into(),
            epoch: "epoch-1".into(),
            end_position: end.map(str::to_owned),
        }],
    }
}

#[tokio::test]
async fn unsupported_never_claims_absence_execution_or_durability() {
    let adapter = UnsupportedLifecycle;
    assert!(matches!(
        adapter.resolve("orchard", "browser").await,
        Err(SdkError::Unsupported(_))
    ));
    assert!(matches!(
        adapter
            .ensure_instance(&AppOpenInput {
                release: release(),
                operation_id: "open-1".into(),
                grant_ref: "unverified-reference".into()
            })
            .await,
        Err(SdkError::Unsupported(_))
    ));
    assert!(matches!(
        adapter.acquire(&release()).await,
        Err(SdkError::Unsupported(_))
    ));
    assert!(matches!(
        adapter.prepare("instance").await,
        Err(SdkError::Unsupported(_))
    ));
    assert!(matches!(
        adapter
            .request(&demand(AppSyncUrgency::Foreground, None))
            .await,
        Err(SdkError::Unsupported(_))
    ));
}

struct ReadOnlyRuntime;
#[async_trait]
impl AppRuntime for ReadOnlyRuntime {
    async fn resolve(&self, _: &str, _: &str) -> elohim_sdk::Result<Answer<AppReleaseView>> {
        Ok(Answer::Present(release()))
    }
}
#[tokio::test]
async fn replacement_adapter_uses_same_interface_without_inheriting_mutations() {
    let adapter: Box<dyn AppRuntime> = Box::new(ReadOnlyRuntime);
    assert!(matches!(
        adapter.resolve("orchard", "browser").await.unwrap(),
        Answer::Present(_)
    ));
    assert!(matches!(
        adapter.observe("unknown").await,
        Err(SdkError::Unsupported(_))
    ));
}

#[test]
fn offline_requires_verified_code_and_data() {
    for code in [None, Some(false), Some(true)] {
        for data in [None, Some(false), Some(true)] {
            let residency = AppResidencyView {
                release_cid: "fixture".into(),
                pinned: true,
                running: true,
                artifacts_verified: code,
                data_verified: data,
            };
            assert_eq!(
                available_offline(&residency),
                code == Some(true) && data == Some(true)
            );
        }
    }
}

#[test]
fn refresh_promotes_one_scope_and_cancellation_preserves_background() {
    let mut queue = SyncDemandQueue::new(4, 2);
    let background = queue
        .request(demand(AppSyncUrgency::Background, Some("1")))
        .unwrap();
    let first = queue
        .request(demand(AppSyncUrgency::Foreground, Some("10")))
        .unwrap();
    let second = queue
        .request(demand(AppSyncUrgency::Foreground, Some("20")))
        .unwrap();
    assert_eq!(queue.scopes().len(), 1);
    assert_eq!(
        queue.scopes()["own:observations"],
        AppSyncUrgency::Foreground
    );
    queue.release(first);
    queue.release(first);
    assert_eq!(
        queue.scopes()["own:observations"],
        AppSyncUrgency::Foreground
    );
    queue.release(second);
    assert_eq!(
        queue.scopes()["own:observations"],
        AppSyncUrgency::Background
    );
    assert!(queue.demand(background).is_some());
}

#[test]
fn fixed_targets_preserve_unknown_epoch_and_large_positions() {
    let mut queue = SyncDemandQueue::new(4, 2);
    let first = queue
        .request(demand(AppSyncUrgency::Foreground, Some("9007199254740993")))
        .unwrap();
    let second = queue
        .request(demand(AppSyncUrgency::Foreground, Some("9007199254740994")))
        .unwrap();
    assert!(!queue.caught_up(first, |_, _| None));
    assert!(!queue.caught_up(first, |_, _| Some(("epoch-2".into(), u64::MAX))));
    assert!(!queue.caught_up(first, |_, _| Some(("epoch-1".into(), 9007199254740992))));
    assert!(queue.caught_up(first, |_, _| Some(("epoch-1".into(), 9007199254740993))));
    assert!(!queue.caught_up(second, |_, _| Some(("epoch-1".into(), 9007199254740993))));
    let unknown = queue
        .request(demand(AppSyncUrgency::Foreground, None))
        .unwrap();
    assert!(!queue.caught_up(unknown, |_, _| Some(("epoch-1".into(), u64::MAX))));
    assert!(!queue.caught_up(999, |_, _| unreachable!()));
}

#[test]
fn progress_distinguishes_unknown_behind_unreachable_and_caught_up() {
    let mut queue = SyncDemandQueue::new(4, 2);
    let id = queue
        .request(demand(AppSyncUrgency::Foreground, Some("10")))
        .unwrap();
    let unavailable = queue
        .progress(id, |_, _| Answer::Unreachable, |_| Some(false))
        .unwrap();
    assert_eq!(unavailable[0].reachable, Some(false));
    assert_eq!(unavailable[0].caught_up, None);
    let unknown = queue.progress(id, |_, _| Answer::Absent, |_| None).unwrap();
    assert_eq!(unknown[0].reachable, None);
    assert_eq!(unknown[0].caught_up, None);
    let behind = queue
        .progress(
            id,
            |_, _| Answer::Present(("epoch-1".into(), 9)),
            |_| Some(true),
        )
        .unwrap();
    assert_eq!(behind[0].caught_up, Some(false));
    assert_eq!(behind[0].local_position.as_deref(), Some("9"));
    let complete = queue
        .progress(
            id,
            |_, _| Answer::Present(("epoch-1".into(), 10)),
            |_| Some(false),
        )
        .unwrap();
    assert_eq!(complete[0].caught_up, Some(true));
    // A retained cursor reached the captured target, although this device is now offline.
    // Neither fact establishes the remote device's current end.
    assert_eq!(complete[0].reachable, Some(false));
    assert_eq!(complete[0].local_position.as_deref(), Some("10"));
    assert!(queue
        .progress(999, |_, _| unreachable!(), |_| unreachable!())
        .is_none());
}

#[test]
fn bounded_requests_reject_invalid_or_duplicate_targets_without_losing_prior_demands() {
    let mut queue = SyncDemandQueue::new(1, 1);
    for end in ["-1", "+1", "", "18446744073709551616", "1.0"] {
        assert!(queue
            .request(demand(AppSyncUrgency::Foreground, Some(end)))
            .is_err());
    }
    let id = queue
        .request(demand(AppSyncUrgency::Background, Some("0")))
        .unwrap();
    assert!(queue
        .request(demand(AppSyncUrgency::Foreground, Some("1")))
        .is_err());
    assert_eq!(
        queue.scopes()["own:observations"],
        AppSyncUrgency::Background
    );
    queue.release(id);
    let mut duplicate = demand(AppSyncUrgency::Foreground, Some("1"));
    duplicate.targets.push(duplicate.targets[0].clone());
    assert!(SyncDemandQueue::new(4, 4).request(duplicate).is_err());
    let mut empty = demand(AppSyncUrgency::Foreground, None);
    empty.targets.clear();
    assert!(queue.request(empty).is_err());
}
