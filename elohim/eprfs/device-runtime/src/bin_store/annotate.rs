use super::{files, DerivationKeysAnnotation, Refusal, Release, ReleaseAnnotations, Result, Store};
use std::{collections::BTreeMap, path::Path};

impl Store {
    /// Attach producer-observed derivation keys to an already-held release without ingesting it.
    pub fn annotate_derivation_keys(
        &self,
        release_cid: &str,
        keys: BTreeMap<String, String>,
        expected_head: &str,
        repo: &Path,
    ) -> Result<Release> {
        let not_held = || {
            Refusal::new(
                "release-not-held",
                format!("release {release_cid} is not held in this store"),
            )
        };
        // Refuse an absent release before acquiring the store lock: even the lock file would be
        // a write, and an annotation refusal is required to leave the store byte-for-byte alone.
        self.record(release_cid).ok_or_else(&not_held)?;
        let repo = self.repo_root(Some(repo))?;
        let _lock = files::Lock::acquire(&self.root, files::MUTATION_WAIT)?;
        self.require_migrated()?;
        // Re-read under the mutation lock so a concurrent removal cannot turn annotation into a
        // new release record.
        let mut release = self.record(release_cid).ok_or_else(not_held)?;
        let exact_bins = keys.len() == release.bins.len()
            && release.bins.keys().all(|name| keys.contains_key(name));
        if !exact_bins || keys.values().any(|key| key.trim().is_empty()) {
            return Err(Refusal::new(
                "derivation-keys-invalid",
                "producer derivation keys require one non-empty key for every release binary",
            ));
        }
        let annotation = ReleaseAnnotations {
            derivation_keys: DerivationKeysAnnotation {
                keys,
                covered_by_rcid: false,
                source: "producer".into(),
            },
        };
        if release
            .annotations
            .as_ref()
            .is_some_and(|held| held != &annotation)
        {
            return Err(Refusal::new(
                "derivation-keys-conflict",
                "release is already annotated with different derivation keys",
            ));
        }
        let head = self.git(&repo, &["rev-parse", "HEAD"])?;
        if head != expected_head {
            return Err(Refusal::new(
                "source-head-moved",
                format!("repository HEAD is {head}, not expected {expected_head}"),
            ));
        }
        if release.annotations.is_some() {
            return Ok(release);
        }
        release.annotations = Some(annotation);
        self.save(&release)?;
        Ok(release)
    }
}
