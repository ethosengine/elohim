//! Host registry for persistent storage and discovery
//!
//! Manages host registration in MongoDB and provides lookup operations.

use bson::{doc, oid::ObjectId, DateTime};
use tracing::{info, warn};

use crate::db::schemas::{HostDoc, HostStatus, HOST_COLLECTION};
use crate::db::{MongoClient, MongoCollection};
use crate::nats::HostRouter;
use crate::types::DoorwayError;

/// Host registry backed by MongoDB
pub struct HostRegistry {
    collection: MongoCollection<HostDoc>,
    router: HostRouter,
}

impl HostRegistry {
    /// Create a new host registry
    pub async fn new(mongo: &MongoClient, router: HostRouter) -> Result<Self, DoorwayError> {
        let collection = mongo.collection::<HostDoc>(HOST_COLLECTION).await?;
        Ok(Self { collection, router })
    }

    /// Register a new host or update existing
    pub async fn register(&self, host: HostDoc) -> Result<ObjectId, DoorwayError> {
        let node_id = host.node_id.clone();

        // Check if host already exists
        if let Some(existing) = self
            .collection
            .find_one(doc! { "node_id": &node_id })
            .await?
        {
            // Update existing host
            self.collection
                .update_one(
                    doc! { "_id": existing._id },
                    doc! {
                        "$set": {
                            "status": "online",
                            "last_heartbeat": DateTime::now(),
                            "conductor_url": &host.conductor_url,
                            "host_url": &host.host_url,
                            "app_port_min": host.app_port_min as i32,
                            "app_port_max": host.app_port_max as i32,
                            "version": &host.version,
                            "metadata.updated_at": DateTime::now(),
                        }
                    },
                )
                .await?;

            info!("Updated existing host: {}", node_id);

            // Update router cache
            let mut updated = host;
            updated._id = existing._id;
            self.router.register_host(updated).await;

            existing
                ._id
                .ok_or_else(|| DoorwayError::Database("Missing host ID".into()))
        } else {
            // Insert new host
            let id = self.collection.insert_one(host.clone()).await?;
            info!("Registered new host: {}", node_id);

            // Add to router cache
            let mut registered = host;
            registered._id = Some(id);
            self.router.register_host(registered).await;

            Ok(id)
        }
    }

    /// Deregister a host
    pub async fn deregister(&self, node_id: &str) -> Result<bool, DoorwayError> {
        let result = self
            .collection
            .update_one(
                doc! { "node_id": node_id },
                doc! {
                    "$set": {
                        "status": "deregistered",
                        "metadata.updated_at": DateTime::now(),
                    }
                },
            )
            .await?;

        if result.modified_count > 0 {
            info!("Deregistered host: {}", node_id);
            self.router.deregister_host(node_id).await;
            Ok(true)
        } else {
            warn!("Host not found for deregistration: {}", node_id);
            Ok(false)
        }
    }

    /// Update host status
    pub async fn update_status(
        &self,
        node_id: &str,
        status: HostStatus,
    ) -> Result<bool, DoorwayError> {
        let status_str = match status {
            HostStatus::Online => "online",
            HostStatus::Offline => "offline",
            HostStatus::Maintenance => "maintenance",
            HostStatus::Deregistered => "deregistered",
        };

        let result = self
            .collection
            .update_one(
                doc! { "node_id": node_id },
                doc! {
                    "$set": {
                        "status": status_str,
                        "metadata.updated_at": DateTime::now(),
                    }
                },
            )
            .await?;

        if result.modified_count > 0 {
            self.router.update_status(node_id, status).await;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Record a heartbeat from a host
    pub async fn heartbeat(
        &self,
        node_id: &str,
        active_connections: i32,
    ) -> Result<bool, DoorwayError> {
        let result = self
            .collection
            .update_one(
                doc! { "node_id": node_id },
                doc! {
                    "$set": {
                        "status": "online",
                        "active_connections": active_connections,
                        "last_heartbeat": DateTime::now(),
                        "metadata.updated_at": DateTime::now(),
                    }
                },
            )
            .await?;

        if result.modified_count > 0 {
            self.router
                .handle_heartbeat(node_id, active_connections)
                .await;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get a host by node ID
    pub async fn get(&self, node_id: &str) -> Result<Option<HostDoc>, DoorwayError> {
        self.collection.find_one(doc! { "node_id": node_id }).await
    }

    /// List all registered hosts
    pub async fn list(&self) -> Result<Vec<HostDoc>, DoorwayError> {
        self.collection.find_many(doc! {}).await
    }

    /// List online hosts
    pub async fn list_online(&self) -> Result<Vec<HostDoc>, DoorwayError> {
        self.collection.find_many(doc! { "status": "online" }).await
    }

    /// Get hosts by region
    pub async fn list_by_region(&self, region: &str) -> Result<Vec<HostDoc>, DoorwayError> {
        self.collection
            .find_many(doc! { "region": region, "status": "online" })
            .await
    }

    /// Get the host router for request routing
    pub fn router(&self) -> &HostRouter {
        &self.router
    }

    /// Load all online hosts into the router cache
    pub async fn load_into_router(&self) -> Result<usize, DoorwayError> {
        let hosts = self.list_online().await?;
        let count = hosts.len();

        for host in hosts {
            self.router.register_host(host).await;
        }

        info!("Loaded {} hosts into router cache", count);
        Ok(count)
    }

    /// Mark offline hosts that haven't sent heartbeat
    pub async fn mark_stale_hosts(&self, stale_threshold_secs: i64) -> Result<usize, DoorwayError> {
        let cutoff = DateTime::from_millis(
            chrono::Utc::now().timestamp_millis() - (stale_threshold_secs * 1000),
        );

        let result = self
            .collection
            .inner()
            .update_many(
                doc! {
                    "status": "online",
                    "last_heartbeat": { "$lt": cutoff }
                },
                doc! {
                    "$set": {
                        "status": "offline",
                        "metadata.updated_at": DateTime::now(),
                    }
                },
            )
            .await
            .map_err(|e| DoorwayError::Database(format!("Failed to mark stale hosts: {}", e)))?;

        if result.modified_count > 0 {
            info!("Marked {} stale hosts as offline", result.modified_count);
        }

        Ok(result.modified_count as usize)
    }
}
