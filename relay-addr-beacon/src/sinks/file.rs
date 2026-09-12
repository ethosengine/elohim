//! `file` — the household-ownable projection of shared membership.
//!
//! Shared membership is a SET OF ORIGINS eligible to serve one public name.
//! The Cloudflare sink projects that set as `A`/`AAAA` records for the fleet;
//! this sink projects the SAME set into a JSON document on a filesystem the
//! household owns. Both are projections of one protocol fact ("address-set
//! contribution with ownership + freshness"), reached through the SAME
//! `reconcile_membership` call, driven by the SAME serving probe and the SAME
//! `state::Membership` join/leave hysteresis. Nothing about membership is
//! decided here — this sink only writes what the shared decision already is.
//!
//! Why a file sink exists at all: a household cannot certify public-name
//! transition against a DNS zone it does not own, and a test-only proxy that
//! does not execute the real routing decision certifies nothing. A beacon leg
//! per doorway, writing its own entry into one membership document, IS the
//! routing apparatus — the same code path the fleet runs, with a
//! household-ownable projection target instead of Cloudflare's.
//!
//! Ownership rules mirror the Cloudflare shared lane exactly:
//!
//! - **Exact-owner writes.** A leg only ever adds, refreshes or removes the
//!   member entry whose `owner` is its own. A sibling's entry is never
//!   touched — not rewritten, not reordered into a different value, not
//!   reaped. (The file sink deliberately has NO stale-sibling reap: on one
//!   host the legs share a clock and a filesystem, so an absent sibling is an
//!   absent PROCESS, which is a household fact to read, not a record to
//!   garbage-collect.)
//! - **Freshness.** A serving leg re-stamps its own `updated_at` once its
//!   stamp is older than `--shared-refresh-secs`, so a leg whose origin never
//!   changes still proves it is alive.
//! - **Restart starts withdrawn.** Membership state is ephemeral evidence
//!   (`state::Membership`), so a restarted leg withdraws its entry and must
//!   earn it back with `--serving-join-after` consecutive serving probes.
//!
//! Two legs write one file, so every read-modify-write is taken under an
//! advisory lock file (`<path>.lock`, `O_EXCL` create) and committed by
//! `rename` over the target, which is atomic on POSIX: a reader never observes
//! a half-written document, and a leg never clobbers a sibling's concurrent
//! write.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::{debug, info, warn};

use super::{AddrUpdate, Sink};

/// How long to wait for a sibling leg's lock before giving up this cycle.
/// Failing the cycle is correct: the caller retries on the next probe, and the
/// membership decision itself is unchanged by the delay.
const LOCK_WAIT: Duration = Duration::from_secs(5);
/// Poll interval while waiting for the lock. Short — the critical section is a
/// few-kilobyte read, edit and rename on a local filesystem.
const LOCK_POLL: Duration = Duration::from_millis(25);
/// A lock file older than this is treated as abandoned (its holder was killed
/// mid-write) and broken. Must be far longer than any honest critical section.
const LOCK_STALE: Duration = Duration::from_secs(30);

/// One origin currently eligible to serve the public name, and the owner that
/// contributed it. `owner` is the identity key: exactly one entry per owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipMember {
    /// The beacon leg that owns this entry (`--record-owner`).
    pub owner: String,
    /// The origin a client should try for the public name (`scheme://host:port`).
    pub origin: String,
    /// RFC3339 UTC instant this entry was last written by its owner.
    pub updated_at: String,
}

/// The membership document: one public name, the set of origins eligible to
/// serve it. This is the household's projection of the same set the Cloudflare
/// sink projects as A records.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipDoc {
    /// The public name this set serves.
    pub name: String,
    /// Eligible origins, ordered by owner so the document is deterministic and
    /// two legs cannot disagree about ordering alone.
    #[serde(default)]
    pub members: Vec<MembershipMember>,
    /// RFC3339 UTC instant the document itself last changed.
    pub updated_at: String,
}

impl MembershipDoc {
    /// The entry contributed by `owner`, if it is currently a member.
    ///
    /// A reader's accessor: the write path needs the mutable form and goes
    /// through `FileMembershipSink::join`, so this exists for tests and for
    /// any future diagnostic that reads the document back.
    #[cfg(test)]
    pub fn member(&self, owner: &str) -> Option<&MembershipMember> {
        self.members.iter().find(|m| m.owner == owner)
    }
}

/// A configured file membership sink — one beacon leg's contribution to one
/// membership document.
pub struct FileMembershipSink {
    path: PathBuf,
    public_name: String,
    owner: String,
    origin: String,
    refresh_secs: u64,
}

/// Held for the duration of one read-modify-write. Dropping it releases the
/// advisory lock.
struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                warn!(lock = %self.path.display(), %error, "could not release membership lock");
            }
        }
    }
}

impl FileMembershipSink {
    pub fn new(
        path: PathBuf,
        public_name: String,
        owner: String,
        origin: String,
        refresh_secs: u64,
    ) -> Self {
        Self {
            path,
            public_name,
            owner,
            origin,
            refresh_secs,
        }
    }

    /// Does this leg write the document for `record_name`? Compared on the
    /// normalized lane key, so either DNS spelling of the name selects it.
    pub fn owns_lane(&self, record_name: &str) -> bool {
        crate::config::lane_key(&self.public_name) == crate::config::lane_key(record_name)
    }

    fn lock_path(&self) -> PathBuf {
        let mut name = self.path.as_os_str().to_os_string();
        name.push(".lock");
        PathBuf::from(name)
    }

    /// Reconcile ONLY this leg's contribution.
    ///
    /// `serving` is the already-decided membership verdict from
    /// `state::Membership` (join/leave hysteresis over the serving probe) —
    /// identical to what the Cloudflare sink is handed on the same cycle.
    pub async fn reconcile_membership(&self, serving: bool) -> Result<()> {
        let _guard = self.acquire_lock().await?;
        let existed = self.path.exists();
        let mut doc = self.read_doc();
        let now = now_rfc3339()?;
        let mut changed = doc.name != self.public_name;
        doc.name = self.public_name.clone();

        if serving {
            changed |= self.join(&mut doc, &now);
        } else {
            let before = doc.members.len();
            doc.members.retain(|member| member.owner != self.owner);
            if doc.members.len() != before {
                info!(
                    owner = %self.owner,
                    name = %self.public_name,
                    "withdrew own entry from household membership"
                );
                changed = true;
            }
        }

        // Materialise the document even on a no-op first cycle: a household
        // reading "is there a membership authority at all?" must not have to
        // distinguish "no file yet" from "the beacon never started".
        if !changed && existed {
            debug!(owner = %self.owner, "household membership already reconciled");
            return Ok(());
        }
        doc.members.sort_by(|a, b| a.owner.cmp(&b.owner));
        doc.updated_at = now;
        self.write_atomically(&doc)
    }

    /// Add or refresh our own entry. Returns whether the document changed.
    fn join(&self, doc: &mut MembershipDoc, now: &str) -> bool {
        let mut changed = false;
        // Defensive: a hand-edited document could carry two entries for one
        // owner. Exactly one entry per owner is the invariant this sink keeps,
        // and "without duplicating" is a contract the household asserts on.
        let mut seen = false;
        doc.members.retain(|member| {
            if member.owner != self.owner {
                return true;
            }
            if seen {
                changed = true;
                return false;
            }
            seen = true;
            true
        });

        match doc.members.iter_mut().find(|m| m.owner == self.owner) {
            Some(mine) => {
                let stale = age_secs(&mine.updated_at).is_none_or(|age| age >= self.refresh_secs);
                if mine.origin != self.origin {
                    mine.origin = self.origin.clone();
                    changed = true;
                }
                if changed || stale {
                    mine.updated_at = now.to_string();
                    changed = true;
                }
            }
            None => {
                doc.members.push(MembershipMember {
                    owner: self.owner.clone(),
                    origin: self.origin.clone(),
                    updated_at: now.to_string(),
                });
                info!(
                    owner = %self.owner,
                    origin = %self.origin,
                    name = %self.public_name,
                    "joined household membership"
                );
                changed = true;
            }
        }
        changed
    }

    /// Read the current document. A missing file is an empty set (first run);
    /// an unparseable one is logged and treated the same way rather than
    /// wedging the leg forever — the next write re-establishes a valid
    /// document, and our own entry is re-added by the join path above.
    fn read_doc(&self) -> MembershipDoc {
        match std::fs::read_to_string(&self.path) {
            Ok(raw) => match serde_json::from_str::<MembershipDoc>(&raw) {
                Ok(doc) => doc,
                Err(error) => {
                    warn!(
                        path = %self.path.display(),
                        %error,
                        "membership document is corrupt — rebuilding from this leg's own entry"
                    );
                    MembershipDoc::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => MembershipDoc::default(),
            Err(error) => {
                warn!(
                    path = %self.path.display(),
                    %error,
                    "membership document unreadable — rebuilding from this leg's own entry"
                );
                MembershipDoc::default()
            }
        }
    }

    /// Write through a sibling temp file plus `rename`, which is atomic on
    /// POSIX: a concurrent reader observes either the old document or the new
    /// one, never a truncated one.
    fn write_atomically(&self, doc: &MembershipDoc) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("creating membership directory {}", parent.display())
                })?;
            }
        }
        let serialized = serde_json::to_string_pretty(doc).context("serializing membership")?;
        // The temp name carries the owner so two legs never contend for one
        // temp path even inside the (already exclusive) critical section.
        let tmp = self.path.with_extension(format!("{}.tmp", self.owner));
        std::fs::write(&tmp, format!("{serialized}\n"))
            .with_context(|| format!("writing temp membership file {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).with_context(|| {
            format!(
                "renaming temp membership file {} -> {}",
                tmp.display(),
                self.path.display()
            )
        })?;
        Ok(())
    }

    /// Take the advisory lock guarding this document. `create_new` is the
    /// atomic `O_EXCL` primitive every POSIX filesystem gives us without a
    /// dependency; an abandoned lock (holder killed mid-write) is broken after
    /// `LOCK_STALE` so one crash cannot wedge the household forever.
    async fn acquire_lock(&self) -> Result<LockGuard> {
        let lock = self.lock_path();
        if let Some(parent) = lock.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("creating membership directory {}", parent.display())
                })?;
            }
        }
        let deadline = SystemTime::now() + LOCK_WAIT;
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock)
            {
                Ok(_) => return Ok(LockGuard { path: lock }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if lock_is_abandoned(&lock) {
                        warn!(lock = %lock.display(), "breaking abandoned membership lock");
                        let _ = std::fs::remove_file(&lock);
                        continue;
                    }
                    if SystemTime::now() >= deadline {
                        return Err(anyhow!(
                            "membership lock {} still held after {:?} — sibling leg is wedged",
                            lock.display(),
                            LOCK_WAIT
                        ));
                    }
                    tokio::time::sleep(LOCK_POLL).await;
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("taking membership lock {}", lock.display()))
                }
            }
        }
    }
}

impl Sink for FileMembershipSink {
    fn name(&self) -> &'static str {
        "file"
    }

    /// Membership here is keyed by ORIGIN, not by the detected WAN address, so
    /// an address publish has nothing to project. The serving probe drives
    /// every write (see `reconcile_membership`).
    async fn publish(&self, _update: &AddrUpdate) -> Result<()> {
        debug!("file membership sink: address publish is a no-op (membership is origin-keyed)");
        Ok(())
    }
}

/// Has a lock file outlived any honest critical section?
fn lock_is_abandoned(lock: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(lock) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age > LOCK_STALE)
}

/// Current UTC instant, RFC3339 — the same stamp vocabulary the Cloudflare
/// lane reads `modified_on` in.
fn now_rfc3339() -> Result<String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the unix epoch")?
        .as_secs();
    let secs = i64::try_from(secs).context("system clock is beyond the representable range")?;
    OffsetDateTime::from_unix_timestamp(secs)
        .context("unrepresentable current time")?
        .format(&Rfc3339)
        .context("formatting current time as RFC3339")
}

/// Age in seconds of an RFC3339 stamp, or `None` when it cannot be parsed
/// (which callers treat as stale — an unreadable stamp proves no freshness).
fn age_secs(stamp: &str) -> Option<u64> {
    let parsed = OffsetDateTime::parse(stamp, &Rfc3339).ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let now = i64::try_from(now).ok()?;
    u64::try_from(now - parsed.unix_timestamp()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("beacon-membership-{tag}-{nanos}/name.json"))
    }

    fn leg(path: &Path, owner: &str, origin: &str) -> FileMembershipSink {
        FileMembershipSink::new(
            path.to_path_buf(),
            "elohim.local".into(),
            owner.into(),
            origin.into(),
            300,
        )
    }

    fn read(path: &Path) -> MembershipDoc {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn withdrawal_materialises_the_document_and_names_the_public_name() {
        let path = scratch("materialise");
        leg(&path, "alpha", "http://localhost:8888")
            .reconcile_membership(false)
            .await
            .unwrap();
        let doc = read(&path);
        assert_eq!(doc.name, "elohim.local");
        assert!(doc.members.is_empty());
        // No residue: neither the lock nor the temp file survives a cycle.
        assert!(!path.with_extension("alpha.tmp").exists());
        assert!(!PathBuf::from(format!("{}.lock", path.display())).exists());
    }

    #[tokio::test]
    async fn a_leg_only_ever_writes_its_own_entry() {
        let path = scratch("exact-owner");
        let alpha = leg(&path, "alpha", "http://localhost:8888");
        let apex = leg(&path, "apex", "http://localhost:8889");
        alpha.reconcile_membership(true).await.unwrap();
        apex.reconcile_membership(true).await.unwrap();

        let before = read(&path);
        assert_eq!(before.members.len(), 2);
        let sibling_before = before.member("alpha").cloned().unwrap();

        apex.reconcile_membership(false).await.unwrap();
        let after = read(&path);
        assert_eq!(after.members.len(), 1);
        assert!(after.member("apex").is_none());
        // Byte-identical, stamp included: leg B's withdrawal did not rewrite
        // leg A's entry, only removed its own.
        assert_eq!(after.member("alpha").cloned().unwrap(), sibling_before);
    }

    #[tokio::test]
    async fn rejoining_never_duplicates_an_owner() {
        let path = scratch("no-duplicate");
        let alpha = leg(&path, "alpha", "http://localhost:8888");
        for _ in 0..4 {
            alpha.reconcile_membership(true).await.unwrap();
        }
        alpha.reconcile_membership(false).await.unwrap();
        alpha.reconcile_membership(true).await.unwrap();
        let doc = read(&path);
        assert_eq!(doc.members.iter().filter(|m| m.owner == "alpha").count(), 1);
    }

    #[tokio::test]
    async fn a_hand_edited_duplicate_is_collapsed_to_one_entry() {
        let path = scratch("collapse");
        let alpha = leg(&path, "alpha", "http://localhost:8888");
        alpha.reconcile_membership(true).await.unwrap();
        let mut doc = read(&path);
        let mine = doc.member("alpha").cloned().unwrap();
        doc.members.push(mine);
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();

        alpha.reconcile_membership(true).await.unwrap();
        assert_eq!(read(&path).members.len(), 1);
    }

    #[tokio::test]
    async fn a_corrupt_document_is_rebuilt_rather_than_wedging_the_leg() {
        let path = scratch("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"\x00not json at all}}}").unwrap();
        leg(&path, "alpha", "http://localhost:8888")
            .reconcile_membership(true)
            .await
            .unwrap();
        let doc = read(&path);
        assert_eq!(doc.member("alpha").unwrap().origin, "http://localhost:8888");
    }

    #[tokio::test]
    async fn an_origin_change_rewrites_only_our_own_entry() {
        let path = scratch("origin-change");
        leg(&path, "alpha", "http://localhost:8888")
            .reconcile_membership(true)
            .await
            .unwrap();
        leg(&path, "apex", "http://localhost:8889")
            .reconcile_membership(true)
            .await
            .unwrap();
        leg(&path, "alpha", "http://localhost:9888")
            .reconcile_membership(true)
            .await
            .unwrap();
        let doc = read(&path);
        assert_eq!(doc.member("alpha").unwrap().origin, "http://localhost:9888");
        assert_eq!(doc.member("apex").unwrap().origin, "http://localhost:8889");
    }

    #[tokio::test]
    async fn concurrent_legs_do_not_clobber_each_other() {
        let path = scratch("concurrent");
        let alpha = leg(&path, "alpha", "http://localhost:8888");
        let apex = leg(&path, "apex", "http://localhost:8889");
        for _ in 0..25 {
            let (a, b) = tokio::join!(
                alpha.reconcile_membership(true),
                apex.reconcile_membership(true)
            );
            a.unwrap();
            b.unwrap();
            // Every observation of the document is a complete, parseable one.
            let doc = read(&path);
            assert!(doc.members.len() <= 2);
        }
        let doc = read(&path);
        assert_eq!(doc.members.len(), 2);
        assert_eq!(doc.members[0].owner, "alpha");
        assert_eq!(doc.members[1].owner, "apex");
        assert!(!PathBuf::from(format!("{}.lock", path.display())).exists());
    }

    #[tokio::test]
    async fn an_abandoned_lock_is_broken_rather_than_wedging_the_household() {
        let path = scratch("abandoned-lock");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let lock = PathBuf::from(format!("{}.lock", path.display()));
        std::fs::write(&lock, b"9999999").unwrap();
        let stale = SystemTime::now() - (LOCK_STALE + Duration::from_secs(5));
        let stale = std::fs::FileTimes::new().set_modified(stale);
        std::fs::File::options()
            .write(true)
            .open(&lock)
            .unwrap()
            .set_times(stale)
            .unwrap();

        leg(&path, "alpha", "http://localhost:8888")
            .reconcile_membership(true)
            .await
            .unwrap();
        assert_eq!(read(&path).members.len(), 1);
        assert!(!lock.exists());
    }

    #[test]
    fn an_unparseable_stamp_is_treated_as_stale() {
        assert!(age_secs("not-a-time").is_none());
        assert!(age_secs(&now_rfc3339().unwrap()).unwrap() < 5);
    }
}
