//! `ActorClaim` — who is acting, claimed by the actor itself, in flight.
//!
//! WHY THIS IS A RECORD AND NOT A FIELD. Attribution written *about* an actor, *after* the
//! fact, by whoever happened to be dispatching, is the degenerate form: it cannot be disputed
//! by the party it names, it cannot be revised mid-run, and it is only ever as good as the
//! dispatcher's memory. A claim inverts all three. The actor registers it for itself while the
//! work is happening, later claims supersede earlier ones without rewriting anything, and the
//! whole history stays readable — including the claim that turned out to be wrong.
//!
//! **Honor-system by construction, and that is the honest ceiling.** Nothing here proves the
//! claimed identity; a claim is a signature on a statement, not a credential. What it buys is
//! that the statement now EXISTS as an addressable record, so a later attestation has something
//! precise to agree or disagree with. `definition_cid` narrows it one honest notch further: it
//! is the content address of the agent's own package — the BUILD, never the instance. Two runs
//! of the same persona share a `definition_cid` and are not thereby the same actor.
//!
//! **Claims stack; latest wins; history is never rewritten.** [`ActorStore::current_for`] reads
//! the LAST claim appended for a session, which makes a persona switch mid-session a one-line
//! append rather than a mutation. There is no delete and no update — the store is the same
//! append-only JSONL discipline as [`crate::store::SidecarFlowStore`], for the same reason: the
//! superseded claim is evidence, not garbage.
//!
//! **This module never reads a clock and never reads git.** `claimed_at` is supplied by the
//! caller, exactly like every other timestamp in this crate, so that two peers folding the same
//! records mint the same CIDs with no shared clock. A claim is dated by the tree it was made
//! against; deciding what that means is the caller's job, and refusing when there is no such
//! tree is the caller's refusal to make.

#[cfg(feature = "sidecar")]
use std::path::{Path, PathBuf};

use cid::Cid;
use serde::{Deserialize, Serialize};

use crate::error::{FabricError, Result};
use crate::model::{atom_cid, AgentRef};

/// The prefix of an AI-agent claim. Held as a constant because [`parse_agent_ref`] is the
/// single source of truth for that shape and every other reader — including the CLI that
/// derives a package path from the role — goes through it.
const AGENT_PREFIX: &str = "agent:";
/// The prefix of a human participant's claim, `human:<handle>`. The handle is a slug the human
/// chose for themselves — never derived from git, never an email, never minted by the substrate
/// on their behalf. Both prefixes fill the same [`AgentRef`] slot: on the household mesh every
/// participant is an `AgentPubKey` with a source chain, human or not, and these two forms are
/// the repository node's rehearsal of that until the mesh mints the key.
const HUMAN_PREFIX: &str = "human:";
/// A model segment no AI agent may claim. `agent:<person>@human` is the forgery the human form
/// exists to make unnecessary: a substrate asserting a persona nobody claimed. Refused in
/// [`parse_agent_ref`] itself, so every caller that validates the agent shape refuses it too.
const RESERVED_MODEL: &str = "human";

/// The declared content-address scheme for [`ActorClaim::definition_cid`].
const DEFINITION_CID_PREFIX: &str = "sha256:";

/// A registered, session-scoped identity claim.
///
/// Identity is the dag-cbor atom CID of these four fields, which is what makes a re-claim
/// idempotent: the same actor, in the same session, against the same tree, with the same
/// package build, mints the same address. Two claims differing only in `definition_cid` are
/// two records on purpose — "the same persona, rebuilt" is a real change to attribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorClaim {
    /// The claimed identity, `agent:<role>@<model>`. Validated by [`parse_agent_ref`]; the
    /// [`AgentRef`] newtype is the crate's agent slot, so a claim drops into the same
    /// `provider`/`raised_by` positions every other record uses.
    pub claimed: AgentRef,
    /// The run this claim scopes. Claims are per-session because an actor's identity is a
    /// property of a run, not of a repository: the same host runs many personas.
    pub session: String,
    /// When the claim was made, in the caller's supplied encoding (RFC3339 in practice).
    /// Never read from a clock here — see the module doc.
    pub claimed_at: String,
    /// Content address of the agent package this claim was made from, `sha256:<64 hex>`.
    ///
    /// `None` is **honest absence**, and the `skip_serializing_if` is what makes it honest:
    /// a claim made where no package could be read must encode byte-identically to one made
    /// before this field existed, or every claim is re-addressed by a field nobody set. Same
    /// additive discipline as [`crate::model::Bound::sense`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_cid: Option<String>,
}

impl ActorClaim {
    /// The sole constructor. Every refusal below is a shape that would read as evidence and is
    /// not: an unparseable identity, an unscoped claim, an undated claim, or a definition
    /// address that is not an address.
    ///
    /// The struct's fields are `pub` (the crate's convention — serde and callers both need
    /// them), so a hand-built literal can still bypass this. That is the same trade
    /// [`crate::model::DepEdge`] makes: the constructor is the gate every real caller goes
    /// through, and the read path re-verifies the CID rather than the semantics.
    pub fn new(
        claimed: &str,
        session: &str,
        claimed_at: &str,
        definition_cid: Option<String>,
    ) -> Result<Self> {
        // Shape first: a claim whose identity does not parse can never be attributed to
        // anything, so nothing else about it is worth checking.
        let participant = parse_participant_ref(claimed)?;

        // A human has no build. A definition address on a human claim is not honest narrowing,
        // it is a category error that would read as a real package address and compare unequal
        // to every genuine one — refused, not dropped, so the caller learns what it asserted.
        if matches!(participant, ParticipantRef::Human { .. }) && definition_cid.is_some() {
            return Err(FabricError::Decode(format!(
                "actor claim `{claimed}` is a human participant and carries a definition_cid — \
                 a human has no package build to address; omit it"
            )));
        }

        let session = session.trim();
        if session.is_empty() {
            return Err(FabricError::Decode(
                "actor claim needs a session — an unscoped claim cannot be superseded, because \
                 there is no run whose latest claim it could be"
                    .into(),
            ));
        }

        let claimed_at = claimed_at.trim();
        if claimed_at.is_empty() {
            return Err(FabricError::Decode(
                "actor claim needs a claimed_at — an undated claim is unorderable against its \
                 own supersessions"
                    .into(),
            ));
        }

        if let Some(cid) = definition_cid.as_deref() {
            validate_definition_cid(cid)?;
        }

        Ok(Self {
            claimed: AgentRef(claimed.to_string()),
            session: session.to_string(),
            claimed_at: claimed_at.to_string(),
            definition_cid,
        })
    }

    /// The `<role>` half of the claimed identity — the segment that names WHICH agent
    /// definition this is, and therefore the segment a caller resolves a package path from.
    /// Re-parses rather than caching: the parse is the single source of truth for the shape,
    /// and a cached copy is one more thing that can disagree with the hashed bytes.
    pub fn role(&self) -> Result<String> {
        Ok(parse_agent_ref(&self.claimed.0)?.0)
    }

    /// The `<model>` half — which build of the runtime made the claim.
    pub fn model(&self) -> Result<String> {
        Ok(parse_agent_ref(&self.claimed.0)?.1)
    }

    /// Which kind of participant claimed, with its parsed halves. Re-parses for the same
    /// reason [`Self::role`] does: the parse is the single source of truth for the shape.
    pub fn participant(&self) -> Result<ParticipantRef> {
        parse_participant_ref(&self.claimed.0)
    }
}

/// A claimed identity, parsed. Two kinds fill the one [`AgentRef`] slot.
///
/// The distinction is load-bearing for attribution and for nothing else: an AI agent's produce
/// is rent on a commons it did not make and flows to the pool that constituted the persona,
/// while a human's produce is their labor. The substrate must be able to tell them apart without
/// either one being able to pass as the other — which is why the human form has no `@` half
/// and the agent form refuses `human` as a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipantRef {
    /// `agent:<role>@<model>` — an AI agent, identified by its definition and its runtime build.
    Agent { role: String, model: String },
    /// `human:<handle>` — a human participant, identified by the handle they claimed.
    Human { handle: String },
}

/// Parse either participant form, refusing anything else.
///
/// ONE source of truth for the two shapes together. [`ActorClaim::new`] validates through
/// this; callers that accept only the agent form keep using [`parse_agent_ref`], and callers
/// that accept any participant (a note's `--as`, a contribution's `author`) use this one.
pub fn parse_participant_ref(claimed: &str) -> Result<ParticipantRef> {
    if let Some(handle) = claimed.strip_prefix(HUMAN_PREFIX) {
        if handle.contains('@') {
            return Err(malformed(
                claimed,
                "a human handle carries no `@` — it is a slug the human chose, never an email \
                 and never a role@model pair",
            ));
        }
        if handle.is_empty() || !handle.chars().all(is_role_char) {
            return Err(malformed(
                claimed,
                "the handle must be a non-empty `[a-z0-9-]+`",
            ));
        }
        return Ok(ParticipantRef::Human {
            handle: handle.to_string(),
        });
    }
    let (role, model) = parse_agent_ref(claimed)?;
    Ok(ParticipantRef::Agent { role, model })
}

/// Split `agent:<role>@<model>` into its halves, refusing anything else.
///
/// ONE source of truth for the shape, used by [`ActorClaim::new`] to validate and by callers
/// to derive a package path from the role. A second parser is how "the CLI accepted it but the
/// store refused it" happens — or worse, the reverse.
///
/// The charsets are deliberately narrow and lowercase-only: these strings become filesystem
/// path segments and log keys, and a role that differs from another only by case is an
/// impersonation surface for free. `.` is admitted in the model half alone, because model
/// builds carry dotted versions (`opus-5.1`) while roles never do.
pub fn parse_agent_ref(claimed: &str) -> Result<(String, String)> {
    let Some(rest) = claimed.strip_prefix(AGENT_PREFIX) else {
        return Err(malformed(claimed, "it does not begin with `agent:`"));
    };
    let Some((role, model)) = rest.split_once('@') else {
        return Err(malformed(
            claimed,
            "it carries no `@` separating the role from the model",
        ));
    };
    if model.contains('@') {
        return Err(malformed(claimed, "it carries more than one `@`"));
    }
    if role.is_empty() || !role.chars().all(is_role_char) {
        return Err(malformed(
            claimed,
            "the role segment must be a non-empty `[a-z0-9-]+`",
        ));
    }
    if model.is_empty() || !model.chars().all(is_model_char) {
        return Err(malformed(
            claimed,
            "the model segment must be a non-empty `[a-z0-9.-]+`",
        ));
    }
    if model == RESERVED_MODEL {
        return Err(malformed(
            claimed,
            "`human` is not a model — a human participant claims as `human:<handle>`, and the \
             substrate never mints an agent persona for a person",
        ));
    }
    Ok((role.to_string(), model.to_string()))
}

fn is_role_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
}

fn is_model_char(c: char) -> bool {
    is_role_char(c) || c == '.'
}

fn malformed(claimed: &str, why: &str) -> FabricError {
    FabricError::Decode(format!(
        "actor claim `{claimed}` is neither `agent:<role>@<model>` nor `human:<handle>`: {why}"
    ))
}

/// A definition address must be a full sha2-256 digest in lowercase hex — never a truncated
/// fingerprint and never mixed case. A short or upper-cased address still *looks* like an
/// address, which is exactly why it is refused here instead of stored and later compared
/// against a real one that will never match it.
fn validate_definition_cid(value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix(DEFINITION_CID_PREFIX) else {
        return Err(FabricError::Decode(format!(
            "definition cid `{value}` must begin with `{DEFINITION_CID_PREFIX}`"
        )));
    };
    if hex.len() != 64
        || !hex
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
    {
        return Err(FabricError::Decode(format!(
            "definition cid `{value}` must be `{DEFINITION_CID_PREFIX}` followed by 64 \
             lowercase hex characters"
        )));
    }
    Ok(())
}

/// A present agent witnessing a human participant — `human:<handle>` attested by
/// `agent:<role>@<model>`, never self-declared by the substrate.
///
/// **The elohim witness the human.** A human's standing identity at the repository node is
/// established by the act of an agent present with them, who names what it actually knows in
/// `basis` and takes responsibility for it — the DHT's humanity-witness pattern (humanity is
/// attested by others) taken up by the agents working here. It is never inferred from git, an
/// email or a workspace namespace, and it is not a credential: a witness is contestable by an
/// adverse attestation (a roster `Contest` row), and the human may always claim for themselves
/// beside it.
///
/// Hashed as its own atom, exactly like [`ActorClaim`]: the same witness by the same agent in the
/// same session at the same time with the same basis mints the same address, and a re-witness
/// (any field different) is a new record rather than a mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorWitness {
    /// Who is witnessed: `human:<handle>`. Only a human is witnessed; an agent claims for itself.
    pub subject: AgentRef,
    /// Who witnesses: `agent:<role>@<model>`. A human's own act is a claim, not a witness.
    pub witness: AgentRef,
    /// The witnessing agent's session.
    pub session: String,
    /// One line the witness stands behind — what it actually knows of who is present.
    pub basis: String,
    /// When, in the caller's supplied encoding. Never read from a clock here.
    pub claimed_at: String,
}

impl ActorWitness {
    /// The sole constructor. Refuses a subject that is not a human, a witness that is not an
    /// agent, an unscoped or undated witness, and — the load-bearing one — an empty basis: a
    /// witness that names nothing it knows is an assertion nobody stands behind.
    pub fn new(
        subject: &str,
        witness: &str,
        session: &str,
        basis: &str,
        claimed_at: &str,
    ) -> Result<Self> {
        if !matches!(
            parse_participant_ref(subject)?,
            ParticipantRef::Human { .. }
        ) {
            return Err(FabricError::Decode(format!(
                "witness subject `{subject}` is not a human participant — only `human:<handle>` \
                 is witnessed; an agent claims for itself"
            )));
        }
        // The agent parser refuses `human:` outright, so a human can never pass as a witness.
        parse_agent_ref(witness).map_err(|e| {
            FabricError::Decode(format!(
                "witness `{witness}` is not an agent — a human's own act is a claim, not a \
                 witness: {e}"
            ))
        })?;
        let session = session.trim();
        if session.is_empty() {
            return Err(FabricError::Decode(
                "a witness needs the witnessing agent's session".into(),
            ));
        }
        let basis = basis.trim();
        if basis.is_empty() {
            return Err(FabricError::Decode(
                "a witness needs a basis — one line the witness stands behind; a witness that \
                 names nothing it knows is an assertion nobody is responsible for"
                    .into(),
            ));
        }
        let claimed_at = claimed_at.trim();
        if claimed_at.is_empty() {
            return Err(FabricError::Decode("a witness needs a claimed_at".into()));
        }
        Ok(Self {
            subject: AgentRef(subject.to_string()),
            witness: AgentRef(witness.to_string()),
            session: session.to_string(),
            basis: basis.to_string(),
            claimed_at: claimed_at.to_string(),
        })
    }

    /// The witnessed handle (the `<handle>` of `human:<handle>`).
    pub fn handle(&self) -> Result<String> {
        match parse_participant_ref(&self.subject.0)? {
            ParticipantRef::Human { handle } => Ok(handle),
            ParticipantRef::Agent { .. } => Err(FabricError::Decode(format!(
                "witness subject `{}` is not a human participant",
                self.subject.0
            ))),
        }
    }
}

/// The domain tag every actor-record signature covers, ahead of the signed record's CID. A
/// signature over a bare CID string could be replayed as a signature over anything else that
/// signs the same string; the tag makes it mean exactly "this device signed this actor record".
const RECORD_SIGNING_DOMAIN: &str = "elohim:actor-record-signature:v1:";

/// The exact bytes a device signs for an actor record: the domain tag followed by the record's
/// CID in its canonical string form.
pub fn record_signing_message(record_cid: &str) -> Vec<u8> {
    format!("{RECORD_SIGNING_DOMAIN}{record_cid}").into_bytes()
}

/// Checks a detached ed25519 signature against a signer named by its `did:key`.
///
/// A seam, not an implementation: this crate is a pure model and never decodes a did:key or
/// holds a key. The caller supplies verification — `epr-cli`'s device-key module in practice —
/// and every closure `Fn(&str, &[u8], &[u8]) -> bool` already is one.
pub trait SignatureVerifier {
    /// `true` iff `signature` is `signer_did_key`'s valid signature over `message`.
    fn verify(&self, signer_did_key: &str, message: &[u8], signature: &[u8]) -> bool;
}

impl<F> SignatureVerifier for F
where
    F: Fn(&str, &[u8], &[u8]) -> bool,
{
    fn verify(&self, signer_did_key: &str, message: &[u8], signature: &[u8]) -> bool {
        self(signer_did_key, message, signature)
    }
}

/// A device's signature over one actor record — an [`ActorClaim`] or an [`ActorWitness`] — named
/// by the record's CID.
///
/// **A second record, never a field.** The signature cannot sit inside the atom it signs, and
/// adding a field to [`ActorClaim`] would re-address every claim ever written; so the signature is
/// its own appended record pointing at the signed one. An unsigned claim stays exactly as valid as
/// it always was — nothing anywhere refuses an act for want of this record. What it adds is that a
/// later reader can check which device made the statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordSignature {
    /// The CID of the signed record (a claim or a witness), in its canonical string form.
    pub claim_cid: String,
    /// The signing device, as a `did:key` — public material only.
    pub signer: String,
    /// The 64-byte ed25519 signature over [`record_signing_message`], lowercase hex.
    pub signature: String,
}

impl RecordSignature {
    /// Build from a record CID, the signer's did:key and the raw signature bytes. Refuses a
    /// signer that is not did:key-shaped and a signature that is not 64 bytes; it does NOT
    /// verify — verification needs a key this crate never holds (see [`SignatureVerifier`]).
    pub fn new(claim_cid: &Cid, signer: &str, signature: &[u8]) -> Result<Self> {
        validate_did_key_shape(signer)?;
        if signature.len() != ED25519_SIGNATURE_LEN {
            return Err(FabricError::Decode(format!(
                "an ed25519 signature is {ED25519_SIGNATURE_LEN} bytes, got {}",
                signature.len()
            )));
        }
        Ok(Self {
            claim_cid: claim_cid.to_string(),
            signer: signer.to_string(),
            signature: hex_encode(signature),
        })
    }

    /// `true` iff the signature verifies, under `verifier`, as `signer`'s over the signed CID.
    /// A malformed hex signature is simply `false`.
    pub fn verify(&self, verifier: &dyn SignatureVerifier) -> bool {
        match hex_decode(&self.signature) {
            Some(bytes) => verifier.verify(
                &self.signer,
                &record_signing_message(&self.claim_cid),
                &bytes,
            ),
            None => false,
        }
    }
}

/// Raw ed25519 signature length.
pub(crate) const ED25519_SIGNATURE_LEN: usize = 64;

/// A did:key carries an ed25519 key here and nothing else: `did:key:z` + base58btc. The shape is
/// checked (this crate holds no base58 codec); whether it decodes is the verifier's question.
pub(crate) fn validate_did_key_shape(value: &str) -> Result<()> {
    const BASE58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    match value.strip_prefix("did:key:z") {
        Some(rest) if !rest.is_empty() && rest.chars().all(|c| BASE58.contains(c)) => Ok(()),
        _ => Err(FabricError::Decode(format!(
            "`{value}` is not a did:key (`did:key:z<base58btc>`)"
        ))),
    }
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Lowercase hex only; anything else (odd length, upper case, non-hex) is `None`.
pub(crate) fn hex_decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2)
        || !text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
    {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

/// One appended actor record. Append-only; supersessions are new records.
///
/// A tagged envelope, so that a *different act* about the same statement lands without breaking
/// the line format every already-written sidecar uses: a claim (the actor's own statement), a
/// signature over a record (a device standing behind it) and a witness (a present agent attesting
/// a human). The envelope is a storage detail and — exactly as in [`crate::store::FlowRecord`] —
/// never participates in identity: each record's CID is its payload's, so every claim written
/// before the second and third kinds existed keeps its address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActorRecord {
    Claim(ActorClaim),
    Signed(RecordSignature),
    Witness(ActorWitness),
}

impl ActorRecord {
    /// The record's identity: the atom CID of the PAYLOAD.
    pub fn cid(&self) -> Result<Cid> {
        match self {
            ActorRecord::Claim(claim) => atom_cid(claim),
            ActorRecord::Signed(signature) => atom_cid(signature),
            ActorRecord::Witness(witness) => atom_cid(witness),
        }
    }
}

pub trait ActorStore {
    /// Append a record; returns the record's atom CID.
    fn append(&mut self, record: ActorRecord) -> Result<Cid>;

    /// All records in append order, with their CIDs.
    fn records(&self) -> Result<Vec<(Cid, ActorRecord)>>;

    /// Every claim in append order. An exhaustive `match` rather than an `if let`, so that a
    /// second record kind cannot slip through as a claim: adding one turns this into a compile
    /// error at exactly the place that has to decide what the new kind means to this reader.
    ///
    /// A signature and a witness are not claims: a signature is a device standing behind a
    /// record, and a witness is one participant's statement about another. Neither says who is
    /// acting in a session, so neither can answer [`Self::current_for`].
    fn claims(&self) -> Result<Vec<(Cid, ActorClaim)>> {
        Ok(self
            .records()?
            .into_iter()
            .filter_map(|(cid, record)| match record {
                ActorRecord::Claim(claim) => Some((cid, claim)),
                ActorRecord::Signed(_) | ActorRecord::Witness(_) => None,
            })
            .collect())
    }

    /// Who is acting in `session` right now: the LAST claim appended for it.
    ///
    /// Append order is the whole ordering rule, and it is the right one even though
    /// `claimed_at` exists: claims made within one session against one tree share a timestamp
    /// by construction (the caller dates them from history, not a clock), so a timestamp sort
    /// could not order a mid-session persona switch at all. Order-of-arrival can.
    ///
    /// `None` is honest absence — the session registered nothing. It is never an error, because
    /// "nobody claimed" is a normal, common answer that callers must be able to act on.
    fn current_for(&self, session: &str) -> Result<Option<(Cid, ActorClaim)>> {
        Ok(self
            .claims()?
            .into_iter()
            .rfind(|(_, claim)| claim.session == session))
    }
}

/// In-memory store — tests and short-lived reads.
#[derive(Debug, Default)]
pub struct MemoryActorStore {
    records: Vec<(Cid, ActorRecord)>,
}

impl MemoryActorStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ActorStore for MemoryActorStore {
    fn append(&mut self, record: ActorRecord) -> Result<Cid> {
        let cid = record.cid()?;
        self.records.push((cid, record));
        Ok(cid)
    }

    fn records(&self) -> Result<Vec<(Cid, ActorRecord)>> {
        Ok(self.records.clone())
    }
}

/// The offline floor: an append-only JSONL log under `<root>/.eprfs/status/actors.jsonl`.
///
/// A SEPARATE log from `flows.jsonl` on purpose. Identity claims and value-chain records have
/// different write rates, different readers, and — the load-bearing one — different failure
/// consequences: a decision authority that consults the identity sidecar must be able to fail
/// to read it and still decide. Interleaving the two would make one unreadable line cost both.
///
/// Each line carries the record and its dag-cbor atom CID; CIDs are re-verified on read (a
/// tampered line is an integrity error, not silent drift).
///
/// Behind the default-on `sidecar` feature — see [`crate::store::SidecarFlowStore`].
#[cfg(feature = "sidecar")]
#[derive(Debug)]
pub struct SidecarActorStore {
    log_path: PathBuf,
}

#[cfg(feature = "sidecar")]
impl SidecarActorStore {
    /// Open (creating directories/log as needed) the sidecar under `root`.
    pub fn open(root: &Path) -> Result<Self> {
        let log_path = crate::sidecar::open_log(root, "actors.jsonl")?;
        Ok(Self { log_path })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Serialize current-claim/check/append using this returned handle only. The OS lock
    /// releases when it drops, including process death. Each append is synced; a transaction
    /// is not a rollback boundary. Never independently reopen the log inside this scope.
    pub fn transaction(&self) -> Result<SidecarActorTransaction> {
        Ok(SidecarActorTransaction {
            log: crate::sidecar::LockedLog::exclusive(&self.log_path)?,
        })
    }
}

/// A process-exclusive view of the existing actor store, implementing the same store seam.
#[cfg(feature = "sidecar")]
pub struct SidecarActorTransaction {
    log: crate::sidecar::LockedLog,
}

#[cfg(feature = "sidecar")]
#[derive(Serialize, Deserialize)]
struct SidecarLine {
    cid: String,
    record: ActorRecord,
}

#[cfg(feature = "sidecar")]
impl ActorStore for SidecarActorStore {
    fn append(&mut self, record: ActorRecord) -> Result<Cid> {
        self.transaction()?.append(record)
    }

    fn records(&self) -> Result<Vec<(Cid, ActorRecord)>> {
        SidecarActorTransaction {
            // Attribution cannot stall a governance decision behind a suspended writer.
            // Busy is an I/O error, never a coherent-looking empty actor history.
            log: crate::sidecar::LockedLog::try_shared(&self.log_path)?,
        }
        .records()
    }
}

#[cfg(feature = "sidecar")]
impl ActorStore for SidecarActorTransaction {
    fn append(&mut self, record: ActorRecord) -> Result<Cid> {
        if !self.log.validated() {
            self.records()?;
        }
        let cid = record.cid()?;
        let line = serde_json::to_string(&SidecarLine {
            cid: cid.to_string(),
            record,
        })
        .map_err(|e| FabricError::Encode(e.to_string()))?;
        self.log.append(line)?;
        Ok(cid)
    }

    fn records(&self) -> Result<Vec<(Cid, ActorRecord)>> {
        let contents = self.log.contents()?;
        let mut records = Vec::new();
        for line in contents.lines().filter(|l| !l.trim().is_empty()) {
            let parsed: SidecarLine =
                serde_json::from_str(line).map_err(|e| FabricError::Decode(e.to_string()))?;
            let computed = parsed.record.cid()?;
            if computed.to_string() != parsed.cid {
                return Err(FabricError::Integrity {
                    stored: parsed.cid,
                    computed: computed.to_string(),
                });
            }
            records.push((computed, parsed.record));
        }
        self.log.mark_validated();
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "sidecar")]
    use std::fs;

    const AT: &str = "2026-08-15T00:00:00Z";
    /// A well-formed definition address: 64 lowercase hex characters.
    const DEF: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn claim(claimed: &str, session: &str) -> ActorClaim {
        ActorClaim::new(claimed, session, AT, None).expect("valid claim")
    }

    // ── constructor refusals ────────────────────────────────────────────────────────────

    #[test]
    fn a_claimed_identity_without_the_agent_prefix_is_refused() {
        let err = ActorClaim::new("scribe@opus-5", "s1", AT, None)
            .expect_err("a bare role@model is not a claimed identity");
        assert!(err.to_string().contains("agent:"), "got: {err}");
    }

    #[test]
    fn a_claimed_identity_without_an_at_sign_is_refused() {
        let err =
            ActorClaim::new("agent:scribe", "s1", AT, None).expect_err("no model half is refused");
        assert!(err.to_string().contains('@'), "got: {err}");
    }

    #[test]
    fn a_claimed_identity_with_two_at_signs_is_refused() {
        assert!(ActorClaim::new("agent:scribe@opus@5", "s1", AT, None).is_err());
    }

    #[test]
    fn an_empty_role_or_model_half_is_refused() {
        assert!(ActorClaim::new("agent:@opus-5", "s1", AT, None).is_err());
        assert!(ActorClaim::new("agent:scribe@", "s1", AT, None).is_err());
        assert!(ActorClaim::new("agent:@", "s1", AT, None).is_err());
    }

    #[test]
    fn an_uppercase_or_punctuated_role_is_refused() {
        // Case-only variants of a real role are an impersonation surface, so the charset is
        // lowercase-only rather than case-folded.
        assert!(ActorClaim::new("agent:Scribe@opus-5", "s1", AT, None).is_err());
        assert!(ActorClaim::new("agent:scribe/../x@opus-5", "s1", AT, None).is_err());
        assert!(ActorClaim::new("agent:scribe_x@opus-5", "s1", AT, None).is_err());
        // A dot belongs to the model half alone.
        assert!(ActorClaim::new("agent:scr.ibe@opus-5", "s1", AT, None).is_err());
        assert!(ActorClaim::new("agent:scribe@opus-5.1", "s1", AT, None).is_ok());
    }

    #[test]
    fn an_uppercase_model_is_refused() {
        assert!(ActorClaim::new("agent:scribe@Opus-5", "s1", AT, None).is_err());
    }

    // ── participant_ref: the human form beside the agent form ──────────────────────────

    #[test]
    fn participant_ref_parses_both_kinds() {
        assert_eq!(
            parse_participant_ref("human:matthew").unwrap(),
            ParticipantRef::Human {
                handle: "matthew".into()
            }
        );
        assert_eq!(
            parse_participant_ref("agent:scribe@opus-5").unwrap(),
            ParticipantRef::Agent {
                role: "scribe".into(),
                model: "opus-5".into()
            }
        );
        let c = claim("human:matthew", "s1");
        assert_eq!(
            c.participant().unwrap(),
            ParticipantRef::Human {
                handle: "matthew".into()
            }
        );
    }

    #[test]
    fn participant_ref_refuses_the_forged_agent_form_for_a_person() {
        // `agent:<person>@human` is the substrate asserting a persona nobody claimed. Refused in
        // the agent parser itself, so every agent-only caller refuses it too.
        let err = parse_agent_ref("agent:matthew@human").expect_err("forgery refused");
        assert!(err.to_string().contains("human:<handle>"), "got: {err}");
        assert!(parse_participant_ref("agent:matthew@human").is_err());
        assert!(ActorClaim::new("agent:matthew@human", "s1", AT, None).is_err());
    }

    #[test]
    fn participant_ref_refuses_an_email_and_any_substrate_minted_shape() {
        // An email is a cross-namespace key the substrate must never copy into a claim.
        assert!(parse_participant_ref("human:mbd06b@gmail.com").is_err());
        assert!(parse_participant_ref("human:Matthew Dowell").is_err());
        assert!(parse_participant_ref("human:").is_err());
        assert!(parse_participant_ref("human:Matthew").is_err());
        assert!(parse_participant_ref("human:matt/../x").is_err());
        assert!(parse_participant_ref("person:matthew").is_err());
        assert!(parse_participant_ref("matthew").is_err());
        // A human handle keeps the agent role charset: lowercase slug, no dots.
        assert!(parse_participant_ref("human:matt.hew").is_err());
        assert!(parse_participant_ref("human:matt-hew-2").is_ok());
    }

    #[test]
    fn a_human_claim_refuses_a_definition_cid_and_omits_the_key() {
        // A human has no build; a definition address on a human claim is a category error.
        let err = ActorClaim::new("human:matthew", "s1", AT, Some(DEF.into()))
            .expect_err("a human has no package to address");
        assert!(err.to_string().contains("human"), "got: {err}");
        // And with none, the wire shape is byte-identical in form to an agent claim without
        // one: the same struct, the same omitted key, no new field on the hashed atom.
        let c = claim("human:matthew", "s1");
        let json = serde_json::to_string(&c).unwrap();
        assert!(!json.contains("definitionCid"), "got: {json}");
        assert!(c.role().is_err(), "role/model are agent-only reads");
    }

    #[test]
    fn a_blank_session_is_refused() {
        let err = ActorClaim::new("agent:scribe@opus-5", "  \n ", AT, None)
            .expect_err("an unscoped claim cannot be superseded");
        assert!(err.to_string().contains("session"), "got: {err}");
    }

    #[test]
    fn a_blank_claimed_at_is_refused() {
        let err = ActorClaim::new("agent:scribe@opus-5", "s1", "   ", None)
            .expect_err("an undated claim is unorderable");
        assert!(err.to_string().contains("claimed_at"), "got: {err}");
    }

    #[test]
    fn a_definition_cid_that_is_not_a_full_lowercase_hex_digest_is_refused() {
        let full = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        // No scheme.
        assert!(ActorClaim::new("agent:s@m", "s1", AT, Some(full.to_string())).is_err());
        // Truncated — a fingerprint, not an address.
        assert!(ActorClaim::new(
            "agent:s@m",
            "s1",
            AT,
            Some(format!("sha256:{}", &full[..16]))
        )
        .is_err());
        // Upper-cased hex would never compare equal to a real digest.
        assert!(ActorClaim::new(
            "agent:s@m",
            "s1",
            AT,
            Some(format!("sha256:{}", full.to_uppercase()))
        )
        .is_err());
        // Not hex at all.
        assert!(ActorClaim::new(
            "agent:s@m",
            "s1",
            AT,
            Some(format!("sha256:{}", "z".repeat(64)))
        )
        .is_err());
        assert!(ActorClaim::new("agent:s@m", "s1", AT, Some(DEF.to_string())).is_ok());
    }

    #[test]
    fn role_and_model_read_back_the_parsed_halves() {
        let c = ActorClaim::new("agent:rust-architect@opus-5.1", "s1", AT, None).unwrap();
        assert_eq!(c.role().unwrap(), "rust-architect");
        assert_eq!(c.model().unwrap(), "opus-5.1");
    }

    // ── current_for ─────────────────────────────────────────────────────────────────────

    #[test]
    fn current_for_is_the_latest_claim_in_a_session_not_the_first() {
        let mut store = MemoryActorStore::new();
        store
            .append(ActorRecord::Claim(claim("agent:scribe@opus-5", "s1")))
            .unwrap();
        let later = claim("agent:rust-architect@opus-5", "s1");
        let later_cid = store.append(ActorRecord::Claim(later.clone())).unwrap();

        let (cid, current) = store.current_for("s1").unwrap().expect("a claim");
        assert_eq!(cid, later_cid);
        assert_eq!(current, later, "claims stack; the latest wins");
    }

    #[test]
    fn current_for_never_leaks_across_sessions() {
        let mut store = MemoryActorStore::new();
        let a = claim("agent:scribe@opus-5", "s1");
        let b = claim("agent:rust-architect@opus-5", "s2");
        store.append(ActorRecord::Claim(a.clone())).unwrap();
        store.append(ActorRecord::Claim(b.clone())).unwrap();

        assert_eq!(store.current_for("s1").unwrap().unwrap().1, a);
        assert_eq!(store.current_for("s2").unwrap().unwrap().1, b);
        assert!(
            store.current_for("s3").unwrap().is_none(),
            "an unregistered session is honest absence, never an error"
        );
    }

    #[test]
    fn a_superseded_claim_is_still_in_the_history() {
        let mut store = MemoryActorStore::new();
        let first = claim("agent:scribe@opus-5", "s1");
        store.append(ActorRecord::Claim(first.clone())).unwrap();
        store
            .append(ActorRecord::Claim(claim("agent:storyteller@opus-5", "s1")))
            .unwrap();

        let claims = store.claims().unwrap();
        assert_eq!(claims.len(), 2, "supersession appends; it never rewrites");
        assert_eq!(claims[0].1, first);
    }

    // ── round trip + integrity ──────────────────────────────────────────────────────────

    #[cfg(feature = "sidecar")]
    #[test]
    fn a_claim_round_trips_through_both_stores_with_the_same_cid() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = ActorClaim::new("agent:scribe@opus-5", "s1", AT, Some(DEF.to_string())).unwrap();

        let mut memory = MemoryActorStore::new();
        let memory_cid = memory.append(ActorRecord::Claim(c.clone())).unwrap();

        let mut sidecar = SidecarActorStore::open(dir.path()).unwrap();
        let sidecar_cid = sidecar.append(ActorRecord::Claim(c.clone())).unwrap();

        assert_eq!(
            memory_cid, sidecar_cid,
            "identity is the payload, not the store"
        );
        // `records()` re-verifies every stored CID, so a successful read IS the round trip.
        let read = sidecar.records().unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].0, sidecar_cid);
        assert!(matches!(&read[0].1, ActorRecord::Claim(got) if got == &c));
        assert_eq!(
            sidecar.log_path(),
            dir.path().join(".eprfs/status/actors.jsonl")
        );
    }

    #[test]
    fn a_claim_with_no_definition_cid_omits_the_key_entirely() {
        // Honest absence must not encode as a present-but-null field, or a claim made without a
        // readable package would be addressed differently from one made before the field existed.
        let c = claim("agent:scribe@opus-5", "s1");
        let json = serde_json::to_string(&c).unwrap();
        assert!(!json.contains("definitionCid"), "got: {json}");
        assert!(json.contains("claimedAt"), "camelCase on the wire: {json}");
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn a_tampered_sidecar_line_is_an_integrity_error_not_silent_drift() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut store = SidecarActorStore::open(dir.path()).unwrap();
        store
            .append(ActorRecord::Claim(claim("agent:scribe@opus-5", "s1")))
            .unwrap();

        // Rewrite the claimed identity in place, leaving the stored CID untouched — the exact
        // shape an impersonation edit would take.
        let path = store.log_path().to_path_buf();
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("agent:scribe@opus-5", "agent:storyteller@opus-5");
        fs::write(&path, text).unwrap();

        let err = store
            .records()
            .expect_err("a line whose payload no longer matches its CID must not read back");
        assert!(matches!(err, FabricError::Integrity { .. }), "got: {err:?}");
    }

    // ── golden ──────────────────────────────────────────────────────────────────────────

    /// Pinned so the canonical dag-cbor encoding of an `ActorClaim` can never silently drift
    /// (mirrors `note::tests::note_event_cid_is_stable`). Every field is a literal, so this
    /// golden is independent of git, the clock, and the tree. A change here re-addresses every
    /// claim ever written.
    #[test]
    fn actor_claim_cid_is_stable() {
        let claim = ActorClaim {
            claimed: AgentRef("agent:scribe@opus-5".to_string()),
            session: "golden-session".to_string(),
            claimed_at: "2026-08-15T00:00:00Z".to_string(),
            definition_cid: Some(
                "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
            ),
        };
        let cid = atom_cid(&claim).expect("cid");
        assert_eq!(
            cid.to_string(),
            "bafyreib3xfuy6hchjcbtrjktxfho52oqbs5pbu2e5depagycipxx4jnvpi"
        );
    }
}
