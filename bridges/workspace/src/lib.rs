//! Environment translation consumed by elohim-node. A discovered URL does not
//! establish reachability or authorize its reader. No sockets, credentials or ingress creation.
use std::collections::BTreeMap;
use url::Url;

/// Explicit launch configuration, not a client-supplied proxy destination.
pub struct WorkspaceBridge {
    services: BTreeMap<String, Url>,
}

impl WorkspaceBridge {
    /// Che endpoint declarations are supplied by the workspace-runtime boundary.
    /// Only credential-free HTTPS endpoints are representable in this adapter.
    pub fn from_che_endpoints(
        endpoints: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, &'static str> {
        let mut services = BTreeMap::new();
        for (service, endpoint) in endpoints {
            let url = Url::parse(&endpoint).map_err(|_| "invalid workspace endpoint")?;
            if service.is_empty()
                || url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err("workspace endpoint must be named HTTPS without credentials or query");
            }
            if services.insert(service, url).is_some() {
                return Err("duplicate workspace service");
            }
        }
        Ok(Self { services })
    }

    /// Exact configured service lookup. Never interprets a request as a path or URL.
    pub fn handle_request(&self, service: &str) -> Option<&Url> {
        self.services.get(service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_declared_services_resolve_without_claiming_ingress_creation() {
        let bridge = WorkspaceBridge::from_che_endpoints([(
            "rakia".into(),
            "https://workspace.example/apps/rakia".into(),
        )])
        .unwrap();
        assert_eq!(
            bridge.handle_request("rakia").unwrap().path(),
            "/apps/rakia"
        );
        for unknown in ["/etc/passwd", "../rakia", "http://localhost:8090", "other"] {
            assert!(bridge.handle_request(unknown).is_none());
        }
    }
    #[test]
    fn credential_urls_and_duplicate_service_names_refuse() {
        for endpoint in [
            "file:///etc/passwd",
            "http://localhost:8090",
            "https://user:secret@workspace.example",
            "https://workspace.example#secret",
            "https://workspace.example?token=secret",
        ] {
            assert!(
                WorkspaceBridge::from_che_endpoints([("rakia".into(), endpoint.into())]).is_err()
            );
        }
        assert!(WorkspaceBridge::from_che_endpoints([
            ("rakia".into(), "https://one.example".into()),
            ("rakia".into(), "https://two.example".into())
        ])
        .is_err());
    }
}
