// ============================================================================
// Database Models
// ============================================================================

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub public_key: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: i64,
    pub hash: String,
    pub filename: String,
    pub size: i64,
    pub owner_id: i64,
    pub description: Option<String>,
    pub chunks: i32,
    pub merkle_root: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SharedFile {
    pub id: i64,
    pub file_id: i64,
    pub filename: String,
    pub shared_by: String,
    pub shared_with_id: i64,
    pub shared_with_username: String,
    pub commitment: Option<Vec<u8>>,
    pub shared_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub total_users: i64,
    pub total_files: i64,
    pub unique_files: i64,
    pub total_shares: i64,
    pub total_bytes: i64,
    pub saved_bytes: i64,
    pub dedup_rate: f64,
    pub bloom_fp_rate: f64,
}