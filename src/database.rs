use anyhow::Result;
use futures_util::StreamExt;
use log::error;
use mongodb::{Client, Collection, bson::{DateTime, doc}};
use rand::{Rng, distr::Alphanumeric};
use serde::{Deserialize, Serialize};

use crate::{environment::{AS_MONGODB_DATABASE, FILE_TIMEOUT_HOURS, MONGODB_DATABASE}, get_time_millis};

use crate::environment::MONGODB_URI;

use once_cell::sync::OnceCell;

pub static DATABASE: OnceCell<Client> = OnceCell::new();

pub async fn connect() {
    let client = Client::with_uri_str(&*MONGODB_URI)
        .await
        .expect("Failed to connect to MongoDB");
    DATABASE.set(client).expect("Failed to set MongoDB client");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDocument {
    pub id: String,
    pub name: Option<String>,
    pub content_type: String,
    pub size: u64,
    pub uploaded_at: DateTime,
    pub user_id: String,
    pub signing_key: String,
    
    // This attribute should be set by other applications
    // If false for too long, the file will be deleted
    pub linked: bool,
    pub linked_at: Option<DateTime>,

    // This attribute indicates if the file not to be served
    // even if it exists in the database and storage
    // (e.g., flagged for abuse)
    pub hidden: bool,
}

impl FileDocument {
    pub fn new(
        id: String,
        name: Option<String>,
        content_type: String,
        size: u64,
        user_id: String,
    ) -> Self {        
        let secret_key: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        Self {
            id,
            name,
            content_type,
            size,
            uploaded_at: DateTime::now(),
            user_id,
            signing_key: secret_key,
            linked: false,
            linked_at: None,
            hidden: false,
        }
    }
}

#[derive(Clone)]
pub struct FileRepository {}

impl FileRepository {
    pub fn get_collection() -> Collection<FileDocument> {
        let client = DATABASE.get().expect("MongoDB client not initialized");
        let db = client.database(&MONGODB_DATABASE);
        db.collection::<FileDocument>("files")
    }

    pub async fn insert_file(file: FileDocument) -> Result<()> {
        Self::get_collection().insert_one(file).await?;
        Ok(())
    }

    pub async fn get_file(id: &str) -> Result<Option<FileDocument>> {
        let result = Self::get_collection().find_one(doc! { "id": id }).await?;
        Ok(result)
    }

    pub async fn find_expired_files() -> Result<Vec<FileDocument>> {        
        let cutoff_time = get_time_millis() as i64 - &*FILE_TIMEOUT_HOURS * 3600 * 1000;
        let cutoff = DateTime::from_millis(cutoff_time);
        let filter = doc! {
            "$or": [
                {
                    "linked": false,
                    "uploaded_at": { "$lt": cutoff }
                },
                {
                    "linked": false,
                    "linked_at": { "$lt": cutoff }
                },
            ]
        };
        let mut cursor = Self::get_collection().find(filter).await?;
        let mut expired_files = Vec::new();
        while let Some(result) = cursor.next().await {
            match result {
                Ok(file) => expired_files.push(file),
                Err(e) => error!("Error reading file document: {}", e),
            }
        }
        Ok(expired_files)
    }

    pub async fn delete_file(id: &str) -> Result<()> {
        Self::get_collection().delete_one(doc! { "id": id }).await?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Session {
    pub id: String,
    pub token: String,
    pub friendly_name: String,
    pub user_id: String,
    pub expires_at: u64,
}

pub async fn get_session(token: &str) -> Result<Option<Session>> {
    let client = DATABASE.get().expect("MongoDB client not initialized");
    let db = client.database(&AS_MONGODB_DATABASE);
    let collection = db.collection::<Session>("sessions");
    let result = collection.find_one(doc! { "token": token }).await?;
    Ok(result)
}
