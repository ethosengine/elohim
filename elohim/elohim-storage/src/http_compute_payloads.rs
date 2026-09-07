//! Local attachment facing. The only cross-peer traffic here is the existing
//! libp2p/iroh blob race; compute payloads never finalize into permanent content.
use super::*;
use crate::compute_payload_store as payload;
use http_body_util::Limited;

impl HttpServer {
    pub(super) async fn handle_compute_payload(
        &self,
        req: Request<Incoming>,
        cid: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if std::env::var("ELOHIM_COMPUTE_LOCAL_API").as_deref() != Ok("1") {
            return Ok(response::not_found("compute local API disabled"));
        }
        if !crate::api::compute_tasks::local_token_authorized(req.headers()) {
            return Ok(response::forbidden(
                &serde_json::json!({"reason":"local-compute-capability-required"}),
            ));
        }
        let owner = req
            .headers()
            .get("x-compute-owner")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        if owner.is_empty() || owner.len() > 256 {
            return Ok(response::bad_request("x-compute-owner required"));
        }
        let ttl = match req.headers().get("x-compute-retain-seconds") {
            Some(v) => match v.to_str().ok().and_then(|s| s.parse::<u64>().ok()) {
                Some(t) => t,
                _ => return Ok(response::bad_request("invalid payload retention seconds")),
            },
            None => 86400,
        };
        let hex = match BlobStore::parse_content_address(cid) {
            Ok(h) if h.len() == 64 && cid.starts_with("bafk") => h,
            _ => return Ok(response::bad_request("raw CID required")),
        };
        let root = self.blob_store.root_dir();
        if req.method() == Method::DELETE {
            payload::release(root, cid, &owner).await?;
            return Ok(response::ok(&serde_json::json!({"released":true})));
        }
        let bytes = if req.method() == Method::PUT {
            Limited::new(req.into_body(), payload::CHUNK_LIMIT)
                .collect()
                .await
                .map_err(|_| StorageError::InvalidInput("compute chunk exceeds 1 MiB".into()))?
                .to_bytes()
                .to_vec()
        } else if req.method() == Method::GET {
            let owner_expiry = payload::owner_expiry(root, cid, &owner).await;
            if owner_expiry.is_some_and(|expiry| expiry <= payload::now()) {
                return Ok(Response::builder()
                    .status(StatusCode::GONE)
                    .body(Full::new(Bytes::from("compute payload expired")))
                    .unwrap());
            }
            if let Ok(bytes) = payload::get(root, cid).await {
                // Reading a retained copy does not slide its expiry window.
                if owner_expiry.is_none() {
                    payload::put(root, cid, &owner, ttl, &bytes).await?;
                }
                return Ok(Response::builder()
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .header(header::CACHE_CONTROL, "no-store")
                    .body(Full::new(Bytes::from(bytes)))
                    .unwrap());
            }
            match self.fetch_compute_chunk(&format!("sha256-{hex}")).await {
                Some(bytes) => bytes,
                None => {
                    return Ok(response::service_unavailable(
                        "compute payload not yet available from native peers",
                    ))
                }
            }
        } else {
            return Ok(response::method_not_allowed());
        };
        payload::put(root, cid, &owner, ttl, &bytes).await?;
        // This lease is operational custody, not permanent inventory. Native
        // peers can request the CID directly; expiry leaves no stale self-advertisement.
        Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CACHE_CONTROL, "no-store")
            .body(Full::new(Bytes::from(bytes)))
            .unwrap())
    }

    async fn fetch_compute_chunk(&self, hash: &str) -> Option<Vec<u8>> {
        // Bounded chunks allow reuse of the native blob protocol without the
        // permanent-content finalizer. Eight candidates, two in flight each.
        #[cfg(feature = "p2p")]
        {
            let connected: std::collections::HashSet<String> = match self.p2p_handle.as_ref() {
                Some(handle) => handle
                    .list_peers()
                    .await
                    .into_iter()
                    .map(|p| p.peer_id)
                    .collect(),
                None => Default::default(),
            };
            let mut candidates: Vec<String> = connected.iter().cloned().collect();
            #[cfg(feature = "p2p-iroh")]
            if let Some(leg) = crate::p2p_iroh::iroh_fetch_leg() {
                let me = leg.endpoint().node_id();
                candidates.extend(leg.book().snapshot(Some(&me)).into_iter().map(|e| {
                    e.libp2p_peer_id
                        .or(e.agent_cid)
                        .unwrap_or_else(|| e.addr.node_id.to_string())
                }));
            }
            candidates.sort();
            candidates.dedup();
            // Rotate the bounded fallback so a larger household cannot starve
            // peers whose IDs sort after the first eight.
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            if !candidates.is_empty() {
                let offset =
                    NEXT.fetch_add(8, std::sync::atomic::Ordering::Relaxed) % candidates.len();
                candidates.rotate_left(offset);
            }
            candidates.truncate(8);
            let sender = match self.p2p_handle.as_ref() {
                Some(h) => h.command_sender(),
                None => {
                    let (tx, _rx) = tokio::sync::mpsc::channel(1);
                    tx
                }
            };
            if let crate::p2p::blob_fetch::FetchOutcome::Hit { bytes, .. } =
                crate::p2p::blob_swarm::race_fetch_dual(
                    hash,
                    candidates,
                    &sender,
                    |p| connected.contains(p),
                    2,
                    std::time::Duration::from_secs(5),
                )
                .await
            {
                if bytes.len() <= payload::CHUNK_LIMIT {
                    return Some(bytes);
                }
            }
        }
        let _ = hash;
        None
    }
}
