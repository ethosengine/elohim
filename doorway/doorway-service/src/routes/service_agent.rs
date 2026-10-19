//! Service-agent registration: a stewarded runner, never a human.
//!
//! # What a service agent is
//!
//! A **service agent** is a stewarded compute resource — the first instance is
//! `agent:ci-runner-jenkins-matthew`, the CI runner that sends pinned Act I
//! stages to household compute providers. It is the Z.D deploy-svc-agent shape
//! (`genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md`
//! §2, §7): one agent key, "no human standing — it can only act inside its
//! bounds". It is NOT:
//!
//! - a **human**: no `imagodei::create_human` runs, no `Human` entry is
//!   authored, no `humans` projection row is bound. The existing `jenkins-ci`
//!   seed actor registered through the human path and so IS a hosted Human
//!   record today — an ontology defect this module does not repeat (and does
//!   not change in this slice).
//! - a **device that speaks for its steward**: it carries no roster row and no
//!   `DeviceBinding`, so it cannot approve a next device.
//! - an **admin**: authorizing its CREATION takes the doorway's admin bootstrap
//!   key, but the account is never promoted. "May create a runner" and "is this
//!   doorway's admin" are different questions; the human path conflates them
//!   for `jenkins-ci` and this path deliberately does not.
//!
//! # Its one standing: a hosted-cell promise from a sponsor
//!
//! The runner's cell is hosted on the sponsor's pool conductor, and that
//! hosting is the existing `hosted-cell` `delegates-compute` commitment
//! (`routes::hosted_cell`), issued under `RUNNER_TERMS`. The **provider** is the
//! pool peer acting for itself — forced by the grant surface, which refuses any
//! performer that is not that peer's own cell actor. The doorway carries the
//! message; it never promises. Unlike the human leg, the promise is REQUIRED: a
//! runner with no notarized sponsor is hosted compute nobody promised, so the
//! registration unwinds (deprovisions the cell) rather than keep it.
//!
//! Capacity grants (`measure-stage`, from each provider peer independently) are
//! not issued here — any provider issues them to the runner's agent key
//! through its own compute-grant surface.
//!
//! Everything in this module is pure so the decisions can be pinned without a
//! conductor; the register handler in `auth_routes` composes them.

use crate::auth::PermissionLevel;

/// The agency phase a stewarded runner registers under.
///
/// A DECLARED stage, never a default: a register body that names no phase is
/// still `hosted` (a human). Every existing gate that may give an account a
/// cell matches `hosted` POSITIVELY (`UserDoc::may_be_hosted`,
/// `chaperone::phase_forbids_auto_provision`, `node_key_bind::may_graduate`),
/// so this phase fails closed everywhere for free: the runner's cell is
/// installed once, at registration, and login/connect never install another.
pub const SERVICE_AGENT_PHASE: &str = "service-agent";

/// Prefix every service-agent id carries, so the id is unmistakably not a
/// human id wherever it surfaces (JWT `human_id` claim, account page).
pub const SERVICE_AGENT_ID_PREFIX: &str = "agent:";

/// Upper bound on the slug after the prefix.
const SERVICE_AGENT_SLUG_MAX: usize = 96;

/// Why a service-agent registration was refused, before or after any side
/// effect. Each variant carries a machine-readable code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAgentRefusal {
    /// `serviceAgentId` missing.
    IdMissing,
    /// `serviceAgentId` not `agent:<lowercase-slug>`.
    IdMalformed,
    /// The body named a `humanId`: a runner is never a human record.
    HumanIdForbidden,
    /// The body supplied an `agentPubKey`: the key is minted by provisioning,
    /// never claimed by the caller.
    AgentKeyForbidden,
    /// No credential store — the account could not be kept.
    NoCredentialStore,
    /// No conductor pool — there is nowhere to host the runner's cell.
    NoConductorPool,
    /// No pool-compute configuration — nobody can promise the hosting.
    NoSponsor,
    /// C1: the sponsor named is the runner itself.
    SelfSponsorship,
    /// The pool peer answered with a provider that is not the configured
    /// performer (or answered with none): the promise is not the one asked for.
    SponsorMismatch,
    /// A HUMAN registration named an `agent:` id: that namespace is reserved
    /// for service agents, because `/hc/connect` scopes on it.
    ReservedHumanId,
}

impl ServiceAgentRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::IdMissing => "SERVICE_AGENT_ID_REQUIRED",
            Self::IdMalformed => "SERVICE_AGENT_ID_INVALID",
            Self::HumanIdForbidden => "SERVICE_AGENT_NOT_HUMAN",
            Self::AgentKeyForbidden => "SERVICE_AGENT_KEY_NOT_CLAIMABLE",
            Self::NoCredentialStore => "SERVICE_AGENT_NO_CREDENTIAL_STORE",
            Self::NoConductorPool => "SERVICE_AGENT_UNHOSTABLE",
            Self::NoSponsor => "SERVICE_AGENT_NO_SPONSOR",
            Self::SelfSponsorship => "SERVICE_AGENT_SELF_SPONSORSHIP",
            Self::SponsorMismatch => "SERVICE_AGENT_SPONSOR_MISMATCH",
            Self::ReservedHumanId => "HUMAN_ID_RESERVED",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::IdMissing => "A service-agent registration must name serviceAgentId (agent:<slug>)",
            Self::IdMalformed => {
                "serviceAgentId must be 'agent:' followed by 1-96 lowercase letters, digits or '-'"
            }
            Self::HumanIdForbidden => {
                "A service agent is not a human record: humanId must not be supplied"
            }
            Self::AgentKeyForbidden => {
                "A service agent's key is minted by its hosting conductor: agentPubKey must not be supplied"
            }
            Self::NoCredentialStore => {
                "This doorway has no credential store, so it cannot keep a service-agent account"
            }
            Self::NoConductorPool => {
                "This doorway operates no conductor pool, so it cannot host a service agent's cell"
            }
            Self::NoSponsor => {
                "This doorway's pool peer is not configured to promise hosting, so no service agent can be sponsored"
            }
            Self::SelfSponsorship => "A service agent cannot sponsor itself",
            Self::SponsorMismatch => {
                "The pool peer's hosting promise did not name the expected sponsor"
            }
            Self::ReservedHumanId => {
                "humanId values beginning 'agent:' are reserved for service agents"
            }
        }
    }

    /// The HTTP status the register handler answers with. Request-shape
    /// refusals are 400; a doorway that cannot host or sponsor is 503; a
    /// sponsorship that failed its own integrity check is 502 (the upstream
    /// answered, but not with the promise asked for) — except self-sponsorship,
    /// which is a refusal of the request itself (403).
    pub fn status(self) -> hyper::StatusCode {
        use hyper::StatusCode;
        match self {
            Self::IdMissing
            | Self::IdMalformed
            | Self::HumanIdForbidden
            | Self::AgentKeyForbidden
            | Self::ReservedHumanId => StatusCode::BAD_REQUEST,
            Self::NoCredentialStore | Self::NoConductorPool | Self::NoSponsor => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Self::SelfSponsorship => StatusCode::FORBIDDEN,
            Self::SponsorMismatch => StatusCode::BAD_GATEWAY,
        }
    }
}

/// Parse and validate a `serviceAgentId`. Pure.
///
/// `agent:` followed by 1..=96 of `[a-z0-9-]`, not starting or ending with `-`.
/// Lowercase-only so two spellings of one runner cannot become two accounts.
pub fn parse_service_agent_id(raw: Option<&str>) -> Result<String, ServiceAgentRefusal> {
    let raw = raw.map(str::trim).filter(|s| !s.is_empty());
    let Some(raw) = raw else {
        return Err(ServiceAgentRefusal::IdMissing);
    };
    let slug = raw
        .strip_prefix(SERVICE_AGENT_ID_PREFIX)
        .ok_or(ServiceAgentRefusal::IdMalformed)?;
    let well_formed = !slug.is_empty()
        && slug.len() <= SERVICE_AGENT_SLUG_MAX
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !well_formed {
        return Err(ServiceAgentRefusal::IdMalformed);
    }
    Ok(raw.to_string())
}

/// The request-shape checks a service-agent registration must pass before any
/// side effect. Pure. Returns the validated service-agent id.
///
/// A runner is never a human record (`humanId` refused) and never claims its
/// own key (`agentPubKey` refused) — the hosting conductor mints it.
pub fn validate_service_agent_request(
    service_agent_id: Option<&str>,
    human_id: &str,
    agent_pub_key: &str,
) -> Result<String, ServiceAgentRefusal> {
    if !human_id.trim().is_empty() {
        return Err(ServiceAgentRefusal::HumanIdForbidden);
    }
    if !agent_pub_key.trim().is_empty() {
        return Err(ServiceAgentRefusal::AgentKeyForbidden);
    }
    parse_service_agent_id(service_agent_id)
}

/// C1 anti-self-election, before the promise is asked for. Pure.
///
/// The sponsor (the pool peer's configured performer — the provider the grant
/// surface will write) must be a named key and must not be the runner's own
/// freshly minted key. Structurally the grant surface already makes the
/// provider its own cell actor, so a runner cannot reach this path as its own
/// sponsor; this check is the doorway refusing to even ask.
pub fn sponsorship_is_not_self(sponsor: &str, runner: &str) -> Result<(), ServiceAgentRefusal> {
    let sponsor = sponsor.trim();
    if sponsor.is_empty() {
        return Err(ServiceAgentRefusal::NoSponsor);
    }
    if sponsor == runner.trim() {
        return Err(ServiceAgentRefusal::SelfSponsorship);
    }
    Ok(())
}

/// After the promise: the provider the pool peer NAMED must be the sponsor the
/// doorway asked, and still not the runner. Pure. Returns the sponsor to record.
///
/// A missing provider is a refusal here (unlike the human leg, where it only
/// leaves the household unnamed): the sponsor IS the runner's standing, and a
/// runner whose sponsor cannot be named cannot be held to account by anyone.
pub fn sponsor_named_by_peer(
    answered_provider: Option<&str>,
    configured_performer: &str,
    runner: &str,
) -> Result<String, ServiceAgentRefusal> {
    let provider = answered_provider
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or(ServiceAgentRefusal::SponsorMismatch)?;
    if provider != configured_performer.trim() {
        return Err(ServiceAgentRefusal::SponsorMismatch);
    }
    sponsorship_is_not_self(provider, runner)?;
    Ok(provider.to_string())
}

/// Whether a supplied admin bootstrap key PROMOTES the registrant to Admin.
/// Pure.
///
/// For every human phase this keeps today's behavior (a matching key promotes;
/// that is how the `jenkins-ci` seed actor is provisioned, unchanged in this
/// slice). For a service agent the key only AUTHORIZES creation — checked in
/// the register arm by `operator_registration_authority` — and never promotes.
pub fn bootstrap_key_promotes_to_admin(agency_phase: &str) -> bool {
    agency_phase != SERVICE_AGENT_PHASE
}

/// The permission level a service-agent account is stored with. Pure, and the
/// only answer is `Authenticated`: a runner acts inside its grants, and the
/// doorway's permission ladder is not where its authority lives.
pub fn service_agent_permission_level() -> PermissionLevel {
    PermissionLevel::Authenticated
}

/// Whether a registration in this phase authors a `Human` (imagodei
/// `create_human`) or binds a `humans` projection row. Pure. False only for a
/// service agent.
pub fn phase_authors_human_record(agency_phase: &str) -> bool {
    agency_phase != SERVICE_AGENT_PHASE
}

// =============================================================================
// The runner's /hc/connect capability: function-listed, never All
// =============================================================================

/// Is this session a stewarded runner's? Pure, keyed on the SIGNED `human_id`
/// claim, which for a service agent is its `agent:<slug>` id (stored as the
/// row's `human_id` at registration and carried by every token minted from
/// that row). No I/O, so `/hc/connect` decides without a credential-store read
/// and cannot fail open on one. The namespace is reserved: a human
/// registration naming an `agent:` id is refused ([`human_id_is_reserved`]),
/// so a human can never be mistaken for a runner — and a runner, whose id is
/// always prefixed, can never be mistaken for a human.
pub fn is_service_agent_session(human_id_claim: &str) -> bool {
    human_id_claim.starts_with(SERVICE_AGENT_ID_PREFIX)
}

/// Whether a HUMAN registration's caller-supplied `humanId` falls in the
/// service-agent namespace (and must be refused). Pure.
pub fn human_id_is_reserved(agency_phase: &str, human_id: &str) -> bool {
    agency_phase != SERVICE_AGENT_PHASE && is_service_agent_session(human_id.trim())
}

/// What one role cell's `/hc/connect` capability may call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionGrantScope {
    /// Every function: a human's own hosted cell. Unchanged.
    All,
    /// Exactly these `(zome, function)` pairs.
    Listed(&'static [(&'static str, &'static str)]),
    /// Author no grant on this cell at all: a permanent chain action that
    /// could call nothing is not worth writing.
    NoGrant,
}

/// The runner's whole callable surface on its `lamad` (content) cell: submit
/// and follow a compute task it requests. `compute_task` is ONE extern with an
/// `operation` field (`submit`/`get`/`list`, plus provider-only
/// `accept`/`complete`/`decline` that the zome refuses unless the caller IS
/// the task's provider — `content_store/src/compute_task.rs`). Derived from the
/// requester flow in `genesis/agentic/compute/workspace.mjs` (submit → POST
/// tasks; poll → GET tasks/{hash}), which the storage adapter maps onto exactly
/// this extern (`elohim-storage/src/api/compute_tasks.rs` `call`).
pub const RUNNER_CONTENT_CELL_FUNCTIONS: &[(&str, &str)] = &[("content_store", "compute_task")];

/// The runner's reads on its `mishpat` (governance) cell: the two the
/// requester-side completion check uses to verify the grant it spends — the
/// exact signed grant record and its authority/withdrawal links
/// (`compute_tasks.rs` `fetch_authenticated_grant`, `observe`). Read-only;
/// no commitment authoring, so the runner cannot issue or accept a grant.
pub const RUNNER_GOVERNANCE_CELL_FUNCTIONS: &[(&str, &str)] = &[
    ("mishpat", "get_commitment_record"),
    ("mishpat", "get_commitment_authority_links"),
];

/// The capability a session gets on one role cell. Pure.
///
/// Human sessions: `All` on every cell, exactly as before. Runner sessions:
/// `lamad` and `mishpat` are function-listed; every other role — `imagodei`
/// (identity: `create_human`, device approval, recovery), `infrastructure`,
/// `node_registry`, and any clone cell — gets NO grant. Fail closed: an
/// unknown role is never widened for a runner.
pub fn session_grant_scope(human_id_claim: &str, role: &str) -> SessionGrantScope {
    if !is_service_agent_session(human_id_claim) {
        return SessionGrantScope::All;
    }
    match role {
        "lamad" => SessionGrantScope::Listed(RUNNER_CONTENT_CELL_FUNCTIONS),
        "mishpat" => SessionGrantScope::Listed(RUNNER_GOVERNANCE_CELL_FUNCTIONS),
        _ => SessionGrantScope::NoGrant,
    }
}

// =============================================================================
// The side-effecting leg, composed from the pure decisions above
// =============================================================================

/// A runner that now exists: its cell is installed and its sponsor promised.
#[derive(Debug, Clone)]
pub struct ProvisionedServiceAgent {
    pub service_agent_id: String,
    pub agent: crate::conductor::ProvisionedAgent,
    pub grant: crate::routes::hosted_cell::HostedCellGrant,
    /// The pool peer that promised the hosting (checked: configured performer,
    /// never the runner).
    pub sponsor: String,
    cfg: crate::routes::hosted_cell::PoolComputeConfig,
}

/// Why the service-agent leg refused, carried to the register handler's
/// response verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceAgentError {
    pub status: hyper::StatusCode,
    pub code: String,
    pub message: String,
}

impl From<ServiceAgentRefusal> for ServiceAgentError {
    fn from(r: ServiceAgentRefusal) -> Self {
        Self {
            status: r.status(),
            code: r.code().into(),
            message: r.message().into(),
        }
    }
}

/// Install a stewarded runner's cell and obtain its sponsor's hosting promise.
///
/// Order is chosen so every refusal that CAN come before a side effect does:
/// creation authority (the admin bootstrap key — same trust class as the
/// `jenkins-ci` provisioning), request shape, credential store, conductor pool
/// and sponsor configuration are all checked first. Then the cell is
/// provisioned, C1 is checked against the minted key, and the promise is asked
/// for. A failure after provisioning DEPROVISIONS the cell: no `create_human`
/// runs on this path, so uninstalling cannot strand a Human, and a runner with
/// no notarized sponsor must not keep hosted compute. A retry is safe — the
/// provisioner finds an existing app for the identifier before installing.
pub async fn provision_service_agent(
    state: &crate::server::AppState,
    identifier: &str,
    service_agent_id: Option<&str>,
    human_id: &str,
    agent_pub_key: &str,
    admin_bootstrap_key: Option<&str>,
    credential_store_present: bool,
) -> Result<ProvisionedServiceAgent, ServiceAgentError> {
    use crate::routes::hosted_cell::{issue_runner_hosted_cell_grant, pool_compute_config};

    // SCAFFOLD (C13 graduated authority). Today's creation criterion is the
    // doorway admin bootstrap key — the same trust class that provisions
    // `jenkins-ci`, and weaker than intended: it proves "holds this doorway's
    // admin secret", not "is the sponsoring steward". SUCCESSOR: creation is
    // authorized by the sponsoring pool peer itself — a verb on its own local
    // compute API (the `operator_verbs` family) that mints the runner key and
    // issues the `hosted-cell` promise, so the promiser and the authorizer are
    // one party at the acting node. GRADUATION TRIGGER: that peer-side verb
    // lands, or the runner's reach ceiling is asked to rise above `commons`
    // (never on this scaffold). Production hardening of the credential:
    // genesis/data/timeline/backlog/security-ci-substrate-authorization-grant-coherence.md.
    if let Err(refusal) = crate::routes::auth_routes::operator_registration_authority(
        admin_bootstrap_key,
        state.args.api_key_admin.as_deref(),
    ) {
        tracing::warn!(
            identifier,
            code = refusal.code(),
            "service-agent: refused creation without operator authority"
        );
        return Err(ServiceAgentError {
            status: hyper::StatusCode::FORBIDDEN,
            code: refusal.code().into(),
            message: refusal.message().into(),
        });
    }
    let service_agent_id =
        validate_service_agent_request(service_agent_id, human_id, agent_pub_key)?;
    if !credential_store_present {
        return Err(ServiceAgentRefusal::NoCredentialStore.into());
    }
    let registry = state
        .conductor_registry
        .as_ref()
        .ok_or(ServiceAgentRefusal::NoConductorPool)?;
    let cfg = pool_compute_config(&state.args).ok_or(ServiceAgentRefusal::NoSponsor)?;

    let provisioner = crate::conductor::AgentProvisioner::new(std::sync::Arc::clone(registry))
        .with_app_id(state.args.installed_app_id.clone())
        .with_bundle_path(state.args.happ_bundle_path.clone());
    let agent = provisioner
        .provision_agent(identifier)
        .await
        .map_err(|e| ServiceAgentError {
            status: hyper::StatusCode::SERVICE_UNAVAILABLE,
            code: "PROVISIONING_FAILED".into(),
            message: format!("Agent provisioning failed: {e}"),
        })?;

    let promised = async {
        sponsorship_is_not_self(&cfg.performer, &agent.agent_pub_key)?;
        let grant = issue_runner_hosted_cell_grant(&cfg, &agent.agent_pub_key, chrono::Utc::now())
            .await
            .map_err(|e| ServiceAgentError {
                status: hyper::StatusCode::SERVICE_UNAVAILABLE,
                code: "SERVICE_AGENT_SPONSOR_UNREACHABLE".into(),
                message: format!("The sponsor's hosting promise could not be notarized: {e}"),
            })?;
        let sponsor = sponsor_named_by_peer(
            grant.provider.as_deref(),
            &cfg.performer,
            &agent.agent_pub_key,
        )
        .map_err(ServiceAgentError::from);
        Ok::<_, ServiceAgentError>((grant, sponsor))
    }
    .await;

    match promised {
        Ok((grant, Ok(sponsor))) => {
            tracing::info!(
                identifier,
                service_agent = %service_agent_id,
                agent = %agent.agent_pub_key,
                sponsor = %sponsor,
                grant_cid = %grant.grant_cid,
                valid_until = %grant.valid_until,
                "service-agent: cell hosted under a notarized sponsor promise"
            );
            Ok(ProvisionedServiceAgent {
                service_agent_id,
                agent,
                grant,
                sponsor,
                cfg,
            })
        }
        Ok((grant, Err(e))) => {
            // The peer promised, but not as asked: withdraw it, then uninstall.
            unwind(
                &cfg,
                Some(&grant.grant_cid),
                &provisioner,
                Some(&agent.agent_pub_key),
            )
            .await;
            Err(e)
        }
        Err(e) => {
            unwind(&cfg, None, &provisioner, Some(&agent.agent_pub_key)).await;
            Err(e)
        }
    }
}

/// Undo a runner whose account row could not be kept: withdraw THIS
/// registration's promise and — unless `keep_cell` — uninstall its cell.
/// Best-effort, logged.
///
/// `keep_cell` is for the duplicate-identifier race: the provisioner keys the
/// cell by identifier, so the concurrent registration that won the row holds
/// the SAME cell. Uninstalling it would strand the winner; only this
/// registration's own (separately notarized) promise is withdrawn.
pub async fn unwind_service_agent(
    state: &crate::server::AppState,
    sa: &ProvisionedServiceAgent,
    keep_cell: bool,
) {
    let Some(registry) = state.conductor_registry.as_ref() else {
        return;
    };
    let provisioner = crate::conductor::AgentProvisioner::new(std::sync::Arc::clone(registry))
        .with_app_id(state.args.installed_app_id.clone());
    let uninstall = (!keep_cell).then_some(sa.agent.agent_pub_key.as_str());
    unwind(&sa.cfg, Some(&sa.grant.grant_cid), &provisioner, uninstall).await;
}

async fn unwind(
    cfg: &crate::routes::hosted_cell::PoolComputeConfig,
    grant_cid: Option<&str>,
    provisioner: &crate::conductor::AgentProvisioner,
    uninstall_agent: Option<&str>,
) {
    if let Some(cid) = grant_cid {
        if let Err(e) = crate::routes::hosted_cell::revoke_hosted_cell_grant(cfg, cid).await {
            tracing::warn!(
                grant_cid = cid,
                "service-agent unwind: promise not withdrawn: {e}"
            );
        }
    }
    if let Some(agent) = uninstall_agent {
        if let Err(e) = provisioner.deprovision_agent(agent).await {
            tracing::warn!(agent, "service-agent unwind: cell not uninstalled: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_agent_id_must_be_a_prefixed_lowercase_slug() {
        assert_eq!(
            parse_service_agent_id(Some("agent:ci-runner-jenkins-matthew")),
            Ok("agent:ci-runner-jenkins-matthew".to_string())
        );
        assert_eq!(
            parse_service_agent_id(Some("  agent:r1  ")),
            Ok("agent:r1".to_string()),
            "surrounding whitespace is not part of the id"
        );
        assert_eq!(
            parse_service_agent_id(None),
            Err(ServiceAgentRefusal::IdMissing)
        );
        assert_eq!(
            parse_service_agent_id(Some("  ")),
            Err(ServiceAgentRefusal::IdMissing)
        );
        for bad in [
            "ci-runner",       // no prefix
            "human:ci-runner", // wrong prefix
            "agent:",          // empty slug
            "agent:CI-Runner", // uppercase: two spellings, one runner
            "agent:ci runner", // whitespace
            "agent:-ci",       // leading dash
            "agent:ci-",       // trailing dash
            "agent:ci/runner", // path separator
            "agent:ci_runner", // underscore
        ] {
            assert_eq!(
                parse_service_agent_id(Some(bad)),
                Err(ServiceAgentRefusal::IdMalformed),
                "{bad} must be refused"
            );
        }
        let long = format!("agent:{}", "a".repeat(SERVICE_AGENT_SLUG_MAX + 1));
        assert_eq!(
            parse_service_agent_id(Some(&long)),
            Err(ServiceAgentRefusal::IdMalformed)
        );
    }

    #[test]
    fn a_service_agent_names_no_human_and_claims_no_key() {
        assert_eq!(
            validate_service_agent_request(Some("agent:r1"), "", ""),
            Ok("agent:r1".to_string())
        );
        assert_eq!(
            validate_service_agent_request(Some("agent:r1"), "human-matthew", ""),
            Err(ServiceAgentRefusal::HumanIdForbidden),
            "a runner must never be bound to a human record"
        );
        assert_eq!(
            validate_service_agent_request(Some("agent:r1"), "", "uhCAkClaimed"),
            Err(ServiceAgentRefusal::AgentKeyForbidden),
            "a caller-claimed key would let anyone register a runner over someone else's key"
        );
    }

    /// C1 anti-self-election, both before and after the promise.
    #[test]
    fn a_runner_cannot_sponsor_itself() {
        assert_eq!(sponsorship_is_not_self("uhCAkPool", "uhCAkRunner"), Ok(()));
        assert_eq!(
            sponsorship_is_not_self("uhCAkRunner", "uhCAkRunner"),
            Err(ServiceAgentRefusal::SelfSponsorship)
        );
        assert_eq!(
            sponsorship_is_not_self("  ", "uhCAkRunner"),
            Err(ServiceAgentRefusal::NoSponsor)
        );
        assert_eq!(
            sponsor_named_by_peer(Some("uhCAkPool"), "uhCAkPool", "uhCAkRunner"),
            Ok("uhCAkPool".to_string())
        );
        assert_eq!(
            sponsor_named_by_peer(Some("uhCAkRunner"), "uhCAkRunner", "uhCAkRunner"),
            Err(ServiceAgentRefusal::SelfSponsorship),
            "even a peer that answered as the runner cannot make it its own sponsor"
        );
    }

    /// C12: only the pool peer promises. A promise naming anyone else — or
    /// nobody — is not the one the doorway asked for, and the runner is unwound.
    #[test]
    fn the_sponsor_recorded_is_the_pool_peer_that_promised() {
        assert_eq!(
            sponsor_named_by_peer(Some("uhCAkSomeoneElse"), "uhCAkPool", "uhCAkRunner"),
            Err(ServiceAgentRefusal::SponsorMismatch)
        );
        assert_eq!(
            sponsor_named_by_peer(None, "uhCAkPool", "uhCAkRunner"),
            Err(ServiceAgentRefusal::SponsorMismatch),
            "an unnamed sponsor cannot be held to account"
        );
        assert_eq!(
            sponsor_named_by_peer(Some(""), "uhCAkPool", "uhCAkRunner"),
            Err(ServiceAgentRefusal::SponsorMismatch)
        );
    }

    /// C13: the bootstrap key authorizes creating a runner; it never makes the
    /// runner this doorway's admin. Human phases keep today's promotion.
    #[test]
    fn a_service_agent_is_never_admin() {
        assert!(!bootstrap_key_promotes_to_admin(SERVICE_AGENT_PHASE));
        assert_eq!(
            service_agent_permission_level(),
            PermissionLevel::Authenticated
        );
        assert_ne!(service_agent_permission_level(), PermissionLevel::Admin);
        for human_phase in ["hosted", "doorway", "node", "device"] {
            assert!(
                bootstrap_key_promotes_to_admin(human_phase),
                "{human_phase}: jenkins-ci provisioning is unchanged in this slice"
            );
        }
    }

    #[test]
    fn no_human_record_is_authored_for_a_service_agent() {
        assert!(!phase_authors_human_record(SERVICE_AGENT_PHASE));
        for human_phase in ["hosted", "doorway", "node", "device"] {
            assert!(phase_authors_human_record(human_phase));
        }
    }

    /// The phase fails closed at every gate that may give an account a cell,
    /// because each matches `hosted` positively.
    #[test]
    fn the_service_agent_phase_is_never_treated_as_hosted() {
        let row = crate::db::schemas::UserDoc {
            agency_phase: Some(SERVICE_AGENT_PHASE.to_string()),
            ..Default::default()
        };
        assert!(
            !row.may_be_hosted(),
            "login must never install a second cell"
        );
        assert!(
            !crate::routes::node_key_bind::may_graduate(&row),
            "a runner does not graduate onto a node key"
        );
        assert!(
            crate::conductor::chaperone::phase_forbids_auto_provision(Some(SERVICE_AGENT_PHASE)),
            "/hc/connect must never auto-install a second cell for a runner"
        );
    }

    /// The register body a runner-ensure script sends parses into the fields
    /// the service-agent arm reads; hosting is DECLARED by the phase, and a body
    /// that names no phase stays a human (`hosted`) registration.
    #[test]
    fn the_register_body_declares_the_service_agent_kind() {
        let body: crate::routes::auth_routes::RegisterRequest =
            serde_json::from_value(serde_json::json!({
                "identifier": "ci-runner-jenkins@alpha.elohim.host",
                "password": "x".repeat(48),
                "agencyPhase": "service-agent",
                "serviceAgentId": "agent:ci-runner-jenkins-matthew",
                "adminBootstrapKey": "k",
            }))
            .expect("register body");
        assert_eq!(body.agency_phase.as_deref(), Some(SERVICE_AGENT_PHASE));
        assert_eq!(
            validate_service_agent_request(
                body.service_agent_id.as_deref(),
                &body.human_id,
                &body.agent_pub_key
            ),
            Ok("agent:ci-runner-jenkins-matthew".to_string())
        );
        let human: crate::routes::auth_routes::RegisterRequest =
            serde_json::from_value(serde_json::json!({"identifier": "a@b", "password": "p"}))
                .expect("human body");
        assert_eq!(
            human.agency_phase, None,
            "no phase is never a service agent"
        );
        assert_eq!(human.service_agent_id, None);
    }

    /// Every role a hosted cell carries today (elohim `workdir/happ.yaml`).
    const ROLES: [&str; 5] = [
        "lamad",
        "infrastructure",
        "imagodei",
        "mishpat",
        "node_registry",
    ];

    fn runner_surface() -> Vec<(&'static str, &'static str)> {
        ROLES
            .iter()
            .flat_map(
                |role| match session_grant_scope("agent:ci-runner-jenkins-matthew", role) {
                    SessionGrantScope::Listed(fns) => fns.to_vec(),
                    SessionGrantScope::NoGrant => Vec::new(),
                    SessionGrantScope::All => {
                        panic!("a runner got an all-function grant on {role}")
                    }
                },
            )
            .collect()
    }

    /// C12: the runner's capability is function-listed and holds exactly what
    /// submitting and following its own compute task needs.
    #[test]
    fn a_runner_grant_is_function_listed_and_includes_compute_task() {
        assert_eq!(
            session_grant_scope("agent:r1", "lamad"),
            SessionGrantScope::Listed(&[("content_store", "compute_task")])
        );
        assert_eq!(
            session_grant_scope("agent:r1", "mishpat"),
            SessionGrantScope::Listed(&[
                ("mishpat", "get_commitment_record"),
                ("mishpat", "get_commitment_authority_links"),
            ])
        );
        assert!(runner_surface().contains(&("content_store", "compute_task")));
    }

    /// The runner cannot author a human record, approve a device, run
    /// recovery, or issue/accept a commitment: none of those functions — nor
    /// any identity-cell function at all — is in its capability.
    #[test]
    fn a_runner_grant_excludes_human_authoring_and_device_approval() {
        let surface = runner_surface();
        for (zome, function) in [
            ("imagodei", "create_human"),
            ("imagodei", "update_human"),
            ("imagodei", "create_hosted_agent_binding"),
            ("imagodei", "create_self_revocation"),
            ("mishpat", "create_commitment"),
            ("mishpat", "create_mishpat_commitment"),
        ] {
            assert!(
                !surface.contains(&(zome, function)),
                "runner must not hold {zome}::{function}"
            );
        }
        assert!(
            surface.iter().all(|(zome, _)| *zome != "imagodei"),
            "no identity-cell function at all"
        );
        assert_eq!(
            session_grant_scope("agent:r1", "imagodei"),
            SessionGrantScope::NoGrant,
            "no grant is even authored on the identity cell"
        );
        assert_eq!(
            session_grant_scope("agent:r1", "lamad.fixtures"),
            SessionGrantScope::NoGrant,
            "an unknown or clone role is never widened for a runner"
        );
        assert!(
            surface
                .iter()
                .all(|(_, f)| f.starts_with("get_") || *f == "compute_task"),
            "beyond compute_task the runner holds reads only"
        );
    }

    /// Human sessions keep today's grant: All, on every role.
    #[test]
    fn a_human_session_grant_is_unchanged() {
        for human_id in ["human-matthew", "3f1c0e7e-uuid", "uhCHkabc", ""] {
            for role in ROLES {
                assert_eq!(session_grant_scope(human_id, role), SessionGrantScope::All);
            }
        }
    }

    /// The `agent:` namespace the scope keys on is reserved: a human
    /// registration may not claim it, so a human is never down-scoped by
    /// accident and never passes as a runner.
    #[test]
    fn the_service_agent_namespace_is_reserved_from_human_registration() {
        assert!(human_id_is_reserved("hosted", "agent:x"));
        assert!(human_id_is_reserved("doorway", " agent:x"));
        assert!(human_id_is_reserved("node", "agent:x"));
        assert!(!human_id_is_reserved("hosted", "human-matthew"));
        assert!(!human_id_is_reserved("hosted", ""));
        assert!(!human_id_is_reserved(SERVICE_AGENT_PHASE, "agent:x"));
        assert_eq!(
            ServiceAgentRefusal::ReservedHumanId.status(),
            hyper::StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn refusal_statuses_separate_request_shape_from_capability() {
        use hyper::StatusCode;
        assert_eq!(
            ServiceAgentRefusal::IdMalformed.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ServiceAgentRefusal::HumanIdForbidden.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ServiceAgentRefusal::NoSponsor.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            ServiceAgentRefusal::SelfSponsorship.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ServiceAgentRefusal::SponsorMismatch.status(),
            StatusCode::BAD_GATEWAY
        );
        // Every code is distinct, so a script can branch on it.
        let all = [
            ServiceAgentRefusal::IdMissing,
            ServiceAgentRefusal::IdMalformed,
            ServiceAgentRefusal::HumanIdForbidden,
            ServiceAgentRefusal::AgentKeyForbidden,
            ServiceAgentRefusal::NoCredentialStore,
            ServiceAgentRefusal::NoConductorPool,
            ServiceAgentRefusal::NoSponsor,
            ServiceAgentRefusal::SelfSponsorship,
            ServiceAgentRefusal::SponsorMismatch,
            ServiceAgentRefusal::ReservedHumanId,
        ];
        let codes: std::collections::HashSet<_> = all.iter().map(|r| r.code()).collect();
        assert_eq!(codes.len(), all.len());
    }
}
