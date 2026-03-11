//! Bitswap query state machine — manages get and sync queries.

use crate::stats::{REQUESTS_TOTAL, REQUEST_DURATION_SECONDS};
use cid::Cid;
use fnv::{FnvHashMap, FnvHashSet};
use libp2p::PeerId;
use prometheus::HistogramTimer;
use std::collections::{HashSet, VecDeque};

/// Query id.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QueryId(u64);

impl std::fmt::Display for QueryId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Request.
#[derive(Debug, Eq, PartialEq)]
pub enum Request {
    /// Have query.
    Have(PeerId, Cid),
    /// Block query.
    Block(PeerId, Cid),
    /// Missing blocks query.
    MissingBlocks(Cid),
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Have(_, _) => write!(f, "have"),
            Self::Block(_, _) => write!(f, "block"),
            Self::MissingBlocks(_) => write!(f, "missing-blocks"),
        }
    }
}

/// Response.
#[derive(Debug)]
pub enum Response {
    /// Have query.
    Have(PeerId, bool),
    /// Block query.
    Block(PeerId, bool),
    /// Missing blocks query.
    MissingBlocks(Vec<Cid>),
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Have(_, have) => write!(f, "have {}", have),
            Self::Block(_, block) => write!(f, "block {}", block),
            Self::MissingBlocks(missing) => write!(f, "missing-blocks {}", missing.len()),
        }
    }
}

/// Event emitted by a query.
#[derive(Debug)]
pub enum QueryEvent {
    /// A subquery to run.
    Request(QueryId, Request),
    /// A progress event.
    Progress(QueryId, usize),
    /// Complete event.
    Complete(QueryId, Result<(), Cid>),
}

#[derive(Debug)]
pub struct Header {
    pub id: QueryId,
    pub root: QueryId,
    pub parent: Option<QueryId>,
    pub cid: Cid,
    pub timer: HistogramTimer,
    pub label: &'static str,
}

impl Drop for Header {
    fn drop(&mut self) {
        REQUESTS_TOTAL.with_label_values(&[self.label]).inc();
    }
}

#[derive(Debug)]
struct Query {
    hdr: Header,
    state: State,
}

#[derive(Debug)]
enum State {
    None,
    Get(GetState),
    Sync(SyncState),
}

#[derive(Debug, Default)]
struct GetState {
    have: FnvHashSet<QueryId>,
    block: Option<QueryId>,
    providers: Vec<PeerId>,
}

#[derive(Debug, Default)]
struct SyncState {
    missing: FnvHashSet<QueryId>,
    children: FnvHashSet<QueryId>,
    providers: Vec<PeerId>,
}

enum Transition<S, C> {
    Next(S),
    Complete(C),
}

#[derive(Default)]
pub struct QueryManager {
    id_counter: u64,
    peers: HashSet<PeerId>,
    queries: FnvHashMap<QueryId, Query>,
    events: VecDeque<QueryEvent>,
}

impl QueryManager {
    fn start_query(
        &mut self,
        root: QueryId,
        parent: Option<QueryId>,
        cid: Cid,
        req: Request,
        label: &'static str,
    ) -> QueryId {
        let timer = REQUEST_DURATION_SECONDS
            .with_label_values(&[label])
            .start_timer();
        let id = QueryId(self.id_counter);
        self.id_counter += 1;
        let query = Query {
            hdr: Header {
                id,
                root,
                parent,
                cid,
                timer,
                label,
            },
            state: State::None,
        };
        self.queries.insert(id, query);
        tracing::trace!("{} {} {}", root, id, req);
        self.events.push_back(QueryEvent::Request(id, req));
        id
    }

    fn have(&mut self, root: QueryId, parent: QueryId, peer_id: PeerId, cid: Cid) -> QueryId {
        self.start_query(root, Some(parent), cid, Request::Have(peer_id, cid), "have")
    }

    fn block(&mut self, root: QueryId, parent: QueryId, peer_id: PeerId, cid: Cid) -> QueryId {
        self.start_query(
            root,
            Some(parent),
            cid,
            Request::Block(peer_id, cid),
            "block",
        )
    }

    fn missing_blocks(&mut self, parent: QueryId, cid: Cid) -> QueryId {
        self.start_query(
            parent,
            Some(parent),
            cid,
            Request::MissingBlocks(cid),
            "missing-blocks",
        )
    }

    /// Register a connected peer.
    pub fn add_peer(&mut self, peer_id: &PeerId) {
        self.peers.insert(*peer_id);
    }

    /// Remove a disconnected peer.
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
    }

    /// Starts a query to locate and retrieve a block.
    pub fn get(
        &mut self,
        parent: Option<QueryId>,
        cid: Cid,
        providers: impl Iterator<Item = PeerId>,
    ) -> QueryId {
        let timer = REQUEST_DURATION_SECONDS
            .with_label_values(&["get"])
            .start_timer();
        let id = QueryId(self.id_counter);
        self.id_counter += 1;
        let root = parent.unwrap_or(id);
        tracing::trace!("{} {} get", root, id);
        let mut state = GetState::default();

        for peer in providers {
            if state.block.is_none() {
                state.block = Some(self.block(root, id, peer, cid));
            } else {
                state.have.insert(self.have(root, id, peer, cid));
            }
        }

        if state.block.is_none() && !self.peers.is_empty() {
            let peers: Vec<_> = self.peers.iter().copied().collect();
            for peer in peers {
                if state.block.is_none() {
                    state.block = Some(self.block(root, id, peer, cid));
                } else {
                    state.have.insert(self.have(root, id, peer, cid));
                }
            }
        }

        assert!(state.block.is_some(), "no providers for get query");
        let query = Query {
            hdr: Header {
                id,
                root,
                parent,
                cid,
                timer,
                label: "get",
            },
            state: State::Get(state),
        };
        self.queries.insert(id, query);
        id
    }

    /// Starts a sync query to recursively retrieve a DAG.
    pub fn sync(
        &mut self,
        cid: Cid,
        providers: Vec<PeerId>,
        missing: impl Iterator<Item = Cid>,
    ) -> QueryId {
        let timer = REQUEST_DURATION_SECONDS
            .with_label_values(&["sync"])
            .start_timer();
        let id = QueryId(self.id_counter);
        self.id_counter += 1;
        tracing::trace!("{} {} sync", id, id);
        let mut state = SyncState::default();
        for cid in missing {
            state
                .missing
                .insert(self.get(Some(id), cid, providers.iter().copied()));
        }
        if state.missing.is_empty() {
            state.children.insert(self.missing_blocks(id, cid));
        }
        state.providers = providers;
        let query = Query {
            hdr: Header {
                id,
                root: id,
                parent: None,
                cid,
                timer,
                label: "sync",
            },
            state: State::Sync(state),
        };
        self.queries.insert(id, query);
        id
    }

    /// Cancels an in-progress query.
    pub fn cancel(&mut self, root: QueryId) -> bool {
        let query = if let Some(query) = self.queries.remove(&root) {
            query
        } else {
            return false;
        };
        let queries = &self.queries;
        self.events.retain(|event| {
            let (id, req) = match event {
                QueryEvent::Request(id, req) => (id, req),
                QueryEvent::Progress(id, _) => return *id != root,
                QueryEvent::Complete(_, _) => return true,
            };
            if queries.get(id).map(|q| q.hdr.root) != Some(root) {
                return true;
            }
            tracing::trace!("{} {} {} cancel", root, id, req);
            false
        });
        match query.state {
            State::Get(_) => {
                tracing::trace!("{} {} get cancel", root, root);
                true
            }
            State::Sync(state) => {
                for id in state.missing {
                    tracing::trace!("{} {} get cancel", root, id);
                    self.queries.remove(&id);
                }
                tracing::trace!("{} {} sync cancel", root, root);
                true
            }
            State::None => {
                self.queries.insert(root, query);
                false
            }
        }
    }

    fn get_query<F>(&mut self, id: QueryId, f: F)
    where
        F: FnOnce(&mut Self, &Header, GetState) -> Transition<GetState, Result<(), Cid>>,
    {
        if let Some(mut parent) = self.queries.remove(&id) {
            let state = if let State::Get(state) = parent.state {
                state
            } else {
                return;
            };
            match f(self, &parent.hdr, state) {
                Transition::Next(state) => {
                    parent.state = State::Get(state);
                    self.queries.insert(id, parent);
                }
                Transition::Complete(res) => {
                    self.recv_get(parent.hdr, res);
                }
            }
        }
    }

    fn sync_query<F>(&mut self, id: QueryId, f: F)
    where
        F: FnOnce(&mut Self, &Header, SyncState) -> Transition<SyncState, Result<(), Cid>>,
    {
        if let Some(mut parent) = self.queries.remove(&id) {
            let state = if let State::Sync(state) = parent.state {
                state
            } else {
                return;
            };
            match f(self, &parent.hdr, state) {
                Transition::Next(state) => {
                    parent.state = State::Sync(state);
                    self.queries.insert(id, parent);
                }
                Transition::Complete(res) => {
                    self.recv_sync(parent.hdr, res);
                }
            }
        }
    }

    fn recv_have(&mut self, query: Header, peer_id: PeerId, have: bool) {
        self.get_query(query.parent.unwrap(), |mgr, parent, mut state| {
            state.have.remove(&query.id);
            if state.block == Some(query.id) {
                state.block = None;
            }
            if have {
                state.providers.push(peer_id);
            }
            if state.block.is_none() && !state.providers.is_empty() {
                state.block = Some(mgr.block(
                    parent.root,
                    parent.id,
                    state.providers.pop().unwrap(),
                    query.cid,
                ));
            }
            if state.have.is_empty() && state.block.is_none() {
                if state.providers.is_empty() {
                    return Transition::Complete(Err(query.cid));
                } else {
                    return Transition::Complete(Ok(()));
                }
            }
            Transition::Next(state)
        });
    }

    fn recv_block(&mut self, query: Header, peer_id: PeerId, block: bool) {
        if block {
            self.get_query(query.parent.unwrap(), |_mgr, _parent, mut state| {
                state.providers.push(peer_id);
                Transition::Complete(Ok(()))
            });
        } else {
            self.recv_have(query, peer_id, block);
        }
    }

    fn recv_missing_blocks(&mut self, query: Header, missing: Vec<Cid>) {
        let mut num_missing = 0;
        let num_missing_ref = &mut num_missing;
        self.sync_query(query.parent.unwrap(), |mgr, parent, mut state| {
            state.children.remove(&query.id);
            for cid in missing {
                state.missing.insert(mgr.get(
                    Some(parent.root),
                    cid,
                    state.providers.iter().copied(),
                ));
            }
            *num_missing_ref = state.missing.len();
            if state.missing.is_empty() && state.children.is_empty() {
                Transition::Complete(Ok(()))
            } else {
                Transition::Next(state)
            }
        });
        if num_missing != 0 {
            self.events
                .push_back(QueryEvent::Progress(query.root, num_missing));
        }
    }

    fn recv_get(&mut self, query: Header, res: Result<(), Cid>) {
        if let Some(id) = query.parent {
            self.sync_query(id, |mgr, parent, mut state| {
                state.missing.remove(&query.id);
                if res.is_err() {
                    Transition::Complete(res)
                } else {
                    state
                        .children
                        .insert(mgr.missing_blocks(parent.root, query.cid));
                    Transition::Next(state)
                }
            });
        } else {
            self.events.push_back(QueryEvent::Complete(query.id, res));
        }
    }

    fn recv_sync(&mut self, query: Header, res: Result<(), Cid>) {
        self.events.push_back(QueryEvent::Complete(query.id, res));
    }

    /// Dispatch a response to the appropriate query handler.
    pub fn inject_response(&mut self, id: QueryId, res: Response) {
        let query = if let Some(query) = self.queries.remove(&id) {
            query.hdr
        } else {
            return;
        };
        tracing::trace!("{} {} {}", query.root, query.id, res);
        match res {
            Response::Have(peer, have) => self.recv_have(query, peer, have),
            Response::Block(peer, block) => self.recv_block(query, peer, block),
            Response::MissingBlocks(cids) => self.recv_missing_blocks(query, cids),
        }
    }

    /// Returns the header of a query.
    pub fn query_info(&self, id: QueryId) -> Option<&Header> {
        self.queries.get(&id).map(|q| &q.hdr)
    }

    /// Retrieves the next query event.
    pub fn next(&mut self) -> Option<QueryEvent> {
        self.events.pop_front()
    }
}
