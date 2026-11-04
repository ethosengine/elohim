use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use bytes::Bytes;
use eprfs_core::{
    derive_or_reuse, encode_tree, project_view, walk_spine, AttestationDraft, BatchOptions,
    BlobCid, BlobHandle, BlobPresence, DerivationBasis, DerivationIndex, DerivationKey,
    DerivationLookup, DerivationRecord, DerivationUnanswered, Derive, DeriveOutcome, Derived,
    DomainRoster, EprRecord, EprRef, EprfsError, EprfsStorage, FetchPolicy, KeyCustody,
    MethodDescriptor, RecordWitness, Requester, TreeEntry, TreeEntryKind, TreeError, TreeLimits,
    TreeNode, TreePath, TreeView, TrustPolicy, UnrecordedReason, View, Witness,
};
use seam_contracts::{ComeBack, Meanwhile, Unanswered};

#[derive(Default)]
struct Store {
    blobs: HashMap<BlobCid, Bytes>,
    written: Mutex<HashMap<BlobCid, Bytes>>,
    fetched: Mutex<Vec<BlobCid>>,
}

impl Store {
    fn put_tree(&mut self, entries: Vec<TreeEntry>) -> BlobCid {
        let node = TreeNode::new(entries).unwrap();
        let (cid, bytes) = encode_tree(&node, &TreeLimits::default()).unwrap();
        self.blobs.insert(cid.clone(), Bytes::from(bytes));
        cid
    }

    fn fetched(&self) -> Vec<BlobCid> {
        self.fetched.lock().unwrap().clone()
    }
}

#[async_trait]
impl EprfsStorage for Store {
    async fn resolve_epr(&self, reference: &EprRef) -> eprfs_core::Result<EprRecord> {
        Err(EprfsError::EprNotFound(reference.clone()))
    }

    async fn has_blob(&self, cid: &BlobCid) -> eprfs_core::Result<BlobPresence> {
        Ok(
            if self.blobs.contains_key(cid) || self.written.lock().unwrap().contains_key(cid) {
                BlobPresence::Local
            } else {
                BlobPresence::Missing
            },
        )
    }

    async fn fetch_blob(
        &self,
        cid: &BlobCid,
        _policy: FetchPolicy,
    ) -> eprfs_core::Result<BlobHandle> {
        self.fetched.lock().unwrap().push(cid.clone());
        self.blobs
            .get(cid)
            .cloned()
            .or_else(|| self.written.lock().unwrap().get(cid).cloned())
            .map(|bytes| BlobHandle {
                cid: cid.clone(),
                bytes,
            })
            .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))
    }

    async fn put_blob(&self, bytes: Bytes) -> eprfs_core::Result<BlobCid> {
        let cid = BlobCid::compute_raw(&bytes);
        self.written.lock().unwrap().insert(cid.clone(), bytes);
        Ok(cid)
    }

    async fn publish_attestation(&self, _draft: AttestationDraft) -> eprfs_core::Result<EprRef> {
        unreachable!("the projection tests do not publish attestations")
    }
}

fn file(name: &[u8], content: &[u8]) -> TreeEntry {
    TreeEntry {
        name: name.to_vec(),
        kind: TreeEntryKind::File {
            blob: BlobCid::compute_raw(content).into(),
            executable: false,
        },
    }
}

fn directory(name: &[u8], tree: BlobCid) -> TreeEntry {
    TreeEntry {
        name: name.to_vec(),
        kind: TreeEntryKind::Directory { tree: tree.into() },
    }
}

fn external(name: &[u8]) -> TreeEntry {
    TreeEntry {
        name: name.to_vec(),
        kind: TreeEntryKind::External {
            kind: "git-submodule".into(),
            reference: b"pinned".to_vec(),
        },
    }
}

fn path(value: &str) -> TreePath {
    TreePath::from_slash_bytes(value.as_bytes()).unwrap()
}

fn fixture(outside: &[u8], selected: &[u8]) -> (Store, BlobCid, BlobCid, BlobCid) {
    let mut store = Store::default();
    let lib = store.put_tree(vec![file(b"a.rs", selected), file(b"b.rs", b"b")]);
    let src = store.put_tree(vec![directory(b"lib", lib.clone()), external(b"vendor")]);
    let root = store.put_tree(vec![
        file(b"README", outside),
        directory(b"src", src.clone()),
    ]);
    (store, root, src, lib)
}

#[tokio::test]
async fn projected_root_is_the_sparse_tree_and_reads_only_spine_nodes() {
    let (store, root, src, lib) = fixture(b"outside", b"selected");
    let view = View::from_tree_paths([path("src/lib/a.rs")]);
    let spine = walk_spine(
        &store,
        &root,
        &view,
        &BatchOptions::default(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();

    assert_eq!(store.fetched(), vec![root, src, lib]);
    assert_eq!(spine.nodes.len(), 3);
    let (projected, nodes) = project_view(&spine, &TreeLimits::default()).unwrap();
    assert_eq!(nodes.len(), 3);
    assert!(nodes.iter().any(|(cid, _)| cid == projected.cid()));
    assert_ne!(projected.cid(), &spine.root);
}

#[tokio::test]
async fn projection_ignores_outside_changes_and_preserves_an_in_view_directory_link() {
    let (first, first_root, first_src, _) = fixture(b"outside-a", b"selected");
    let (second, second_root, second_src, _) = fixture(b"outside-b", b"selected");
    let selected = View::from_tree_paths([path("src/lib/a.rs")]);
    let first_spine = walk_spine(
        &first,
        &first_root,
        &selected,
        &BatchOptions::default(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    let second_spine = walk_spine(
        &second,
        &second_root,
        &selected,
        &BatchOptions::default(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        project_view(&first_spine, &TreeLimits::default())
            .unwrap()
            .0,
        project_view(&second_spine, &TreeLimits::default())
            .unwrap()
            .0
    );

    let directory_view = View::from_tree_paths([path("src")]);
    let spine = walk_spine(
        &first,
        &first_root,
        &directory_view,
        &BatchOptions::default(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(first.fetched().last(), Some(&first_root));
    let (_, nodes) = project_view(&spine, &TreeLimits::default()).unwrap();
    let projected_root: TreeNode =
        serde_ipld_dagcbor::from_slice(&nodes.last().unwrap().1).unwrap();
    assert_eq!(projected_root.entries.len(), 1);
    assert!(matches!(
        &projected_root.entries[0].kind,
        TreeEntryKind::Directory { tree } if tree.as_blob_cid() == &first_src
    ));
    assert_eq!(first_src, second_src);
}

#[tokio::test]
async fn not_found_drops_empty_spine_while_through_leaf_and_external_are_kept() {
    let (store, root, _, _) = fixture(b"readme", b"selected");
    let view = View::from_tree_paths([
        path("src/missing/deeper"),
        path("README/deeper"),
        path("src/vendor/deeper"),
    ]);
    let spine = walk_spine(
        &store,
        &root,
        &view,
        &BatchOptions::default(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    let (_, nodes) = project_view(&spine, &TreeLimits::default()).unwrap();
    let projected_root: TreeNode =
        serde_ipld_dagcbor::from_slice(&nodes.last().unwrap().1).unwrap();
    assert_eq!(
        projected_root
            .entries
            .iter()
            .map(|entry| entry.name.as_slice())
            .collect::<Vec<_>>(),
        vec![b"README".as_slice(), b"src".as_slice()]
    );
    let projected_src: TreeNode = serde_ipld_dagcbor::from_slice(&nodes[0].1).unwrap();
    assert_eq!(projected_src.entries.len(), 1);
    assert!(matches!(
        projected_src.entries[0].kind,
        TreeEntryKind::External { .. }
    ));
}

#[tokio::test]
async fn a_missing_spine_node_refuses_to_mint_a_partial_key() {
    let (mut store, root, src, _) = fixture(b"readme", b"selected");
    store.blobs.remove(&src);
    let error = walk_spine(
        &store,
        &root,
        &View::from_tree_paths([path("src/lib/a.rs")]),
        &BatchOptions::default(),
        &TreeLimits::default(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        TreeError::Unanswered { cid, path: at, .. } if cid == src && at == path("src")
    ));
}

#[derive(Default)]
struct Index {
    records: Mutex<Vec<DerivationRecord>>,
    fail_record: bool,
}

impl Index {
    fn with_record(record: DerivationRecord) -> Self {
        Self {
            records: Mutex::new(vec![record]),
            fail_record: false,
        }
    }

    fn records(&self) -> Vec<DerivationRecord> {
        self.records.lock().unwrap().clone()
    }
}

#[async_trait]
impl DerivationIndex for Index {
    async fn lookup(
        &self,
        _who: &Requester,
        at: &TreeView,
        method: &MethodDescriptor,
    ) -> Result<DerivationLookup, Unanswered<DerivationUnanswered>> {
        let method = method.cid().unwrap();
        let found: Vec<_> = self
            .records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| record.key.method == method && record.key.input.cid() == at.root())
            .cloned()
            .collect();
        Ok(if found.is_empty() {
            DerivationLookup::NotRecorded
        } else {
            DerivationLookup::Found(found)
        })
    }

    async fn lookup_many(
        &self,
        who: &Requester,
        at: &TreeView,
        methods: &[MethodDescriptor],
    ) -> Result<BTreeMap<BlobCid, DerivationLookup>, Unanswered<DerivationUnanswered>> {
        let mut found = BTreeMap::new();
        for method in methods {
            found.insert(method.cid().unwrap(), self.lookup(who, at, method).await?);
        }
        Ok(found)
    }

    async fn record(&self, record: &DerivationRecord) -> eprfs_core::Result<()> {
        if self.fail_record {
            return Err(EprfsError::Storage("fixture index write failed".into()));
        }
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }
}

struct Policy;

#[allow(
    clippy::result_large_err,
    reason = "The fixture implements the public TrustPolicy contract by value"
)]
impl TrustPolicy for Policy {
    fn accept<'r>(
        &self,
        key: &DerivationKey,
        records: &'r [DerivationRecord],
        me: &str,
        _roster: &dyn DomainRoster,
    ) -> Result<&'r DerivationRecord, Unanswered<DerivationUnanswered>> {
        records
            .iter()
            .find(|record| {
                record.key == *key
                    && (record.deterministic
                        || matches!(record.basis, DerivationBasis::UnderGrant { .. })
                        || (matches!(record.basis, DerivationBasis::Computed)
                            && record.computed_by == me))
            })
            .ok_or_else(|| {
                Unanswered::new(
                    DerivationUnanswered::Mismatch {
                        outputs: records.iter().map(|record| record.output.clone()).collect(),
                    },
                    ComeBack::OnEvent {
                        event: "independent-record-arrives".into(),
                    },
                    Meanwhile::RunHere,
                )
            })
    }
}

struct Roster;

impl DomainRoster for Roster {
    fn domain_of(&self, agent: &str) -> Option<String> {
        Some(agent.to_owned())
    }
}

struct FixtureWitness {
    agent: String,
    fail: bool,
    seen: Mutex<Vec<Vec<u8>>>,
}

impl FixtureWitness {
    fn new(agent: &str) -> Self {
        Self {
            agent: agent.into(),
            fail: false,
            seen: Mutex::new(Vec::new()),
        }
    }

    fn failing(agent: &str) -> Self {
        Self {
            fail: true,
            ..Self::new(agent)
        }
    }

    fn seen(&self) -> Vec<Vec<u8>> {
        self.seen.lock().unwrap().clone()
    }
}

#[async_trait]
impl RecordWitness for FixtureWitness {
    fn agent(&self) -> &str {
        &self.agent
    }

    async fn witness(&self, body: &[u8]) -> eprfs_core::Result<Witness> {
        self.seen.lock().unwrap().push(body.to_vec());
        if self.fail {
            Err(EprfsError::Storage("fixture witness failed".into()))
        } else {
            Ok(Witness::Signature {
                sig: BlobCid::compute(body).to_string(),
            })
        }
    }
}

struct CountingDerive {
    descriptor: MethodDescriptor,
    output: Bytes,
    calls: AtomicUsize,
}

impl CountingDerive {
    fn new(deterministic: bool) -> Self {
        Self {
            descriptor: MethodDescriptor {
                name: "fixture-derive".into(),
                tools: BTreeMap::from([("fixture".into(), "1".into())]),
                flags: Vec::new(),
                params: None,
                exec: None,
                step: Some("contract".into()),
                embedder: None,
                view: vec![path("src")],
                non_path_inputs: BTreeMap::new(),
                deterministic,
            },
            output: Bytes::from_static(b"same deterministic output"),
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Derive for CountingDerive {
    fn descriptor(&self) -> &MethodDescriptor {
        &self.descriptor
    }

    async fn derive(
        &self,
        _input: &TreeView,
        _input_store: &dyn EprfsStorage,
        output_store: &dyn EprfsStorage,
        _prior: Option<&Derived>,
    ) -> eprfs_core::Result<BlobCid> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        output_store.put_blob(self.output.clone()).await
    }
}

fn requester() -> Requester {
    Requester::authenticated("uhCAkrequester", KeyCustody::DeviceHeld)
}

fn computed(outcome: DeriveOutcome) -> Derived {
    match outcome {
        DeriveOutcome::Derived(derived) => derived,
        other => panic!("expected a derived outcome, got {other:?}"),
    }
}

#[tokio::test]
async fn contract_1_two_stores_compute_the_same_deterministic_output() {
    let method_a = CountingDerive::new(true);
    let method_b = CountingDerive::new(true);
    let index_a = Index::default();
    let index_b = Index::default();
    let witness_a = FixtureWitness::new("uhCAkpeer-a");
    let witness_b = FixtureWitness::new("uhCAkpeer-b");
    let root = BlobCid::compute(b"projected input");

    let first = computed(
        derive_or_reuse(
            &index_a,
            &Policy,
            &method_a,
            &root,
            &Store::default(),
            &requester(),
            &witness_a,
            &Roster,
            &[],
            true,
        )
        .await
        .unwrap(),
    );
    let second = computed(
        derive_or_reuse(
            &index_b,
            &Policy,
            &method_b,
            &root,
            &Store::default(),
            &requester(),
            &witness_b,
            &Roster,
            &[],
            true,
        )
        .await
        .unwrap(),
    );

    assert_eq!(first.output, second.output);
    assert_eq!(method_a.calls(), 1);
    assert_eq!(method_b.calls(), 1);
    assert!(!first.reused && !second.reused);
}

#[tokio::test]
async fn contract_3_accepted_fetchable_record_reuses_with_zero_computes_on_the_hit() {
    let method = CountingDerive::new(true);
    let index = Index::default();
    let store = Store::default();
    let witness = FixtureWitness::new("uhCAkpeer");
    let root = BlobCid::compute(b"projected input");
    let first = computed(
        derive_or_reuse(
            &index,
            &Policy,
            &method,
            &root,
            &store,
            &requester(),
            &witness,
            &Roster,
            &[],
            true,
        )
        .await
        .unwrap(),
    );
    assert_eq!(method.calls(), 1);

    let hit = computed(
        derive_or_reuse(
            &index,
            &Policy,
            &method,
            &root,
            &store,
            &requester(),
            &witness,
            &Roster,
            &[],
            true,
        )
        .await
        .unwrap(),
    );
    assert_eq!(hit.output, first.output);
    assert!(hit.reused);
    assert_eq!(method.calls(), 1, "the accepted hit must not run Derive");
    assert_eq!(index.records().len(), 1, "the hit must not record again");
}

#[tokio::test]
async fn contract_3_unfetchable_record_is_an_output_not_fetchable_miss() {
    let producing_method = CountingDerive::new(true);
    let producing_index = Index::default();
    let producing_store = Store::default();
    let witness = FixtureWitness::new("uhCAkpeer");
    let root = BlobCid::compute(b"projected input");
    let produced = computed(
        derive_or_reuse(
            &producing_index,
            &Policy,
            &producing_method,
            &root,
            &producing_store,
            &requester(),
            &witness,
            &Roster,
            &[],
            true,
        )
        .await
        .unwrap(),
    );
    let index = Index::with_record(produced.record);
    let method = CountingDerive::new(true);

    let outcome = derive_or_reuse(
        &index,
        &Policy,
        &method,
        &root,
        &Store::default(),
        &requester(),
        &witness,
        &Roster,
        &[],
        false,
    )
    .await
    .unwrap();
    let DeriveOutcome::NotDerived(unanswered) = outcome else {
        panic!("an unfetchable record without capacity must stay not derived")
    };
    assert_eq!(unanswered.why, DerivationUnanswered::OutputNotFetchable);
    assert!(matches!(unanswered.come_back, ComeBack::OnEvent { .. }));
    assert!(matches!(unanswered.meanwhile, Meanwhile::RunHere));
    assert_eq!(method.calls(), 0);
}

#[tokio::test]
async fn contract_3_miss_without_capacity_is_not_carried_with_all_three_parts() {
    let method = CountingDerive::new(true);
    let outcome = derive_or_reuse(
        &Index::default(),
        &Policy,
        &method,
        &BlobCid::compute(b"projected input"),
        &Store::default(),
        &requester(),
        &FixtureWitness::new("uhCAkpeer"),
        &Roster,
        &[],
        false,
    )
    .await
    .unwrap();
    let DeriveOutcome::NotDerived(unanswered) = outcome else {
        panic!("a capacity refusal must stay not derived")
    };
    assert_eq!(unanswered.why, DerivationUnanswered::NotCarried);
    assert!(matches!(unanswered.come_back, ComeBack::OnEvent { .. }));
    assert!(matches!(unanswered.meanwhile, Meanwhile::Nothing { .. }));
    assert_eq!(method.calls(), 0);
}

#[tokio::test]
async fn contract_4_nondeterministic_records_reuse_only_when_the_policy_accepts_them() {
    let method = CountingDerive::new(false);
    let index = Index::default();
    let store = Store::default();
    let mine = FixtureWitness::new("uhCAkme");
    let root = BlobCid::compute(b"projected input");
    let first = computed(
        derive_or_reuse(
            &index,
            &Policy,
            &method,
            &root,
            &store,
            &requester(),
            &mine,
            &Roster,
            &[],
            true,
        )
        .await
        .unwrap(),
    );
    let own_hit = computed(
        derive_or_reuse(
            &index,
            &Policy,
            &method,
            &root,
            &store,
            &requester(),
            &mine,
            &Roster,
            &[],
            false,
        )
        .await
        .unwrap(),
    );
    assert!(own_hit.reused);

    let mut foreign = first.record;
    foreign.computed_by = "uhCAkother".into();
    let foreign_index = Index::with_record(foreign);
    let refused = derive_or_reuse(
        &foreign_index,
        &Policy,
        &method,
        &root,
        &store,
        &requester(),
        &mine,
        &Roster,
        &[],
        false,
    )
    .await
    .unwrap();
    assert!(matches!(
        refused,
        DeriveOutcome::NotDerived(Unanswered {
            why: DerivationUnanswered::Mismatch { .. },
            ..
        })
    ));
    assert_eq!(method.calls(), 1);
}

#[tokio::test]
async fn r2_witness_failure_returns_the_computed_output_as_typed_unrecorded() {
    let method = CountingDerive::new(true);
    let index = Index::default();
    let store = Store::default();
    let output = derive_or_reuse(
        &index,
        &Policy,
        &method,
        &BlobCid::compute(b"projected input"),
        &store,
        &requester(),
        &FixtureWitness::failing("uhCAkpeer"),
        &Roster,
        &[],
        true,
    )
    .await
    .unwrap();
    let DeriveOutcome::ComputedUnrecorded(unrecorded) = output else {
        panic!("a witness failure must return a typed unrecorded outcome")
    };
    assert!(unrecorded.record.is_none());
    assert!(matches!(
        unrecorded.reason,
        UnrecordedReason::WitnessFailed { .. }
    ));
    assert!(matches!(
        store.has_blob(&unrecorded.output).await.unwrap(),
        BlobPresence::Local
    ));
    assert!(index.records().is_empty());
}

#[tokio::test]
async fn r2_index_failure_returns_the_witnessed_record_as_typed_unrecorded() {
    let method = CountingDerive::new(true);
    let index = Index {
        fail_record: true,
        ..Index::default()
    };
    let output = derive_or_reuse(
        &index,
        &Policy,
        &method,
        &BlobCid::compute(b"projected input"),
        &Store::default(),
        &requester(),
        &FixtureWitness::new("uhCAkpeer"),
        &Roster,
        &[],
        true,
    )
    .await
    .unwrap();
    let DeriveOutcome::ComputedUnrecorded(unrecorded) = output else {
        panic!("an index failure must return a typed unrecorded outcome")
    };
    assert!(unrecorded.record.is_some());
    assert!(matches!(
        unrecorded.reason,
        UnrecordedReason::IndexFailed { .. }
    ));
}

#[tokio::test]
async fn r2_witness_sees_exactly_the_body_retained_by_d7_decoding() {
    let method = CountingDerive::new(true);
    let index = Index::default();
    let witness = FixtureWitness::new("uhCAkpeer");
    let derived = computed(
        derive_or_reuse(
            &index,
            &Policy,
            &method,
            &BlobCid::compute(b"projected input"),
            &Store::default(),
            &requester(),
            &witness,
            &Roster,
            &[],
            true,
        )
        .await
        .unwrap(),
    );
    let seen = witness.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(derived.record.computed_by, witness.agent());
    let retained =
        DerivationRecord::decode_known_epoch(&seen[0], derived.record.witness.clone()).unwrap();
    assert_eq!(retained.body(), seen[0]);
    assert_eq!(retained.record(), &derived.record);
}
