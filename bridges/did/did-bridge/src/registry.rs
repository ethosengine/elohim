//! `MethodRegistry` — routes a DID to the registered resolver by method name.
//! This is the composition point runtimes (doorway, storage) build once and
//! query per request.

use std::collections::HashMap;

use did_types::Did;

use crate::resolver::{DidResolutionError, DidResolutionResult, DidResolver};

/// Routes DIDs to method resolvers. Register one resolver per method; resolution
/// dispatches on `Did::method()`.
#[derive(Default)]
pub struct MethodRegistry {
    resolvers: HashMap<&'static str, Box<dyn DidResolver>>,
}

impl MethodRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        MethodRegistry {
            resolvers: HashMap::new(),
        }
    }

    /// Register a resolver, keyed by its own `method()`. Returns `self` for
    /// builder-style chaining.
    pub fn with(mut self, resolver: Box<dyn DidResolver>) -> Self {
        self.register(resolver);
        self
    }

    /// Register a resolver, keyed by its own `method()`. A later registration
    /// for the same method replaces the earlier one.
    pub fn register(&mut self, resolver: Box<dyn DidResolver>) {
        self.resolvers.insert(resolver.method(), resolver);
    }

    /// The set of registered method names.
    pub fn methods(&self) -> Vec<&'static str> {
        self.resolvers.keys().copied().collect()
    }

    /// Resolve a DID by routing to the resolver for its method. If no resolver
    /// is registered, returns `MethodNotSupported` with the standard error code.
    pub async fn resolve(&self, did: &Did) -> Result<DidResolutionResult, DidResolutionError> {
        match self.resolvers.get(did.method()) {
            Some(resolver) => resolver.resolve(did).await,
            None => Err(DidResolutionError::MethodNotSupported(
                did.method().to_string(),
            )),
        }
    }

    /// Convenience: parse a DID string then resolve it. A syntactically invalid
    /// string yields `InvalidDid` (standard error code).
    pub async fn resolve_str(&self, did: &str) -> Result<DidResolutionResult, DidResolutionError> {
        let parsed = Did::parse(did).map_err(|e| DidResolutionError::InvalidDid(e.to_string()))?;
        self.resolve(&parsed).await
    }
}
