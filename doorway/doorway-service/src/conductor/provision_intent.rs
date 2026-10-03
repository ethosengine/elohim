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
//! recovery problem (the pool scan, `AgentProvisioner::recover_by_pool_scan`),
//! never a correctness problem with the identities themselves.
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
//! `conductor_registry`, reached through [`super::ConductorRegistry`]. With no
//! Mongo configured it is process memory, exactly as the mappings are.
//!
//! The key is the person's identifier, unique. Two writers racing on one
//! identifier — two requests, or two doorway processes on one database — are
//! settled by compare-and-set: [`IntentStore::insert_new`] is first-writer-wins
//! and every later change is [`IntentStore::replace`] against the version it
//! read. The loser re-reads and resolves on the winner's conductor.
//!
//! Two doorways on DIFFERENT databases (doorway-alpha and doorway-alpha-b today)
//! do not share this log and are not serialized by it.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The Mongo collection holding the log, in the doorway's own database.
pub const PROVISIONING_INTENT_COLLECTION: &str = "provisioning_intents";

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
    /// later re-send can use the same key.
    #[serde(default)]
    pub agent_pub_key: Option<String>,
    pub state: IntentState,
    /// When an `install_app` was last written to the conductor. `None` means
    /// nothing identity-bearing has ever been sent for this record.
    #[serde(default)]
    pub install_sent_at: Option<DateTime<Utc>>,
    /// Why the record was abandoned.
    #[serde(default)]
    pub abandoned_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Compare-and-set token: every write is against the version it read.
    pub version: i64,
}

impl ProvisionIntent {
    /// A fresh record: a conductor chosen, nothing sent.
    pub fn intended(identifier: &str, conductor_id: &str, installed_app_id: &str) -> Self {
        let now = Utc::now();
        Self {
            identifier: identifier.to_string(),
            conductor_id: conductor_id.to_string(),
            installed_app_id: installed_app_id.to_string(),
            agent_pub_key: None,
            state: IntentState::Intended,
            install_sent_at: None,
            abandoned_reason: None,
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

/// Counts for the metric: one per state, plus the live records that have not
/// reached `enabled` and have not moved for longer than the in-flight bound.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntentCounts {
    pub by_state: Vec<(IntentState, u64)>,
    pub stale: u64,
}

/// The log's storage. Two compare-and-set writes and two reads; all the policy
/// is in the provisioner, so the memory and Mongo stores cannot disagree about
/// it.
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

    /// Records per state, and how many live not-yet-enabled records have not
    /// been updated since `stale_before`.
    async fn counts(&self, stale_before: DateTime<Utc>) -> Result<IntentCounts, String>;
}

/// The log in process memory — the store when no Mongo is configured, and the
/// store every test drives. Sharing one `Arc` between two registries models two
/// doorway processes on one database, or one doorway across a restart.
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
        Ok(counts)
    }
}

/// A live record that never reached `enabled` and has not moved since
/// `stale_before`: an attempt nobody finished. This is the debt a reaper would
/// work from; nothing here acts on it.
fn is_stale(intent: &ProvisionIntent, stale_before: DateTime<Utc>) -> bool {
    intent.is_live() && intent.state < IntentState::Enabled && intent.updated_at < stale_before
}

/// The log in the doorway's Mongo database.
pub struct MongoIntentStore {
    collection: mongodb::Collection<ProvisionIntent>,
}

impl MongoIntentStore {
    /// Open the collection and make the identifier unique. The unique index is
    /// what makes [`IntentStore::insert_new`] first-writer-wins across
    /// processes; a store whose index could not be created is refused rather
    /// than used without it.
    pub async fn open(client: &mongodb::Client, db_name: &str) -> Result<Self, String> {
        let collection = client
            .database(db_name)
            .collection::<ProvisionIntent>(PROVISIONING_INTENT_COLLECTION);
        let index = mongodb::IndexModel::builder()
            .keys(bson::doc! { "identifier": 1 })
            .options(
                mongodb::options::IndexOptions::builder()
                    .unique(true)
                    .name("identifier_unique".to_string())
                    .build(),
            )
            .build();
        collection
            .create_index(index)
            .await
            .map_err(|e| format!("provisioning intent index: {e}"))?;
        Ok(Self { collection })
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
        self.collection
            .find_one(bson::doc! { "identifier": identifier })
            .await
            .map_err(|e| format!("provisioning intent read: {e}"))
    }

    async fn insert_new(&self, intent: &ProvisionIntent) -> Result<bool, String> {
        match self.collection.insert_one(intent).await {
            Ok(_) => Ok(true),
            Err(e) if is_duplicate_key(&e) => Ok(false),
            Err(e) => Err(format!("provisioning intent insert: {e}")),
        }
    }

    async fn replace(
        &self,
        expected_version: i64,
        intent: &ProvisionIntent,
    ) -> Result<bool, String> {
        let result = self
            .collection
            .replace_one(
                bson::doc! { "identifier": &intent.identifier, "version": expected_version },
                intent,
            )
            .await
            .map_err(|e| format!("provisioning intent write: {e}"))?;
        Ok(result.matched_count == 1)
    }

    async fn counts(&self, stale_before: DateTime<Utc>) -> Result<IntentCounts, String> {
        let mut counts = IntentCounts::default();
        for state in IntentState::ALL {
            let n = self
                .collection
                .count_documents(bson::doc! { "state": state.as_str() })
                .await
                .map_err(|e| format!("provisioning intent count: {e}"))?;
            counts.by_state.push((state, n));
        }
        // Mirrors `is_stale`. `updated_at` is stored as chrono's serde form (an
        // RFC 3339 UTC string), so the bound is serialized the same way and
        // compared as text. Text order is exact down to the second; within one
        // second it can misorder by fractional digits, which cannot matter for
        // a bound measured in minutes.
        let bound =
            bson::to_bson(&stale_before).map_err(|e| format!("provisioning intent count: {e}"))?;
        counts.stale = self
            .collection
            .count_documents(bson::doc! {
                "state": { "$in": [IntentState::Intended.as_str(), IntentState::Installed.as_str()] },
                "updated_at": { "$lt": bound },
            })
            .await
            .map_err(|e| format!("provisioning intent count: {e}"))?;
        Ok(counts)
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

        intent.install_sent_at = Some(Utc::now());
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
        let old = Utc::now() - chrono::Duration::hours(1);
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
            .counts(Utc::now() - chrono::Duration::minutes(1))
            .await
            .unwrap();
        assert_eq!(counts.stale, 1, "only the old, live, unfinished record");
        assert!(counts.by_state.contains(&(IntentState::Abandoned, 1)));
        assert!(counts.by_state.contains(&(IntentState::Intended, 2)));
    }

    /// The wire names are the stored names; renaming a variant would orphan
    /// every record already written.
    #[test]
    fn state_names_are_stable() {
        for state in IntentState::ALL {
            assert_eq!(
                serde_json::to_value(state).unwrap(),
                serde_json::Value::String(state.as_str().to_string())
            );
        }
    }
}
