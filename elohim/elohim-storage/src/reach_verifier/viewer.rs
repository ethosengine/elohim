//! Boundary-minted viewer identities.

use diesel::SqliteConnection;

/// The evidence class that binds a viewer to an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Proof {
    /// No agent binding was presented.
    None,
    /// A doorway asserted an agent through the HTTP hop header.
    HopAsserted,
    /// A transport binding exists but is self-asserted.
    SelfAssertedBinding,
    /// Both transport and agent keys signed the binding.
    CrossSigned,
    /// The call originated inside this process.
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ViewerKind {
    Anonymous,
    DoorwayHop {
        agent_cid: String,
    },
    #[cfg(feature = "p2p")]
    Libp2p {
        peer: libp2p::PeerId,
    },
    #[cfg(feature = "p2p-iroh")]
    Iroh {
        node: iroh::NodeId,
    },
    Local,
}

/// A caller minted at an authenticated transport or HTTP extraction boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Viewer {
    kind: ViewerKind,
}

impl Viewer {
    /// Mint the anonymous viewer.
    pub fn anonymous() -> Self {
        Self {
            kind: ViewerKind::Anonymous,
        }
    }

    /// Mint a doorway-hop viewer from the HTTP extractor's resolved header value.
    pub(crate) fn from_http_extractor(agent_cid: Option<&str>) -> Self {
        match agent_cid {
            Some(agent_cid) => Self {
                kind: ViewerKind::DoorwayHop {
                    agent_cid: agent_cid.to_string(),
                },
            },
            None => Self::anonymous(),
        }
    }

    /// Mint a viewer from an authenticated libp2p connection.
    #[cfg(feature = "p2p")]
    pub fn from_libp2p(peer: libp2p::PeerId) -> Self {
        Self {
            kind: ViewerKind::Libp2p { peer },
        }
    }

    /// Mint a viewer from an authenticated iroh connection.
    #[cfg(feature = "p2p-iroh")]
    pub fn from_iroh(node: iroh::NodeId) -> Self {
        Self {
            kind: ViewerKind::Iroh { node },
        }
    }

    /// Mint the in-process viewer. Socket locality never calls this constructor.
    pub fn local() -> Self {
        Self {
            kind: ViewerKind::Local,
        }
    }

    /// Whether the boundary presented any identity claim.
    pub fn identity_present(&self) -> bool {
        !matches!(self.kind, ViewerKind::Anonymous)
    }

    /// Convert a typed transport viewer to the custody resolver's requester.
    pub(super) fn custody_requester(&self) -> Option<crate::services::custody_standing::Requester> {
        match &self.kind {
            ViewerKind::Anonymous | ViewerKind::DoorwayHop { .. } => None,
            #[cfg(feature = "p2p")]
            ViewerKind::Libp2p { peer } => Some(
                crate::services::custody_standing::Requester::libp2p(peer.to_base58()),
            ),
            #[cfg(feature = "p2p-iroh")]
            ViewerKind::Iroh { node } => Some(crate::services::custody_standing::Requester::iroh(
                node.to_string(),
            )),
            ViewerKind::Local => Some(crate::services::custody_standing::Requester::local()),
        }
    }

    /// Resolve this viewer once from records held by storage.
    pub(super) fn resolve(&self, conn: &mut SqliteConnection) -> ResolvedViewer {
        match &self.kind {
            ViewerKind::Anonymous => ResolvedViewer::anonymous(),
            ViewerKind::DoorwayHop { agent_cid } => {
                ResolvedViewer::from_agent(conn, agent_cid.clone(), Proof::HopAsserted)
            }
            #[cfg(feature = "p2p")]
            ViewerKind::Libp2p { peer } => {
                let peer_id = peer.to_base58();
                let observed_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                match crate::db::peer_identity_bindings::lookup_active(conn, &peer_id, &observed_at)
                {
                    Ok(Some(binding)) => {
                        let proof = if binding.is_cross_signed() {
                            Proof::CrossSigned
                        } else {
                            Proof::SelfAssertedBinding
                        };
                        ResolvedViewer::from_agent(conn, binding.agent_cid, proof)
                    }
                    Ok(None) | Err(_) => ResolvedViewer::unresolved(Proof::None),
                }
            }
            #[cfg(feature = "p2p-iroh")]
            ViewerKind::Iroh { node } => {
                match crate::p2p_iroh::peer_map::lookup_by_iroh_node_id(conn, &node.to_string()) {
                    Ok(Some(manifest)) => ResolvedViewer::from_agent(
                        conn,
                        manifest.agent_cid,
                        Proof::SelfAssertedBinding,
                    ),
                    Ok(None) | Err(_) => ResolvedViewer::unresolved(Proof::None),
                }
            }
            ViewerKind::Local => ResolvedViewer {
                human: None,
                agent: None,
                proof: Proof::Local,
            },
        }
    }
}

/// A viewer resolved from storage records once per request or connection.
#[derive(Debug, Clone)]
pub struct ResolvedViewer {
    human: Option<crate::db::models::Human>,
    agent: Option<String>,
    proof: Proof,
}

impl ResolvedViewer {
    fn from_agent(conn: &mut SqliteConnection, agent: String, proof: Proof) -> Self {
        let human = crate::db::humans::get_human_by_id(conn, &agent)
            .ok()
            .flatten()
            .or_else(|| {
                crate::db::humans::get_human_by_agent_key(conn, &agent)
                    .ok()
                    .flatten()
            });
        Self {
            human,
            agent: Some(agent),
            proof,
        }
    }

    /// Construct the anonymous resolved viewer.
    pub fn anonymous() -> Self {
        Self {
            human: None,
            agent: None,
            proof: Proof::None,
        }
    }

    /// Construct an unresolved transport viewer with the given evidence class.
    pub fn unresolved(proof: Proof) -> Self {
        Self {
            human: None,
            agent: None,
            proof,
        }
    }

    /// Construct the already-resolved HTTP list viewer.
    pub(crate) fn from_http_human(human: Option<&crate::db::models::Human>) -> Self {
        Self {
            human: human.cloned(),
            agent: human.map(|h| h.id.clone()),
            proof: if human.is_some() {
                Proof::HopAsserted
            } else {
                Proof::None
            },
        }
    }

    /// The resolved human projection.
    pub fn human(&self) -> Option<&crate::db::models::Human> {
        self.human.as_ref()
    }

    /// The resolved or hop-asserted canonical agent identifier.
    pub fn agent(&self) -> Option<&str> {
        self.agent.as_deref()
    }

    /// The evidence class supporting the binding.
    pub fn proof(&self) -> Proof {
        self.proof
    }
}
