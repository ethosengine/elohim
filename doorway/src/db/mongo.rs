//! MongoDB client and collection wrapper
//!
//! Pattern adapted from holo-host/rust/util_libs/db/src/mongodb

use bson::{doc, oid::ObjectId, DateTime, Document};
use mongodb::{
    options::{IndexOptions, UpdateModifications},
    results::UpdateResult,
    Client, Collection, IndexModel,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use tracing::{error, info};

use crate::db::schemas::Metadata;
use crate::types::DoorwayError;

/// Trait for schemas that provide index definitions
pub trait IntoIndexes {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)>;
}

/// Trait for schemas with mutable metadata
pub trait MutMetadata {
    fn mut_metadata(&mut self) -> &mut Metadata;
}

/// MongoDB client wrapper
#[derive(Clone)]
pub struct MongoClient {
    client: Client,
    db_name: String,
}

impl MongoClient {
    /// Create a new MongoDB client
    pub async fn new(uri: &str, db_name: &str) -> Result<Self, DoorwayError> {
        info!("Connecting to MongoDB at {}", uri);

        // Use serverSelectionTimeoutMS to avoid hanging on unreachable MongoDB
        let timeout_uri = if uri.contains('?') {
            format!("{}&serverSelectionTimeoutMS=3000&connectTimeoutMS=3000", uri)
        } else {
            format!("{}?serverSelectionTimeoutMS=3000&connectTimeoutMS=3000", uri)
        };

        let client = Client::with_uri_str(&timeout_uri)
            .await
            .map_err(|e| DoorwayError::Database(format!("Failed to connect to MongoDB: {}", e)))?;

        // Verify connection with timeout
        client
            .database(db_name)
            .run_command(doc! { "ping": 1 })
            .await
            .map_err(|e| DoorwayError::Database(format!("MongoDB ping failed: {}", e)))?;

        info!("Connected to MongoDB database '{}'", db_name);

        Ok(Self {
            client,
            db_name: db_name.to_string(),
        })
    }

    /// Get a typed collection
    pub async fn collection<T>(&self, name: &str) -> Result<MongoCollection<T>, DoorwayError>
    where
        T: Serialize + DeserializeOwned + Unpin + Send + Sync + Default + IntoIndexes + MutMetadata,
    {
        MongoCollection::new(&self.client, &self.db_name, name).await
    }

    /// Get the raw MongoDB client
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Get the database name
    pub fn db_name(&self) -> &str {
        &self.db_name
    }
}

/// Typed MongoDB collection with automatic indexing
#[derive(Debug, Clone)]
pub struct MongoCollection<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync,
{
    inner: Collection<T>,
}

impl<T> MongoCollection<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync + Default + IntoIndexes + MutMetadata,
{
    /// Create a new collection and apply indexes
    pub async fn new(
        client: &Client,
        db_name: &str,
        collection_name: &str,
    ) -> Result<Self, DoorwayError> {
        let collection = client.database(db_name).collection::<T>(collection_name);
        let mongo_collection = MongoCollection { inner: collection };

        // Apply indexes
        mongo_collection.apply_indexes().await?;

        Ok(mongo_collection)
    }

    /// Apply schema-defined indexes
    async fn apply_indexes(&self) -> Result<(), DoorwayError> {
        let schema_indices = T::into_indices();

        if schema_indices.is_empty() {
            return Ok(());
        }

        let indices: Vec<IndexModel> = schema_indices
            .into_iter()
            .map(|(keys, opts)| {
                IndexModel::builder()
                    .keys(keys)
                    .options(opts)
                    .build()
            })
            .collect();

        self.inner
            .create_indexes(indices)
            .await
            .map_err(|e| DoorwayError::Database(format!("Failed to create indexes: {}", e)))?;

        Ok(())
    }

    /// Insert a document, setting metadata timestamps
    pub async fn insert_one(&self, mut item: T) -> Result<ObjectId, DoorwayError> {
        let metadata = item.mut_metadata();
        metadata.is_deleted = false;
        metadata.created_at = Some(DateTime::now());
        metadata.updated_at = Some(DateTime::now());

        let result = self
            .inner
            .insert_one(item)
            .await
            .map_err(|e| DoorwayError::Database(format!("Insert failed: {}", e)))?;

        result
            .inserted_id
            .as_object_id()
            .ok_or_else(|| DoorwayError::Database("Failed to get inserted ID".into()))
    }

    /// Find one document by filter
    pub async fn find_one(&self, filter: Document) -> Result<Option<T>, DoorwayError> {
        // Add is_deleted check
        let mut full_filter = filter;
        full_filter.insert("metadata.is_deleted", doc! { "$ne": true });

        self.inner
            .find_one(full_filter)
            .await
            .map_err(|e| DoorwayError::Database(format!("Find failed: {}", e)))
    }

    /// Find many documents by filter
    pub async fn find_many(&self, filter: Document) -> Result<Vec<T>, DoorwayError> {
        use futures_util::StreamExt;

        // Add is_deleted check
        let mut full_filter = filter;
        full_filter.insert("metadata.is_deleted", doc! { "$ne": true });

        let cursor = self
            .inner
            .find(full_filter)
            .await
            .map_err(|e| DoorwayError::Database(format!("Find failed: {}", e)))?;

        let results: Vec<T> = cursor
            .filter_map(|doc| async {
                match doc {
                    Ok(d) => Some(d),
                    Err(e) => {
                        error!("Error reading document: {}", e);
                        None
                    }
                }
            })
            .collect()
            .await;

        Ok(results)
    }

    /// Update one document
    pub async fn update_one(
        &self,
        filter: Document,
        update: impl Into<UpdateModifications>,
    ) -> Result<UpdateResult, DoorwayError> {
        // Add updated_at to the update
        let modifications = update.into();

        self.inner
            .update_one(filter, modifications)
            .await
            .map_err(|e| DoorwayError::Database(format!("Update failed: {}", e)))
    }

    /// Soft delete a document
    pub async fn soft_delete(&self, filter: Document) -> Result<UpdateResult, DoorwayError> {
        let update = doc! {
            "$set": {
                "metadata.is_deleted": true,
                "metadata.deleted_at": DateTime::now(),
                "metadata.updated_at": DateTime::now(),
            }
        };

        self.update_one(filter, update).await
    }

    /// Get the underlying collection for advanced operations
    pub fn inner(&self) -> &Collection<T> {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require a running MongoDB instance
    // See docker-compose.dev.yml for local testing
}
