//! The provisioning intent log — what THIS doorway asked a conductor to do for
//! one person, written down before it asks.
//!
//! # What this is, and what it is not
//!
//! It is a **saga log**, not an authority.
//!
//! - Whether cells exist: **the conductor says.** Every resolution here ends in
//!   a lookup on the conductor the record names, and the conductor's answer
//!   wins over the record.
//! - Whether a human exists: **the DHT says.** `human_created` in this log is
//!   the doorway remembering that its `create_human` call returned; it is not
//!   evidence that a Human entry exists and nothing may be decided on it alone.
//! - The log only remembers **what this doorway asked for**: for which person,
//!   on which conductor, under which app id, with which agent key, and how far
//!   it knows the asking got.
//!
//! It must stay rebuildable from the first two. A record that is lost is a
//! recovery problem (the operator's `POST /admin/provisioning/recover`), never
//! a correctness problem with the identities themselves.
//!
//! # What the record DOES prove
//!
//! One thing, and it is load-bearing: **which agent key this doorway generated
//! for this identifier.** An app found on a conductor is treated as this
//! person's only when the conductor's agent key for it equals the key recorded
//! here. An app id alone never proves that — see
//! `AgentProvisioner::finish_forward`.
//!
//! # Why it exists
//!
//! Each provisioning attempt generates a fresh agent key and the app id embeds
//! the conductor id. Without a record, the only way to learn whether a person
//! was already being provisioned was to ask every conductor in the pool — which
//! (a) made every new registration depend on every conductor being reachable,
//! and (b) still could not see an install that was in flight, so a retry could
//! land a second identity elsewhere. The doorway is the one that started the
//! attempt; it can simply remember that it did.
//!
//! # Where it lives
//!
//! Beside the agent→conductor mappings: the `provisioning_intents` collection
//! of the doorway's own Mongo database (`MONGODB_DB`), next to
//! `conductor_registry`, reached through [`super::ConductorRegistry`].
//!
//! There are exactly three postures, and the middle one is a refusal:
//!
//! - **Mongo configured and reachable** → [`MongoIntentStore`].
//! - **Mongo configured but not usable** → still [`MongoIntentStore`], whose
//!   every operation fails until Mongo answers, so hosted provisioning is
//!   refused with an honest error (and heals by itself when Mongo returns).
//!   The log is never silently kept in memory on a doorway that was told to
//!   persist — an attempt that is forgotten at the next restart is exactly the
//!   second identity this log exists to prevent.
//! - **No Mongo configured at all** (a local-first developer's box) →
//!   [`MemoryIntentStore`], explicitly and logged.
//!
//! The key is the person's identifier, unique. Two writers racing on one
//! identifier — two requests, or two doorway processes on one database — are
//! settled by compare-and-set: [`IntentStore::insert_new`] is first-writer-wins
//! and every later change is [`IntentStore::replace`] against the version it
//! read. The loser re-reads and resolves on the winner's conductor.
//!
//! Two doorways on DIFFERENT databases (doorway-alpha and doorway-alpha-b today)
//! do not share this log and are not serialized by it.
//!
//! # What it holds, and for how long
//!
//! A record holds a person's identifier. It is deleted when the account is
//! closed or the agent deprovisioned ([`IntentStore::delete`],
//! [`IntentStore::delete_by_agent`]). An ABANDONED record — an attempt that
//! left nothing on any conductor — expires [`ABANDONED_RECORD_TTL`] after it
//! was abandoned (a Mongo TTL index on `expire_at`; the memory store purges on
//! [`IntentStore::purge_expired`]).
//!
//! An unauthenticated registration can create a record for any identifier, as
//! it can create a provisioned app for one. What that record can and cannot do
//! to the identifier's rightful holder: it names a conductor and an agent key
//! the DOORWAY generated and no caller ever held, so whoever completes the
//! registration for that identifier later is handed an app nobody else can
//! use — it pins the identifier to a conductor (an availability cost while
//! that conductor is down), never to someone else's identity.

use std::collections::HashMap;
use std::future::IntoFuture;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use bson::serde_helpers::{
    chrono_datetime_as_bson_datetime, chrono_datetime_as_bson_datetime_optional,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The Mongo collection holding the log, in the doorway's own database.
pub const PROVISIONING_INTENT_COLLECTION: &str = "provisioning_intents";

/// How long an abandoned record is kept before it expires.
///
/// A CHOICE, not a derivation: an abandoned record holds nothing the doorway
/// needs (nothing landed), only an identifier and a reason an operator may want
/// to read. A week is long enough to be seen on the metric and looked at, and
/// short enough that identifiers do not accumulate.
pub const ABANDONED_RECORD_TTL: std::time::Duration =
    std::time::Duration::from_secs(7 * 24 * 60 * 60);

/// How far this doorway knows its asking got. Forward-only, except into
/// [`IntentState::Abandoned`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    /// A conductor was chosen. Whether an install has been SENT is
    /// [`ProvisionIntent::install_sent_at`], not the state.
    Intended,
    /// `install_app` returned, or the app was seen on the conductor.
    Installed,
    /// Enabled, cells ready, agent mapping registered — handed to the caller.
    Enabled,
    /// The doorway's `create_human` call returned for this app.
    HumanCreated,
    /// The doorway's account row for this person was written.
    AccountRecorded,
    /// Terminal: the attempt is over and left nothing behind. A later
    /// registration starts fresh over this record.
    Abandoned,
}

impl IntentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Intended => "intended",
            Self::Installed => "installed",
            Self::Enabled => "enabled",
            Self::HumanCreated => "human_created",
            Self::AccountRecorded => "account_recorded",
            Self::Abandoned => "abandoned",
        }
    }

    /// Every state, for metric labels.
    pub const ALL: [IntentState; 6] = [
        Self::Intended,
        Self::Installed,
        Self::Enabled,
        Self::HumanCreated,
        Self::AccountRecorded,
        Self::Abandoned,
    ];
}

/// Why a LIVE record cannot be finished by the doorway on its own.
///
/// Each of these is a named place a record stops, so that "stuck" is a counted
/// state with a reason rather than a registration that answers
/// `PROVISIONING_UNRESOLVED` forever with nobody able to say why. None of them
/// is abandoned automatically: in every one, something may be on a conductor.
/// They are exits for an operator (`POST /admin/provisioning/recover`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StuckReason {
    /// An app exists under the record's id and the conductor's agent key for it
    /// is NOT the key this doorway recorded. It is not this person's app.
    KeyMismatch,
    /// An app already existed under the id when this doorway first tried to
    /// install one. The doorway has no evidence whose it is.
    AppConflict,
    /// The record names a conductor that is not in this doorway's pool, and no
    /// conductor in the pool holds an app with the recorded key.
    ConductorNotInPool,
    /// The app is installed but in a state the doorway cannot bring to ready
    /// (paused, unrecoverable, or it refuses to be enabled).
    AppUnusable,
    /// The app is installed and enabled but its cells did not become ready.
    CellsNotReady,
    /// An install was sent, but the record holds no agent key to prove or
    /// re-send it with.
    NoUsableKey,
}

impl StuckReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KeyMismatch => "key_mismatch",
            Self::AppConflict => "app_conflict",
            Self::ConductorNotInPool => "conductor_not_in_pool",
            Self::AppUnusable => "app_unusable",
            Self::CellsNotReady => "cells_not_ready",
            Self::NoUsableKey => "no_usable_key",
        }
    }

    /// Every reason, for metric labels.
    pub const ALL: [StuckReason; 6] = [
        Self::KeyMismatch,
        Self::AppConflict,
        Self::ConductorNotInPool,
        Self::AppUnusable,
        Self::CellsNotReady,
        Self::NoUsableKey,
    ];
}

/// One person's provisioning record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisionIntent {
    /// The person's identifier — the unique key.
    pub identifier: String,
    /// The conductor this doorway chose.
    pub conductor_id: String,
    /// The deterministic app id on that conductor.
    pub installed_app_id: String,
    /// The agent key generated for the install, canonical form. Recorded in
    /// the same write as `install_sent_at`, BEFORE the install is written, so a
    /// later re-send can use the same key — and so an app found under the id
    /// can be proven to be this person's.
    #[serde(default)]
    pub agent_pub_key: Option<String>,
    pub state: IntentState,
    /// When an `install_app` was last written to the conductor. `None` means
    /// nothing identity-bearing has ever been sent for this record.
    #[serde(default, with = "chrono_datetime_as_bson_datetime_optional")]
    pub install_sent_at: Option<DateTime<Utc>>,
    /// Why the record was abandoned.
    #[serde(default)]
    pub abandoned_reason: Option<String>,
    /// Why a live record cannot be finished without an operator. Cleared
    /// whenever the record moves.
    #[serde(default)]
    pub stuck: Option<StuckReason>,
    /// How many times, each at least one in-flight bound apart, a reachable
    /// conductor has answered that the sent install is NOT there and it could
    /// not be made to be. At the bound the record is abandoned: nothing landed.
    #[serde(default)]
    pub absent_past_bound: u32,
    /// When an abandoned record expires. `None` on a live record.
    #[serde(default, with = "chrono_datetime_as_bson_datetime_optional")]
    pub expire_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub updated_at: DateTime<Utc>,
    /// Compare-and-set token: every write is against the version it read.
    pub version: i64,
}

/// "Now", at the precision the log can store. BSON datetimes hold
/// milliseconds; truncating here keeps a record equal to itself after a round
/// trip through Mongo.
pub fn now() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(Utc::now().timestamp_millis()).unwrap_or_else(Utc::now)
}

impl ProvisionIntent {
    /// A fresh record: a conductor chosen, nothing sent.
    pub fn intended(identifier: &str, conductor_id: &str, installed_app_id: &str) -> Self {
        let now = now();
        Self {
            identifier: identifier.to_string(),
            conductor_id: conductor_id.to_string(),
            installed_app_id: installed_app_id.to_string(),
            agent_pub_key: None,
            state: IntentState::Intended,
            install_sent_at: None,
            abandoned_reason: None,
            stuck: None,
            absent_past_bound: 0,
            expire_at: None,
            created_at: now,
            updated_at: now,
            version: 0,
        }
    }

    /// Whether this record still speaks for the person. An abandoned record
    /// does not: a later registration starts fresh over it.
    pub fn is_live(&self) -> bool {
        self.state != IntentState::Abandoned
    }

    /// Whether anything identity-bearing may exist on the conductor because of
    /// this record: an install was written, or the log has seen it installed.
    pub fn may_have_landed(&self) -> bool {
        self.is_live() && (self.install_sent_at.is_some() || self.state >= IntentState::Installed)
    }
}

/// Counts for the metrics: one per state, the live records that have not
/// reached `enabled` and have not moved for longer than the in-flight bound,
/// and the stuck records by reason.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntentCounts {
    pub by_state: Vec<(IntentState, u64)>,
    pub stale: u64,
    pub stuck: Vec<(StuckReason, u64)>,
}

/// The log's storage. Two compare-and-set writes, reads, and deletes; all the
/// policy is in the provisioner, so the memory and Mongo stores cannot disagree
/// about it.
#[async_trait]
pub trait IntentStore: Send + Sync {
    async fn get(&self, identifier: &str) -> Result<Option<ProvisionIntent>, String>;

    /// First writer wins. `Ok(false)` = a record for this identifier already
    /// exists (live or abandoned); nothing was written.
    async fn insert_new(&self, intent: &ProvisionIntent) -> Result<bool, String>;

    /// Write `intent` (whose `version` must already be `expected_version + 1`)
    /// only if the stored record is still at `expected_version`. `Ok(false)` =
    /// someone else wrote first; nothing was written.
    async fn replace(
        &self,
        expected_version: i64,
        intent: &ProvisionIntent,
    ) -> Result<bool, String>;

    /// Forget one person's record (the account was closed).
    async fn delete(&self, identifier: &str) -> Result<(), String>;

    /// Forget every record holding this agent key (the agent was
    /// deprovisioned). Returns how many were removed.
    async fn delete_by_agent(&self, agent_pub_key: &str) -> Result<u64, String>;

    /// Remove abandoned records whose `expire_at` has passed. Returns how many.
    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, String>;

    /// Every live record that is stuck, for the operator's recovery report.
    async fn stuck_records(&self) -> Result<Vec<ProvisionIntent>, String>;

    /// How many records exist, of any state.
    async fn total(&self) -> Result<u64, String>;

    /// Records per state, stuck records per reason, and how many live
    /// not-yet-enabled records have not been updated since `stale_before`.
    async fn counts(&self, stale_before: DateTime<Utc>) -> Result<IntentCounts, String>;

    /// Whether this store can currently be relied on to remember an attempt.
    /// `false` refuses hosted provisioning.
    fn available(&self) -> bool {
        true
    }
}

/// A live record that never reached `enabled` and has not moved since
/// `stale_before`: an attempt nobody finished.
fn is_stale(intent: &ProvisionIntent, stale_before: DateTime<Utc>) -> bool {
    intent.is_live() && intent.state < IntentState::Enabled && intent.updated_at < stale_before
}

/// The log in process memory — the store when NO Mongo is configured (a
/// local-first developer's box), and the store every unit test drives. Sharing
/// one `Arc` between two registries models two doorway processes on one
/// database, or one doorway across a restart.
#[derive(Debug, Default)]
pub struct MemoryIntentStore {
    intents: Mutex<HashMap<String, ProvisionIntent>>,
}

impl MemoryIntentStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many records exist, of any state.
    pub fn len(&self) -> usize {
        self.intents.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl IntentStore for MemoryIntentStore {
    async fn get(&self, identifier: &str) -> Result<Option<ProvisionIntent>, String> {
        Ok(self
            .intents
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(identifier)
            .cloned())
    }

    async fn insert_new(&self, intent: &ProvisionIntent) -> Result<bool, String> {
        let mut intents = self.intents.lock().unwrap_or_else(|e| e.into_inner());
        if intents.contains_key(&intent.identifier) {
            return Ok(false);
        }
        intents.insert(intent.identifier.clone(), intent.clone());
        Ok(true)
    }

    async fn replace(
        &self,
        expected_version: i64,
        intent: &ProvisionIntent,
    ) -> Result<bool, String> {
        let mut intents = self.intents.lock().unwrap_or_else(|e| e.into_inner());
        match intents.get_mut(&intent.identifier) {
            Some(stored) if stored.version == expected_version => {
                *stored = intent.clone();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn delete(&self, identifier: &str) -> Result<(), String> {
        self.intents
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(identifier);
        Ok(())
    }

    async fn delete_by_agent(&self, agent_pub_key: &str) -> Result<u64, String> {
        let mut intents = self.intents.lock().unwrap_or_else(|e| e.into_inner());
        let before = intents.len();
        intents.retain(|_, i| i.agent_pub_key.as_deref() != Some(agent_pub_key));
        Ok((before - intents.len()) as u64)
    }

    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, String> {
        let mut intents = self.intents.lock().unwrap_or_else(|e| e.into_inner());
        let before = intents.len();
        // Keep a record if it is live, or if its expiry has not come.
        intents.retain(|_, i| i.is_live() || i.expire_at.is_none_or(|at| at > now));
        Ok((before - intents.len()) as u64)
    }

    async fn stuck_records(&self) -> Result<Vec<ProvisionIntent>, String> {
        let intents = self.intents.lock().unwrap_or_else(|e| e.into_inner());
        let mut stuck: Vec<ProvisionIntent> = intents
            .values()
            .filter(|i| i.is_live() && i.stuck.is_some())
            .cloned()
            .collect();
        stuck.sort_by(|a, b| a.identifier.cmp(&b.identifier));
        Ok(stuck)
    }

    async fn total(&self) -> Result<u64, String> {
        Ok(self.len() as u64)
    }

    async fn counts(&self, stale_before: DateTime<Utc>) -> Result<IntentCounts, String> {
        let intents = self.intents.lock().unwrap_or_else(|e| e.into_inner());
        let mut counts = IntentCounts::default();
        for state in IntentState::ALL {
            let n = intents.values().filter(|i| i.state == state).count() as u64;
            counts.by_state.push((state, n));
        }
        counts.stale = intents
            .values()
            .filter(|i| is_stale(i, stale_before))
            .count() as u64;
        for reason in StuckReason::ALL {
            let n = intents
                .values()
                .filter(|i| i.is_live() && i.stuck == Some(reason))
                .count() as u64;
            counts.stuck.push((reason, n));
        }
        Ok(counts)
    }
}

/// The log in the doorway's Mongo database.
///
/// Constructed without I/O. The indexes are created on first use and
/// re-attempted on every operation until they exist, so a Mongo that is down at
/// boot makes provisioning REFUSE (each operation errors) and heal on its own
/// when Mongo returns — it never downgrades to memory.
pub struct MongoIntentStore {
    collection: mongodb::Collection<ProvisionIntent>,
    indexed: tokio::sync::OnceCell<()>,
    /// Whether the last operation reached Mongo. Read by [`IntentStore::available`].
    healthy: AtomicBool,
}

impl MongoIntentStore {
    pub fn new(client: &mongodb::Client, db_name: &str) -> Self {
        Self {
            collection: client
                .database(db_name)
                .collection::<ProvisionIntent>(PROVISIONING_INTENT_COLLECTION),
            indexed: tokio::sync::OnceCell::new(),
            healthy: AtomicBool::new(false),
        }
    }

    /// Create the indexes now. Idempotent; called at boot so a healthy store is
    /// known healthy before the first registration.
    ///
    /// - `identifier`, unique — what makes [`IntentStore::insert_new`]
    ///   first-writer-wins across processes. A store whose unique index cannot
    ///   be created is not used.
    /// - `state` — the per-state counts.
    /// - `agent_pub_key` — deletion on deprovision.
    /// - `expire_at`, TTL 0 — abandoned records expire at the time they name.
    pub async fn ensure_indexes(&self) -> Result<(), String> {
        let result = self
            .indexed
            .get_or_try_init(|| async {
                use mongodb::options::IndexOptions;
                use mongodb::IndexModel;
                let named = |name: &str| IndexOptions::builder().name(name.to_string());
                let indexes = vec![
                    IndexModel::builder()
                        .keys(bson::doc! { "identifier": 1 })
                        .options(named("identifier_unique").unique(true).build())
                        .build(),
                    IndexModel::builder()
                        .keys(bson::doc! { "state": 1, "updated_at": 1 })
                        .options(named("state_updated_at").build())
                        .build(),
                    IndexModel::builder()
                        .keys(bson::doc! { "agent_pub_key": 1 })
                        .options(named("agent_pub_key").build())
                        .build(),
                    IndexModel::builder()
                        .keys(bson::doc! { "expire_at": 1 })
                        .options(
                            named("expire_at_ttl")
                                .expire_after(std::time::Duration::ZERO)
                                .build(),
                        )
                        .build(),
                ];
                self.collection
                    .create_indexes(indexes)
                    .await
                    .map(|_| ())
                    .map_err(|e| format!("provisioning intent indexes: {e}"))
            })
            .await
            .map(|_| ());
        self.healthy.store(result.is_ok(), Ordering::Relaxed);
        result
    }

    /// Run one operation, with the indexes ensured first and the store's health
    /// updated from the outcome.
    async fn run<T, Fut>(&self, what: &str, op: Fut) -> Result<T, String>
    where
        Fut: std::future::Future<Output = Result<T, mongodb::error::Error>>,
    {
        self.ensure_indexes().await?;
        let result = op
            .await
            .map_err(|e| format!("provisioning intent {what}: {e}"));
        self.healthy.store(result.is_ok(), Ordering::Relaxed);
        result
    }
}

/// Mongo's duplicate-key refusal — the unique index answering "already there".
fn is_duplicate_key(error: &mongodb::error::Error) -> bool {
    use mongodb::error::{ErrorKind, WriteFailure};
    matches!(
        error.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(write)) if write.code == 11000
    )
}

#[async_trait]
impl IntentStore for MongoIntentStore {
    async fn get(&self, identifier: &str) -> Result<Option<ProvisionIntent>, String> {
        self.run(
            "read",
            self.collection
                .find_one(bson::doc! { "identifier": identifier })
                .into_future(),
        )
        .await
    }

    async fn insert_new(&self, intent: &ProvisionIntent) -> Result<bool, String> {
        self.ensure_indexes().await?;
        let outcome = match self.collection.insert_one(intent).await {
            Ok(_) => Ok(true),
            // The unique index said "already there": an answer, not a fault.
            Err(e) if is_duplicate_key(&e) => Ok(false),
            Err(e) => Err(format!("provisioning intent insert: {e}")),
        };
        self.healthy.store(outcome.is_ok(), Ordering::Relaxed);
        outcome
    }

    async fn replace(
        &self,
        expected_version: i64,
        intent: &ProvisionIntent,
    ) -> Result<bool, String> {
        let result = self
            .run(
                "write",
                self.collection
                    .replace_one(
                        bson::doc! { "identifier": &intent.identifier, "version": expected_version },
                        intent,
                    )
                    .into_future(),
            )
            .await?;
        Ok(result.matched_count == 1)
    }

    async fn delete(&self, identifier: &str) -> Result<(), String> {
        self.run(
            "delete",
            self.collection
                .delete_one(bson::doc! { "identifier": identifier })
                .into_future(),
        )
        .await
        .map(|_| ())
    }

    async fn delete_by_agent(&self, agent_pub_key: &str) -> Result<u64, String> {
        self.run(
            "delete",
            self.collection
                .delete_many(bson::doc! { "agent_pub_key": agent_pub_key })
                .into_future(),
        )
        .await
        .map(|r| r.deleted_count)
    }

    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, String> {
        // The TTL index removes these on Mongo's own schedule (its monitor runs
        // about once a minute); this is the same predicate, applied now.
        self.run(
            "purge",
            self.collection
                .delete_many(bson::doc! {
                    "state": IntentState::Abandoned.as_str(),
                    "expire_at": { "$lte": bson::DateTime::from_chrono(now) },
                })
                .into_future(),
        )
        .await
        .map(|r| r.deleted_count)
    }

    async fn stuck_records(&self) -> Result<Vec<ProvisionIntent>, String> {
        use futures::TryStreamExt;
        self.ensure_indexes().await?;
        let outcome = async {
            let cursor = self
                .collection
                .find(bson::doc! {
                    "stuck": { "$ne": null },
                    "state": { "$ne": IntentState::Abandoned.as_str() },
                })
                .sort(bson::doc! { "identifier": 1 })
                .await?;
            cursor.try_collect::<Vec<ProvisionIntent>>().await
        }
        .await
        .map_err(|e| format!("provisioning intent read: {e}"));
        self.healthy.store(outcome.is_ok(), Ordering::Relaxed);
        outcome
    }

    async fn total(&self) -> Result<u64, String> {
        self.run(
            "count",
            self.collection.count_documents(bson::doc! {}).into_future(),
        )
        .await
    }

    async fn counts(&self, stale_before: DateTime<Utc>) -> Result<IntentCounts, String> {
        let mut counts = IntentCounts::default();
        for state in IntentState::ALL {
            let n = self
                .run(
                    "count",
                    self.collection
                        .count_documents(bson::doc! { "state": state.as_str() })
                        .into_future(),
                )
                .await?;
            counts.by_state.push((state, n));
        }
        // Mirrors `is_stale`.
        counts.stale = self
            .run(
                "count",
                self.collection
                    .count_documents(bson::doc! {
                        "state": { "$in": [
                            IntentState::Intended.as_str(),
                            IntentState::Installed.as_str(),
                        ] },
                        "updated_at": { "$lt": bson::DateTime::from_chrono(stale_before) },
                    })
                    .into_future(),
            )
            .await?;
        for reason in StuckReason::ALL {
            let n = self
                .run(
                    "count",
                    self.collection
                        .count_documents(bson::doc! {
                            "stuck": reason.as_str(),
                            "state": { "$ne": IntentState::Abandoned.as_str() },
                        })
                        .into_future(),
                )
                .await?;
            counts.stuck.push((reason, n));
        }
        Ok(counts)
    }

    fn available(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_first_writer_for_an_identifier_wins() {
        let store = MemoryIntentStore::new();
        let first = ProvisionIntent::intended("jane@example.com", "conductor-a", "app-a");
        let second = ProvisionIntent::intended("jane@example.com", "conductor-b", "app-b");

        assert!(store.insert_new(&first).await.unwrap());
        assert!(
            !store.insert_new(&second).await.unwrap(),
            "a second record for one identifier must be refused, not overwrite the first"
        );
        assert_eq!(
            store
                .get("jane@example.com")
                .await
                .unwrap()
                .unwrap()
                .conductor_id,
            "conductor-a"
        );
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn a_write_against_a_version_someone_else_moved_is_refused() {
        let store = MemoryIntentStore::new();
        let stored = ProvisionIntent::intended("jane@example.com", "conductor-a", "app-a");
        store.insert_new(&stored).await.unwrap();

        let mut mine = stored.clone();
        mine.version = 1;
        mine.state = IntentState::Installed;
        assert!(store.replace(0, &mine).await.unwrap());

        // A writer that still holds version 0 has lost.
        let mut theirs = stored.clone();
        theirs.version = 1;
        theirs.conductor_id = "conductor-b".to_string();
        assert!(!store.replace(0, &theirs).await.unwrap());
        assert_eq!(
            store
                .get("jane@example.com")
                .await
                .unwrap()
                .unwrap()
                .conductor_id,
            "conductor-a"
        );
    }

    #[test]
    fn only_a_record_that_sent_or_saw_an_install_may_have_landed() {
        let mut intent = ProvisionIntent::intended("jane@example.com", "conductor-a", "app-a");
        assert!(!intent.may_have_landed(), "nothing was sent");

        intent.install_sent_at = Some(now());
        assert!(intent.may_have_landed());

        intent.install_sent_at = None;
        intent.state = IntentState::Installed;
        assert!(intent.may_have_landed());

        intent.state = IntentState::Abandoned;
        assert!(
            !intent.may_have_landed(),
            "an abandoned record speaks for nobody"
        );
    }

    #[tokio::test]
    async fn stale_counts_live_unfinished_records_only() {
        let store = MemoryIntentStore::new();
        let old = now() - chrono::Duration::hours(1);
        for (who, state) in [
            ("unfinished@example.com", IntentState::Intended),
            ("done@example.com", IntentState::Enabled),
            ("gone@example.com", IntentState::Abandoned),
        ] {
            let mut intent = ProvisionIntent::intended(who, "conductor-a", "app");
            intent.state = state;
            intent.updated_at = old;
            store.insert_new(&intent).await.unwrap();
        }
        store
            .insert_new(&ProvisionIntent::intended(
                "fresh@example.com",
                "conductor-a",
                "app",
            ))
            .await
            .unwrap();

        let counts = store
            .counts(now() - chrono::Duration::minutes(1))
            .await
            .unwrap();
        assert_eq!(counts.stale, 1, "only the old, live, unfinished record");
        assert!(counts.by_state.contains(&(IntentState::Abandoned, 1)));
        assert!(counts.by_state.contains(&(IntentState::Intended, 2)));
    }

    /// Stuck records are counted by reason, and an abandoned record is not
    /// stuck whatever it once was.
    #[tokio::test]
    async fn stuck_records_are_counted_by_reason() {
        let store = MemoryIntentStore::new();
        for (who, stuck, state) in [
            (
                "a@example.com",
                Some(StuckReason::KeyMismatch),
                IntentState::Intended,
            ),
            (
                "b@example.com",
                Some(StuckReason::KeyMismatch),
                IntentState::Installed,
            ),
            (
                "c@example.com",
                Some(StuckReason::AppUnusable),
                IntentState::Installed,
            ),
            (
                "d@example.com",
                Some(StuckReason::AppUnusable),
                IntentState::Abandoned,
            ),
            ("e@example.com", None, IntentState::Enabled),
        ] {
            let mut intent = ProvisionIntent::intended(who, "conductor-a", "app");
            intent.stuck = stuck;
            intent.state = state;
            store.insert_new(&intent).await.unwrap();
        }

        let counts = store.counts(now()).await.unwrap();
        assert!(counts.stuck.contains(&(StuckReason::KeyMismatch, 2)));
        assert!(counts.stuck.contains(&(StuckReason::AppUnusable, 1)));
        assert!(counts.stuck.contains(&(StuckReason::AppConflict, 0)));
        let listed: Vec<String> = store
            .stuck_records()
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.identifier)
            .collect();
        assert_eq!(
            listed,
            vec!["a@example.com", "b@example.com", "c@example.com"]
        );
    }

    /// Records do not hold identifiers forever: a closed account's record is
    /// deleted, a deprovisioned agent's records go with it, and an abandoned
    /// record expires — while a live one never does.
    #[tokio::test]
    async fn records_are_deleted_on_closure_and_abandoned_ones_expire() {
        let store = MemoryIntentStore::new();
        let mut closed = ProvisionIntent::intended("closed@example.com", "conductor-a", "app");
        closed.agent_pub_key = Some("uhCAkClosed".to_string());
        let mut hosted = ProvisionIntent::intended("hosted@example.com", "conductor-a", "app");
        hosted.agent_pub_key = Some("uhCAkHosted".to_string());
        let mut abandoned = ProvisionIntent::intended("gone@example.com", "conductor-a", "app");
        abandoned.state = IntentState::Abandoned;
        abandoned.expire_at = Some(now() - chrono::Duration::seconds(1));
        let mut live_with_stale_expiry =
            ProvisionIntent::intended("live@example.com", "conductor-a", "app");
        live_with_stale_expiry.expire_at = Some(now() - chrono::Duration::seconds(1));
        for intent in [&closed, &hosted, &abandoned, &live_with_stale_expiry] {
            store.insert_new(intent).await.unwrap();
        }

        store.delete("closed@example.com").await.unwrap();
        assert_eq!(store.delete_by_agent("uhCAkHosted").await.unwrap(), 1);
        assert_eq!(store.purge_expired(now()).await.unwrap(), 1);

        assert_eq!(store.total().await.unwrap(), 1);
        assert!(
            store.get("live@example.com").await.unwrap().is_some(),
            "only an ABANDONED record expires"
        );
    }

    /// The wire names are the stored names; renaming a variant would orphan
    /// every record already written.
    #[test]
    fn state_and_reason_names_are_stable() {
        for state in IntentState::ALL {
            assert_eq!(
                serde_json::to_value(state).unwrap(),
                serde_json::Value::String(state.as_str().to_string())
            );
        }
        for reason in StuckReason::ALL {
            assert_eq!(
                serde_json::to_value(reason).unwrap(),
                serde_json::Value::String(reason.as_str().to_string())
            );
        }
    }

    // ── against a real mongod ───────────────────────────────────────────────
    //
    // Run when `DOORWAY_TEST_MONGODB_URI` names a mongod. Each test uses a
    // throwaway database and drops it. With the variable unset they print that
    // they did not run — they do not pretend to have passed the Mongo path.

    const MONGO_TEST_ENV: &str = "DOORWAY_TEST_MONGODB_URI";

    struct ThrowawayDb {
        client: mongodb::Client,
        name: String,
    }

    impl ThrowawayDb {
        async fn open(test: &str) -> Option<Self> {
            let Ok(uri) = std::env::var(MONGO_TEST_ENV) else {
                eprintln!(
                    "NOT RUN ({test}): {MONGO_TEST_ENV} is not set — no mongod was exercised"
                );
                return None;
            };
            let client = mongodb::Client::with_uri_str(&uri)
                .await
                .unwrap_or_else(|e| panic!("{MONGO_TEST_ENV}={uri} is not a usable URI: {e}"));
            let name = format!(
                "doorway_intent_test_{}_{}",
                std::process::id(),
                uuid::Uuid::new_v4().simple()
            );
            // Fail loudly if the mongod named is not reachable: a set variable
            // that silently skips is the same lie as an unset one.
            client
                .database(&name)
                .run_command(bson::doc! { "ping": 1 })
                .await
                .unwrap_or_else(|e| panic!("mongod at {uri} did not answer a ping: {e}"));
            Some(Self { client, name })
        }

        fn store(&self) -> MongoIntentStore {
            MongoIntentStore::new(&self.client, &self.name)
        }

        async fn drop_database(self) {
            self.client
                .database(&self.name)
                .drop()
                .await
                .expect("drop the throwaway database");
        }
    }

    #[tokio::test]
    async fn mongo_the_unique_index_makes_the_first_writer_win() {
        let Some(db) = ThrowawayDb::open("unique index").await else {
            return;
        };
        let store = db.store();
        assert!(!store.available(), "not healthy before first contact");
        store.ensure_indexes().await.expect("indexes");
        assert!(store.available());

        let first = ProvisionIntent::intended("jane@example.com", "conductor-a", "app-a");
        let second = ProvisionIntent::intended("jane@example.com", "conductor-b", "app-b");
        assert!(store.insert_new(&first).await.unwrap());
        assert!(
            !store.insert_new(&second).await.unwrap(),
            "a duplicate key must be the answer `false`, not an error"
        );
        assert!(store.available(), "a duplicate key is not a store fault");

        let read = store.get("jane@example.com").await.unwrap().unwrap();
        assert_eq!(read, first, "a record survives the round trip unchanged");
        assert_eq!(store.total().await.unwrap(), 1);

        let names: Vec<String> = db
            .client
            .database(&db.name)
            .collection::<bson::Document>(PROVISIONING_INTENT_COLLECTION)
            .list_index_names()
            .await
            .unwrap();
        for index in [
            "identifier_unique",
            "state_updated_at",
            "agent_pub_key",
            "expire_at_ttl",
        ] {
            assert!(
                names.iter().any(|n| n == index),
                "{index} missing: {names:?}"
            );
        }
        db.drop_database().await;
    }

    #[tokio::test]
    async fn mongo_concurrent_first_inserts_yield_one_record() {
        let Some(db) = ThrowawayDb::open("concurrent insert").await else {
            return;
        };
        let store = std::sync::Arc::new(db.store());
        store.ensure_indexes().await.unwrap();

        let mut tasks = Vec::new();
        for n in 0..16 {
            let store = std::sync::Arc::clone(&store);
            tasks.push(tokio::spawn(async move {
                let intent =
                    ProvisionIntent::intended("jane@example.com", &format!("conductor-{n}"), "app");
                store.insert_new(&intent).await.unwrap()
            }));
        }
        let mut winners = 0;
        for task in tasks {
            if task.await.unwrap() {
                winners += 1;
            }
        }
        assert_eq!(winners, 1, "exactly one concurrent first writer may win");
        assert_eq!(store.total().await.unwrap(), 1);
        db.drop_database().await;
    }

    #[tokio::test]
    async fn mongo_a_versioned_replace_under_contention_has_one_winner() {
        let Some(db) = ThrowawayDb::open("versioned replace").await else {
            return;
        };
        let store = std::sync::Arc::new(db.store());
        let stored = ProvisionIntent::intended("jane@example.com", "conductor-a", "app");
        assert!(store.insert_new(&stored).await.unwrap());

        let mut tasks = Vec::new();
        for n in 0..16 {
            let store = std::sync::Arc::clone(&store);
            let mut next = stored.clone();
            tasks.push(tokio::spawn(async move {
                next.version = 1;
                next.conductor_id = format!("conductor-{n}");
                next.install_sent_at = Some(now());
                next.agent_pub_key = Some(format!("uhCAk{n}"));
                (store.replace(0, &next).await.unwrap(), next)
            }));
        }
        let mut won = Vec::new();
        for task in tasks {
            let (written, intent) = task.await.unwrap();
            if written {
                won.push(intent);
            }
        }
        assert_eq!(won.len(), 1, "one writer per version");
        assert_eq!(
            store.get("jane@example.com").await.unwrap().unwrap(),
            won[0],
            "the stored record is the winner's, whole"
        );
        // A stale version loses even with nobody racing it.
        let mut late = stored.clone();
        late.version = 1;
        assert!(!store.replace(0, &late).await.unwrap());
        db.drop_database().await;
    }

    #[tokio::test]
    async fn mongo_counts_stale_stuck_and_expiry_match_the_memory_store() {
        let Some(db) = ThrowawayDb::open("counts").await else {
            return;
        };
        let mongo = db.store();
        let memory = MemoryIntentStore::new();
        let old = now() - chrono::Duration::hours(1);

        let mut records = Vec::new();
        for (who, state, stuck, updated, agent) in [
            ("old-intended", IntentState::Intended, None, old, "k1"),
            (
                "old-installed",
                IntentState::Installed,
                Some(StuckReason::AppUnusable),
                old,
                "k2",
            ),
            ("old-enabled", IntentState::Enabled, None, old, "k3"),
            (
                "fresh-intended",
                IntentState::Intended,
                Some(StuckReason::KeyMismatch),
                now(),
                "k4",
            ),
            (
                "old-abandoned",
                IntentState::Abandoned,
                Some(StuckReason::KeyMismatch),
                old,
                "k5",
            ),
        ] {
            let mut intent =
                ProvisionIntent::intended(&format!("{who}@example.com"), "conductor-a", "app");
            intent.state = state;
            intent.stuck = stuck;
            intent.updated_at = updated;
            intent.agent_pub_key = Some(agent.to_string());
            if state == IntentState::Abandoned {
                intent.expire_at = Some(now() - chrono::Duration::seconds(5));
            }
            records.push(intent);
        }
        for intent in &records {
            assert!(mongo.insert_new(intent).await.unwrap());
            assert!(memory.insert_new(intent).await.unwrap());
        }

        let bound = now() - chrono::Duration::minutes(1);
        let from_mongo = mongo.counts(bound).await.unwrap();
        assert_eq!(from_mongo, memory.counts(bound).await.unwrap());
        assert_eq!(from_mongo.stale, 2);

        let stuck: Vec<String> = mongo
            .stuck_records()
            .await
            .unwrap()
            .into_iter()
            .map(|i| i.identifier)
            .collect();
        assert_eq!(
            stuck,
            vec!["fresh-intended@example.com", "old-installed@example.com"]
        );

        assert_eq!(mongo.delete_by_agent("k3").await.unwrap(), 1);
        mongo.delete("old-intended@example.com").await.unwrap();
        assert_eq!(
            mongo.purge_expired(now()).await.unwrap(),
            1,
            "only the abandoned record whose expiry has passed"
        );
        assert_eq!(mongo.total().await.unwrap(), 2);
        db.drop_database().await;
    }

    #[tokio::test]
    async fn mongo_a_reopened_store_finds_what_the_last_one_wrote() {
        let Some(db) = ThrowawayDb::open("reopen").await else {
            return;
        };
        let written = {
            let first = db.store();
            let mut intent = ProvisionIntent::intended("jane@example.com", "conductor-a", "app");
            intent.agent_pub_key = Some("uhCAkJane".to_string());
            intent.install_sent_at = Some(now());
            assert!(first.insert_new(&intent).await.unwrap());
            intent
        };

        // A new process: a new store object, indexes ensured again.
        let reopened = db.store();
        reopened.ensure_indexes().await.expect("idempotent");
        assert_eq!(
            reopened.get("jane@example.com").await.unwrap(),
            Some(written.clone())
        );
        assert!(
            !reopened.insert_new(&written).await.unwrap(),
            "the unique index outlives the store object"
        );
        db.drop_database().await;
    }
}
