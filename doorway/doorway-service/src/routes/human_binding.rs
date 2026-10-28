//! Binding a registration to a Human on a conductor agent.
//!
//! # The question this module answers by ASKING
//!
//! "Does this agent already have a Human?" A registration that is handed a
//! Human which already existed is not a creation, it is a RECOVERY — the new
//! account and its session are bound to an identity someone else may hold —
//! and a recovery needs evidence.
//!
//! The registration handler used to learn the answer from an error:
//! `create_human` was called first, and an `"Agent already has a Human
//! profile"` failure meant "it already existed". The imagodei zome does not
//! behave that way. `create_human` is IDEMPOTENT
//! (`elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs`, "Idempotency
//! gate"): when the calling agent already has a Human it returns that Human's
//! `ActionHash` and succeeds. The error string exists only in a doc comment.
//! So the error branch — and the gate that was put on it — never ran, and a
//! registration over an existing Human was answered as a plain success: an
//! account row and a session token carrying the existing Human's id.
//!
//! The lesson is in the test fake below as much as in the code: a fake must be
//! held to the real component's behaviour, or the tests prove a path the system
//! cannot take.
//!
//! # What is done instead
//!
//! [`bind_human`] reads the agent's Human BEFORE `create_human`, under a
//! per-agent lock:
//!
//! - one exists → this is a recovery. With evidence: recovered. Without:
//!   [`BindError::RecoveryRefused`], and `create_human` is never called;
//! - none exists → `create_human`, then read again and check that the Human
//!   that came back is the one THIS call created. If it is not, it is the
//!   recovery case after all, and the same gate applies before any account row
//!   or token is produced.
//!
//! Comparing the returned Human's id with the id the request asked for is NOT
//! evidence: a request can ask for any id, including an existing person's.
//!
//! # "I created it" versus "it already existed"
//!
//! Two things, and the request controls neither:
//!
//! 1. **The read happened first, under the agent's lock.** The lock key is the
//!    conductor's admin address and the installed app id — where the agent
//!    lives — not anything in the request. Within this process, no other
//!    registration can create a Human on that agent between this call's read
//!    ("none") and its create. A registration that cannot take the lock within
//!    the bound is refused ([`BindError::Busy`]); it does not proceed
//!    unserialised.
//! 2. **The Human that came back must be the one that was sent.** After the
//!    create, the agent's Human is read again; it is "created by this call"
//!    only if its action hash is the hash `create_human` returned AND every
//!    field equals what this call sent. That catches a writer outside this
//!    process (another doorway on the same agent) that got in between with
//!    different content.
//!
//! WHAT THIS CANNOT TELL APART: a writer in ANOTHER process that created a
//! Human with byte-identical fields in the same instant. The zome returns the
//! same hash to both and says nothing about which call authored it. Closing
//! that needs the zome to answer "created" or "existed" (a coordinator-only
//! change; it hot-swaps). No deployed shape has two doorway processes sharing
//! one conductor agent today.

use std::future::Future;
use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use holo_hash::ActionHash;
use tracing::{info, warn};

use crate::keyed_lock::KeyedLock;
use crate::routes::zome_helpers::{CreateHumanInput, Human, HumanOutput};
use crate::services::ZomeCaller;
use crate::types::DoorwayError;

/// The two imagodei calls a registration makes on ONE agent.
///
/// A trait so the handler's decision can be driven by a fake in tests. The fake
/// (`tests::ZomeFake`) is pinned to the zome's behaviour by
/// `tests::the_fake_is_held_to_the_zome_create_human_is_idempotent`.
#[async_trait]
pub trait HumanAuthor: Send + Sync {
    /// `imagodei::get_my_human` — the calling agent's Human, if it has one.
    async fn get_my_human(&self) -> Result<Option<HumanOutput>, DoorwayError>;

    /// `imagodei::create_human` — IDEMPOTENT. Returns the `ActionHash` of the
    /// calling agent's Human: a new one built from `input`, or the existing one
    /// unchanged. It does NOT fail when a Human already exists.
    async fn create_human(&self, input: &CreateHumanInput) -> Result<ActionHash, DoorwayError>;
}

/// [`HumanAuthor`] over a signed zome caller — the doorway's singleton for its
/// own agent, or a temporary caller scoped to one hosted person's app.
pub struct ZomeCallerAuthor<'a>(pub &'a ZomeCaller);

#[async_trait]
impl HumanAuthor for ZomeCallerAuthor<'_> {
    async fn get_my_human(&self) -> Result<Option<HumanOutput>, DoorwayError> {
        self.0
            .call("imagodei", "imagodei", "get_my_human", &())
            .await
            .map_err(|e| DoorwayError::Holochain(format!("get_my_human failed: {e}")))
    }

    async fn create_human(&self, input: &CreateHumanInput) -> Result<ActionHash, DoorwayError> {
        self.0
            .call("imagodei", "imagodei", "create_human", input)
            .await
            .map_err(|e| DoorwayError::Holochain(format!("create_human failed: {e}")))
    }
}

/// How a registration came to hold its Human.
#[derive(Debug, Clone)]
pub enum Bound {
    /// The agent had no Human; this call created it.
    Created(HumanOutput),
    /// The agent already had a Human, and the registration presented evidence
    /// that it may be handed it.
    Recovered(HumanOutput),
}

impl Bound {
    pub fn into_output(self) -> HumanOutput {
        match self {
            Self::Created(output) | Self::Recovered(output) => output,
        }
    }

    pub fn was_recovered(&self) -> bool {
        matches!(self, Self::Recovered(_))
    }
}

/// Why a registration was not bound to a Human.
#[derive(Debug)]
pub enum BindError {
    /// The agent already has a Human and the registration presented no
    /// evidence that it is theirs. Nothing was created, and no account row or
    /// token may be produced.
    RecoveryRefused,
    /// Another registration is binding a Human on this agent and did not
    /// finish within the bound. Refused rather than run beside it.
    Busy,
    /// The conductor could not be asked, or failed.
    Conductor(DoorwayError),
}

impl From<DoorwayError> for BindError {
    fn from(error: DoorwayError) -> Self {
        Self::Conductor(error)
    }
}

/// One lock per conductor agent, process-wide.
pub fn agent_locks() -> &'static KeyedLock {
    static LOCKS: OnceLock<KeyedLock> = OnceLock::new();
    LOCKS.get_or_init(KeyedLock::new)
}

/// The evidence a recovery needs, under the store gate
/// (`DOORWAY_PROVISIONING_STORE_GATE`).
///
/// Gate ON: an existing Human is handed to a registration only on `evidence`
/// (this doorway's provisioning record for a hosted agent, the operator
/// credential for the doorway's own agent). Gate OFF, the default: it is handed
/// over as dev handed it over before this gate existed — a registration onto an
/// agent that already has a Human re-attaches it (the "conductor kept, database
/// cleared" recovery). `evidence` is not consulted then. The read that asks
/// whether a Human exists runs either way; only the refusal is gated.
pub async fn recovery_evidence<Evidence, EvidenceFut>(gated: bool, evidence: Evidence) -> bool
where
    Evidence: FnOnce() -> EvidenceFut,
    EvidenceFut: Future<Output = bool>,
{
    !gated || evidence().await
}

/// Is `human` exactly what `sent` asked the zome to create?
fn is_what_was_sent(human: &Human, sent: &CreateHumanInput) -> bool {
    human.id == sent.id
        && human.display_name == sent.display_name
        && human.bio == sent.bio
        && human.affinities == sent.affinities
        && human.profile_reach == sent.profile_reach
        && human.location == sent.location
}

/// Bind a registration to the Human on ONE agent — creating it if the agent has
/// none, recovering it only with evidence.
///
/// - `agent_key` names the agent (conductor admin address + installed app id).
///   It is the lock key and must not be derived from the request.
/// - `lock_wait` must cover the worst case of the three conductor calls made
///   under the lock, so a waiter is still waiting when the holder finishes.
/// - `recovery_authorized` is evaluated ONLY if a Human already exists; it is
///   the evidence that this registration may be handed it.
///
/// See the module header for how "created by this call" is established.
pub async fn bind_human<Evidence, EvidenceFut>(
    author: &dyn HumanAuthor,
    locks: &KeyedLock,
    agent_key: &str,
    lock_wait: Duration,
    input: CreateHumanInput,
    recovery_authorized: Evidence,
) -> Result<Bound, BindError>
where
    Evidence: FnOnce() -> EvidenceFut,
    EvidenceFut: Future<Output = bool>,
{
    // One registration at a time per AGENT. A caller that cannot take the lock
    // is refused: proceeding unserialised is exactly the read-then-create race.
    let Some(_serial) = locks.acquire_within(agent_key, lock_wait).await else {
        return Err(BindError::Busy);
    };

    // ASK FIRST. `create_human` would answer an existing Human with success.
    if let Some(existing) = author.get_my_human().await? {
        return recover(existing, recovery_authorized).await;
    }

    // The agent has no Human, and under the lock nothing in this process can
    // give it one before this create.
    let created = author.create_human(&input).await?;
    let after = author.get_my_human().await?.ok_or_else(|| {
        DoorwayError::Internal(
            "create_human ok but get_my_human returned None — DHT projection lag?".into(),
        )
    })?;

    // Is the Human on the agent the one THIS call created? Its hash must be the
    // hash the create returned, and every field must be what was sent. If not,
    // a writer outside this process got there first: it already existed, and
    // handing it over is a recovery.
    if after.action_hash == created && is_what_was_sent(&after.human, &input) {
        Ok(Bound::Created(after))
    } else {
        warn!(
            agent = agent_key,
            "create_human returned a Human this call did not create — treating it as a recovery"
        );
        recover(after, recovery_authorized).await
    }
}

/// The recovery gate: an existing Human is handed over only with evidence.
async fn recover<Evidence, EvidenceFut>(
    existing: HumanOutput,
    recovery_authorized: Evidence,
) -> Result<Bound, BindError>
where
    Evidence: FnOnce() -> EvidenceFut,
    EvidenceFut: Future<Output = bool>,
{
    if recovery_authorized().await {
        Ok(Bound::Recovered(existing))
    } else {
        Err(BindError::RecoveryRefused)
    }
}

/// How long a registration waits for its agent's lock: the worst case of what
/// the holder does under it — a first connect, a first credential mint, and
/// the three conductor calls (read, create, read) — each at the per-call
/// deadline already in force. Nothing is extended; the bounds are added up.
pub fn agent_lock_wait() -> Duration {
    crate::services::zome_caller::worst_case_first_calls(3)
}

/// The lock key for the agent behind one conductor app.
pub fn agent_key(admin_url: &str, installed_app_id: &str) -> String {
    crate::services::signing_credentials::endpoint_key(admin_url, installed_app_id)
}

/// Log which way a registration was bound. One line, so a live run can be read.
pub fn log_bound(arm: &str, identifier: &str, bound: &Bound) {
    match bound {
        Bound::Created(output) => info!(
            arm,
            identifier,
            human_id = %output.human.id,
            "Registration CREATED the Human on its agent"
        ),
        Bound::Recovered(output) => warn!(
            arm,
            identifier,
            human_id = %output.human.id,
            "Registration RECOVERED the Human that already existed on its agent (evidence presented)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::sync::Notify;

    const WAIT: Duration = Duration::from_secs(30);

    // ── the fake, held to the zome ──────────────────────────────────────────

    /// One conductor agent's imagodei cell, as far as registration can see it.
    ///
    /// HELD TO THE REAL ZOME: `create_human` is idempotent, exactly as
    /// `imagodei::create_human` is — if the agent has a Human it returns that
    /// Human's hash and succeeds, changing nothing. It never raises "Agent
    /// already has a Human profile". `the_fake_is_held_to_the_zome_…` below
    /// reads the zome source and fails if that stops being true. (No sweettest
    /// asserts the idempotency directly; `submit_specialist_revocation.rs`
    /// depends on it by calling `create_human` twice under one agent.)
    #[derive(Default)]
    struct ZomeFake {
        human: Mutex<Option<HumanOutput>>,
        /// Every `create_human` call that reached the conductor.
        creates_received: AtomicU32,
        /// `(entered, release)`: a create signals it arrived, then waits.
        create_gate: Mutex<Option<(Arc<Notify>, Arc<Notify>)>>,
        /// A writer OUTSIDE this process that creates this Human just before
        /// our create lands.
        interloper: Mutex<Option<CreateHumanInput>>,
    }

    fn hash_of(seed: u8) -> ActionHash {
        ActionHash::from_raw_36(vec![seed; 36])
    }

    fn human_from(input: &CreateHumanInput) -> Human {
        Human {
            id: input.id.clone(),
            display_name: input.display_name.clone(),
            bio: input.bio.clone(),
            affinities: input.affinities.clone(),
            profile_reach: input.profile_reach.clone(),
            location: input.location.clone(),
            created_at: "Timestamp(2026-10-04T00:00:00Z)".to_string(),
            updated_at: "Timestamp(2026-10-04T00:00:00Z)".to_string(),
        }
    }

    impl ZomeFake {
        fn with_existing(input: &CreateHumanInput) -> Self {
            let fake = Self::default();
            *fake.human.lock().unwrap() = Some(HumanOutput {
                action_hash: hash_of(1),
                human: human_from(input),
            });
            fake
        }

        fn creates(&self) -> u32 {
            self.creates_received.load(Ordering::SeqCst)
        }

        /// The zome's idempotency gate, verbatim in shape.
        fn create_or_return_existing(&self, input: &CreateHumanInput, seed: u8) -> ActionHash {
            let mut human = self.human.lock().unwrap();
            if let Some(existing) = human.as_ref() {
                return existing.action_hash.clone();
            }
            let created = HumanOutput {
                action_hash: hash_of(seed),
                human: human_from(input),
            };
            let hash = created.action_hash.clone();
            *human = Some(created);
            hash
        }
    }

    #[async_trait]
    impl HumanAuthor for ZomeFake {
        async fn get_my_human(&self) -> Result<Option<HumanOutput>, DoorwayError> {
            Ok(self.human.lock().unwrap().clone())
        }

        async fn create_human(&self, input: &CreateHumanInput) -> Result<ActionHash, DoorwayError> {
            self.creates_received.fetch_add(1, Ordering::SeqCst);
            let gate = self.create_gate.lock().unwrap().take();
            if let Some((entered, release)) = gate {
                entered.notify_one();
                release.notified().await;
            }
            if let Some(theirs) = self.interloper.lock().unwrap().take() {
                self.create_or_return_existing(&theirs, 9);
            }
            Ok(self.create_or_return_existing(input, 2))
        }
    }

    /// THE PIN. The fake's idempotent `create_human` is only worth anything
    /// while the zome's is: read the coordinator source and check its
    /// idempotency gate is there and that it raises no "already has a Human"
    /// error. If the zome changes, this fails and the fake must follow.
    #[test]
    fn the_fake_is_held_to_the_zome_create_human_is_idempotent() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs");
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read the imagodei coordinator at {path:?}: {e}"));

        let start = source
            .find("pub fn create_human(input: CreateHumanInput) -> ExternResult<ActionHash>")
            .expect("imagodei::create_human still returns a bare ActionHash");
        let body = &source[start..];
        let body = &body[..body.find("\n}\n").expect("end of create_human")];

        let gate = body
            .find("if let Some(existing) = get_human_by_agent_key(")
            .expect("create_human reads the agent's existing Human first");
        let returns_existing = body
            .find("return Ok(existing.action_hash);")
            .expect("create_human returns the existing Human's hash — it is idempotent");
        let creates = body
            .find("create_entry(&EntryTypes::Human(")
            .expect("create_human creates the entry");
        assert!(
            gate < returns_existing && returns_existing < creates,
            "the idempotent return must come before the create"
        );
        assert!(
            !body.contains("already has a Human"),
            "create_human now raises an 'already has a Human' error — the fake, and the \
             read-before-create in bind_human, must be revisited"
        );
    }

    // ── fixtures ────────────────────────────────────────────────────────────

    fn operator() -> CreateHumanInput {
        CreateHumanInput {
            id: "human-matthew-manager".to_string(),
            display_name: "Matthew".to_string(),
            bio: Some("operator".to_string()),
            affinities: vec!["stewardship".to_string()],
            profile_reach: "commons".to_string(),
            location: None,
        }
    }

    fn newcomer(id: &str) -> CreateHumanInput {
        CreateHumanInput {
            id: id.to_string(),
            display_name: "Someone Else".to_string(),
            bio: None,
            affinities: Vec::new(),
            profile_reach: "private".to_string(),
            location: None,
        }
    }

    async fn bind(
        fake: &ZomeFake,
        input: CreateHumanInput,
        evidence: bool,
    ) -> Result<Bound, BindError> {
        let locks = KeyedLock::new();
        bind_human(
            fake,
            &locks,
            "agent",
            WAIT,
            input,
            || async move { evidence },
        )
        .await
    }

    // ── an existing Human is a recovery, and a recovery needs evidence ──────

    /// THE LIVE FINDING. The operator's Human is already on the doorway's
    /// agent. A registration with no evidence must be refused — and
    /// `create_human` must not be called at all.
    #[tokio::test]
    async fn a_registration_over_an_existing_human_without_evidence_is_refused() {
        let fake = ZomeFake::with_existing(&operator());

        let outcome = bind(&fake, newcomer("human-someone-new"), false).await;

        assert!(
            matches!(outcome, Err(BindError::RecoveryRefused)),
            "an existing Human was attached to a registration with no evidence: {outcome:?}"
        );
        assert_eq!(fake.creates(), 0, "create_human must not be called");
    }

    /// Naming the existing Human's own id is not evidence. A request can ask
    /// for any id.
    #[tokio::test]
    async fn supplying_the_existing_humans_id_is_not_evidence() {
        let fake = ZomeFake::with_existing(&operator());

        // Every field the request controls, set to the operator's.
        let outcome = bind(&fake, operator(), false).await;

        assert!(
            matches!(outcome, Err(BindError::RecoveryRefused)),
            "{outcome:?}"
        );
        assert_eq!(fake.creates(), 0);
    }

    /// The store gate decides whether the refusal applies. OFF (the default):
    /// an existing Human is re-attached without evidence, as dev does, and the
    /// evidence is not even consulted. ON: no evidence, no Human. Either way the
    /// existing Human is READ first and nothing is created.
    #[tokio::test]
    async fn the_recovery_refusal_applies_only_with_the_store_gate_on() {
        for gated in [false, true] {
            let fake = ZomeFake::with_existing(&operator());
            let locks = KeyedLock::new();
            let consulted = AtomicU32::new(0);

            let outcome = bind_human(
                &fake,
                &locks,
                "agent",
                WAIT,
                newcomer("human-someone-new"),
                || {
                    recovery_evidence(gated, || async {
                        consulted.fetch_add(1, Ordering::SeqCst);
                        false
                    })
                },
            )
            .await;

            if gated {
                assert!(
                    matches!(outcome, Err(BindError::RecoveryRefused)),
                    "{outcome:?}"
                );
                assert_eq!(consulted.load(Ordering::SeqCst), 1);
            } else {
                let bound = outcome.expect("gate off re-attaches, as dev does");
                assert!(bound.was_recovered());
                assert_eq!(bound.into_output().human.id, "human-matthew-manager");
                assert_eq!(consulted.load(Ordering::SeqCst), 0, "evidence not asked");
            }
            assert_eq!(fake.creates(), 0, "{gated}: a recovery creates nothing");
        }
    }

    /// With evidence, the existing Human is recovered — and it is reported as a
    /// recovery, not as a creation.
    #[tokio::test]
    async fn a_registration_over_an_existing_human_with_evidence_recovers_it() {
        let fake = ZomeFake::with_existing(&operator());

        let bound = bind(&fake, newcomer("human-someone-new"), true)
            .await
            .expect("evidence was presented");

        assert!(bound.was_recovered());
        assert_eq!(bound.into_output().human.id, "human-matthew-manager");
        assert_eq!(fake.creates(), 0, "a recovery creates nothing");
    }

    /// The evidence is consulted only when there is something to recover.
    #[tokio::test]
    async fn the_first_registration_on_a_fresh_agent_creates_its_human() {
        let fake = ZomeFake::default();
        let locks = KeyedLock::new();
        let consulted = AtomicU32::new(0);

        let bound = bind_human(&fake, &locks, "agent", WAIT, operator(), || async {
            consulted.fetch_add(1, Ordering::SeqCst);
            false
        })
        .await
        .expect("a fresh agent has nothing to recover");

        assert!(!bound.was_recovered());
        assert_eq!(bound.into_output().human.id, "human-matthew-manager");
        assert_eq!(fake.creates(), 1);
        assert_eq!(
            consulted.load(Ordering::SeqCst),
            0,
            "no evidence is asked for when the agent had no Human"
        );
        assert_eq!(locks.live_keys(), 0);
    }

    // ── the read-then-create race ───────────────────────────────────────────

    /// Two registrations reach a FRESH agent together. Exactly one creates; the
    /// other waits on the agent's lock, then finds a Human there and is held to
    /// the recovery gate — it is not attached to the first one's Human.
    #[tokio::test]
    async fn two_registrations_arriving_together_do_not_both_get_the_human() {
        let fake = Arc::new(ZomeFake::default());
        let locks = Arc::new(KeyedLock::new());
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        *fake.create_gate.lock().unwrap() = Some((Arc::clone(&entered), Arc::clone(&release)));

        let first = {
            let (fake, locks) = (Arc::clone(&fake), Arc::clone(&locks));
            tokio::spawn(async move {
                bind_human(&*fake, &locks, "agent", WAIT, operator(), || async {
                    false
                })
                .await
            })
        };
        entered.notified().await;

        let second = {
            let (fake, locks) = (Arc::clone(&fake), Arc::clone(&locks));
            tokio::spawn(async move {
                // The same fields as the first, and no evidence.
                bind_human(&*fake, &locks, "agent", WAIT, operator(), || async {
                    false
                })
                .await
            })
        };
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        assert_eq!(fake.creates(), 1, "the second ran beside the first");

        release.notify_one();
        let first = first.await.unwrap().expect("the first creates");
        let second = second.await.unwrap();

        assert!(!first.was_recovered());
        assert!(
            matches!(second, Err(BindError::RecoveryRefused)),
            "the second was attached to the first's Human without evidence: {second:?}"
        );
        assert_eq!(fake.creates(), 1, "one create reached the conductor");
        assert_eq!(locks.live_keys(), 0);
    }

    /// A registration that cannot take the agent's lock within the bound is
    /// refused. It does not run unserialised beside the holder.
    #[tokio::test]
    async fn a_registration_that_cannot_take_the_agents_lock_is_refused() {
        let fake = Arc::new(ZomeFake::default());
        let locks = Arc::new(KeyedLock::new());
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        *fake.create_gate.lock().unwrap() = Some((Arc::clone(&entered), Arc::clone(&release)));

        let holder = {
            let (fake, locks) = (Arc::clone(&fake), Arc::clone(&locks));
            tokio::spawn(async move {
                bind_human(&*fake, &locks, "agent", WAIT, operator(), || async {
                    false
                })
                .await
            })
        };
        entered.notified().await;

        let waiter = bind_human(
            &*fake,
            &locks,
            "agent",
            Duration::from_millis(50),
            newcomer("human-someone-new"),
            || async { true },
        )
        .await;
        assert!(matches!(waiter, Err(BindError::Busy)), "{waiter:?}");
        assert_eq!(fake.creates(), 1);

        release.notify_one();
        holder.await.unwrap().expect("the holder creates");
    }

    /// A writer OUTSIDE this process creates a different Human between this
    /// call's read and its create. The zome hands back THEIR Human; it is not
    /// what this call sent, so it is a recovery and the gate applies.
    #[tokio::test]
    async fn a_human_that_is_not_the_one_this_call_sent_is_a_recovery() {
        let fake = ZomeFake::default();
        *fake.interloper.lock().unwrap() = Some(operator());

        let refused = bind(&fake, newcomer("human-someone-new"), false).await;
        assert!(
            matches!(refused, Err(BindError::RecoveryRefused)),
            "{refused:?}"
        );

        let fake = ZomeFake::default();
        *fake.interloper.lock().unwrap() = Some(operator());
        let recovered = bind(&fake, newcomer("human-someone-new"), true)
            .await
            .expect("with evidence");
        assert!(recovered.was_recovered());
        assert_eq!(recovered.into_output().human.id, "human-matthew-manager");
    }

    /// A conductor that cannot be asked is a failure, never a creation and
    /// never a recovery.
    #[tokio::test]
    async fn a_conductor_that_cannot_be_asked_binds_nothing() {
        struct Down;
        #[async_trait]
        impl HumanAuthor for Down {
            async fn get_my_human(&self) -> Result<Option<HumanOutput>, DoorwayError> {
                Err(DoorwayError::Holochain(
                    "get_my_human failed: timed out".into(),
                ))
            }
            async fn create_human(&self, _: &CreateHumanInput) -> Result<ActionHash, DoorwayError> {
                panic!("create_human must not be called when the read failed")
            }
        }
        let locks = KeyedLock::new();
        let outcome = bind_human(&Down, &locks, "agent", WAIT, operator(), || async { true }).await;
        assert!(
            matches!(outcome, Err(BindError::Conductor(_))),
            "{outcome:?}"
        );
    }

    #[test]
    fn only_an_identical_human_is_what_was_sent() {
        let sent = operator();
        assert!(is_what_was_sent(&human_from(&sent), &sent));
        let mut other = human_from(&sent);
        other.display_name = "Someone Else".to_string();
        assert!(!is_what_was_sent(&other, &sent));
        let mut other = human_from(&sent);
        other.affinities.push("extra".to_string());
        assert!(!is_what_was_sent(&other, &sent));
    }
}
