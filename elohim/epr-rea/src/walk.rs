//! The value-chain walk — the mechanical form of "if you change A, walk the valueflow
//! to the finish before you claim coherence."

use cid::Cid;
use elohim_epr::witness::ReaVerb;

use crate::error::Result;
use crate::model::{Commitment, FlowEvent, Process};
use crate::store::FlowStore;

/// Backward walk from a resource: how it came to be.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lineage {
    /// Events that produced this resource.
    pub producing_events: Vec<(Cid, FlowEvent)>,
    /// Process instances whose outputs include this resource.
    pub processes: Vec<(Cid, Process)>,
    /// Inputs those processes consumed/used (the previous links of the chain).
    pub inputs: Vec<Cid>,
    /// Commitments the producing events fulfilled.
    pub commitments: Vec<Cid>,
    /// Intents those commitments (or events directly) satisfied.
    pub intents: Vec<Cid>,
}

/// Forward walk from a resource: what depends on it, and what is now unfulfilled.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Frontier {
    /// Process instances whose inputs include this resource.
    pub dependents: Vec<(Cid, Process)>,
    /// Their outputs (the downstream resources a change here can invalidate).
    pub outputs: Vec<Cid>,
    /// Undischarged commitments in the dependents' scopes — the work a change here
    /// leaves open before the chain reaches the finish.
    pub unfulfilled: Vec<(Cid, Commitment)>,
}

pub trait FlowWalk {
    fn walk_back(&self, resource: &Cid) -> Result<Lineage>;
    fn walk_forward(&self, resource: &Cid) -> Result<Frontier>;
}

fn push_unique(list: &mut Vec<Cid>, cid: &Cid) {
    if !list.contains(cid) {
        list.push(*cid);
    }
}

impl<S: FlowStore + ?Sized> FlowWalk for S {
    fn walk_back(&self, resource: &Cid) -> Result<Lineage> {
        let mut lineage = Lineage::default();

        for (cid, event) in self.events()? {
            if &event.resource == resource && matches!(event.action, ReaVerb::Produce) {
                for fulfilled in &event.fulfills {
                    push_unique(&mut lineage.commitments, fulfilled);
                }
                for satisfied in &event.satisfies {
                    push_unique(&mut lineage.intents, satisfied);
                }
                lineage.producing_events.push((cid, event));
            }
        }

        for (cid, commitment) in self.commitments()? {
            if lineage.commitments.contains(&cid) {
                for satisfied in &commitment.satisfies {
                    push_unique(&mut lineage.intents, satisfied);
                }
            }
        }

        for (cid, process) in self.processes()? {
            if process.outputs.contains(resource) {
                for input in &process.inputs {
                    push_unique(&mut lineage.inputs, input);
                }
                lineage.processes.push((cid, process));
            }
        }

        Ok(lineage)
    }

    fn walk_forward(&self, resource: &Cid) -> Result<Frontier> {
        let mut frontier = Frontier::default();
        let mut scopes: Vec<Cid> = Vec::new();

        for (cid, process) in self.processes()? {
            if process.inputs.contains(resource) {
                for output in &process.outputs {
                    push_unique(&mut frontier.outputs, output);
                }
                push_unique(&mut scopes, &process.in_scope_of);
                frontier.dependents.push((cid, process));
            }
        }

        for scope in &scopes {
            for (cid, commitment) in self.unfulfilled_in_scope(scope)? {
                if !frontier.unfulfilled.iter().any(|(c, _)| c == &cid) {
                    frontier.unfulfilled.push((cid, commitment));
                }
            }
        }

        Ok(frontier)
    }
}
