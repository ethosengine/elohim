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
        use crate::api::hosted_custody::{classify_payload_caller, PayloadCaller};
        if std::env::var("ELOHIM_COMPUTE_LOCAL_API").as_deref() != Ok("1") {
            return Ok(response::not_found("compute local API disabled"));
        }
        // Two arms that never mix: the node-local token (no performer), or a
        // doorway-verified performer under this peer's hosted-cell promise.
        // A present performer never consults the token.
        let performer = req
            .headers()
            .get(crate::api::operator_verbs::VERIFIED_PERFORMER_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let caller = classify_payload_caller(
            performer.as_deref(),
            crate::api::compute_tasks::local_token_authorized(req.headers()),
        );
        if caller == PayloadCaller::Refused {
            return Ok(response::forbidden(
                &serde_json::json!({"reason":"local-compute-capability-required"}),
            ));
        }
        let client_owner = req
            .headers()
            .get("x-compute-owner")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        if client_owner.is_empty() || client_owner.len() > 256 {
            return Ok(response::bad_request("x-compute-owner required"));
        }
        let mut ttl = match req.headers().get("x-compute-retain-seconds") {
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
        let owner = match caller {
            PayloadCaller::Hosted(performer) => {
                ttl = crate::api::hosted_custody::hosted_retention(ttl);
                match self
                    .admit_hosted_custody(&req, &performer, &client_owner, cid)
                    .await
                {
                    Ok(owner) => owner,
                    Err(refusal) => return Ok(refusal),
                }
            }
            _ => client_owner,
        };
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

    /// The hosted arm (`api::hosted_custody`): admit a performer under THIS
    /// peer's active `hosted-cell` grant, decide which lease it acts under, and
    /// record the use before acting. Returns the runner-namespaced owner, or the
    /// refusal response.
    async fn admit_hosted_custody(
        &self,
        req: &Request<Incoming>,
        performer: &str,
        client_owner: &str,
        cid: &str,
    ) -> Result<String, Response<Full<Bytes>>> {
        use crate::api::hosted_custody as custody;
        let refuse = |reason: &str| response::forbidden(&serde_json::json!({ "reason": reason }));
        // The reach the runner declares for the bytes it stocks; absent ⇒ the
        // hosting grant's own reach.
        let declared_reach = req
            .headers()
            .get("x-compute-reach")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let Some(pool) = self.db_pool.as_ref() else {
            return Err(response::service_unavailable("db pool not available"));
        };
        let Some(hc) = self.hc_registry.as_ref().and_then(|r| r.lamad_client()) else {
            return Err(response::service_unavailable("local conductor unavailable"));
        };
        let native = custody::HcCustodyNative(hc);
        let now = chrono::Utc::now().to_rfc3339();
        let grant_cid =
            custody::authorize_custody(pool, &native, performer, &now, declared_reach.as_deref())
                .await
                .map_err(|r| match r.grant_reach() {
                    Some(reach) => response::forbidden(
                        &serde_json::json!({ "reason": r.reason(), "grantReach": reach }),
                    ),
                    None => refuse(r.reason()),
                })?;
        let owner = custody::hosted_owner(performer, client_owner)
            .ok_or_else(|| response::bad_request("x-compute-owner too long for hosted custody"))?;
        // A read of a chunk this runner does not lease is only ever its own
        // task's OUTPUT, named by the task's request action hash.
        if req.method() == Method::GET
            && payload::owner_expiry(self.blob_store.root_dir(), cid, &owner)
                .await
                .is_none()
        {
            let request = req
                .headers()
                .get(custody::REQUEST_HEADER)
                .and_then(|v| v.to_str().ok())
                .filter(|r| !r.is_empty())
                .ok_or_else(|| refuse("not-a-chunk-this-runner-holds"))?;
            let task = custody::CustodyNative::task(&native, request)
                .await
                .map_err(|_| refuse("task-unreadable"))?;
            if !custody::task_output_admits(&task, performer, client_owner, cid) {
                return Err(refuse("not-an-output-of-this-runners-task"));
            }
        }
        // Emit-then-act: an unrecorded use is never performed.
        custody::record_custody_use(
            pool,
            &custody::CustodyNative::own_key(&native),
            performer,
            &grant_cid,
            &now,
        )
        .map_err(|_| response::service_unavailable("custody use could not be recorded"))?;
        Ok(owner)
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
