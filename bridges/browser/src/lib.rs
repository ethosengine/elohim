//! Native messaging host adapter. No browser extension API in the protocol contract.
//! The host supplies the actual caller origin out-of-band and authenticates its session;
//! the peer runtime checks the operation grant. Neither is replaced by an origin allow-list.
use elohim_sdk::{
    lifecycle::{AppInstanceView, AppOpenInput, AppRuntime},
    Result, SdkError,
};
use url::Url;

pub struct BrowserBridge {
    allowed_origin: String,
}

impl BrowserBridge {
    pub fn new(allowed_origin: &str) -> Result<Self> {
        Ok(Self {
            allowed_origin: origin(allowed_origin)?,
        })
    }

    /// Caller origin MUST come from the embedding transport, not the request body.
    /// This bridge performs no session issuance or resource authorization itself.
    pub async fn handle_request(
        &self,
        caller_origin: &str,
        runtime: &dyn AppRuntime,
        request: &AppOpenInput,
    ) -> Result<AppInstanceView> {
        if origin(caller_origin)? != self.allowed_origin {
            return Err(SdkError::Config("browser origin is not admitted".into()));
        }
        runtime.ensure_instance(request).await
    }
}

fn origin(value: &str) -> Result<String> {
    let url = Url::parse(value).map_err(|_| SdkError::Config("invalid browser origin".into()))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(SdkError::Config(
            "browser origin must be an HTTPS origin without credentials".into(),
        ));
    }
    Ok(url.origin().ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_sdk::lifecycle::{AppReleaseView, UnsupportedLifecycle};
    fn request() -> AppOpenInput {
        AppOpenInput {
            release: AppReleaseView {
                app_id: "orchard".into(),
                release_cid: "fixture".into(),
                target: "browser".into(),
                artifact_cids: vec![],
                required_capabilities: vec![],
            },
            operation_id: "open".into(),
            grant_ref: "not-authority".into(),
        }
    }
    #[tokio::test]
    async fn admitted_origin_does_not_create_runtime_authority() {
        let bridge = BrowserBridge::new("https://apps.example").unwrap();
        assert!(matches!(
            bridge
                .handle_request("https://apps.example", &UnsupportedLifecycle, &request())
                .await,
            Err(SdkError::Unsupported(_))
        ));
        assert!(matches!(
            bridge
                .handle_request("https://evil.example", &UnsupportedLifecycle, &request())
                .await,
            Err(SdkError::Config(_))
        ));
    }
    #[test]
    fn origins_reject_credentials_paths_and_opaque_or_insecure_schemes() {
        for origin in [
            "null",
            "file:///x",
            "http://apps.example",
            "https://user:secret@apps.example",
            "https://apps.example/path",
            "https://apps.example#secret",
            "https://apps.example?token=secret",
        ] {
            assert!(BrowserBridge::new(origin).is_err(), "{origin}");
        }
    }
}
