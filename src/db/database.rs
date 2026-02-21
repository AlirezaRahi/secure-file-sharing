// ============================================================================
// Database Connection and Operations
// ============================================================================

use anyhow::{Result, Context, anyhow};
use chrono::{DateTime, Utc};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, Row};
use dotenv::dotenv;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::models::{User, FileRecord, SharedFile, SystemStats};
use crate::crypto::hash::HashValue;

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self> {
    dotenv().ok();
    
    // ساخت پوشه data تو مسیر جاری
    let data_dir = Path::new("./data");
    println!("Creating data directory: {:?}", data_dir);
    
    if !data_dir.exists() {
        fs::create_dir_all(data_dir)
            .context("Failed to create data directory")?;
        println!("✅ Data directory created");
    } else {
        println!("✅ Data directory already exists");
    }
    

    let db_path = data_dir.join("secure_files.db");
    let db_path_str = db_path.to_str()
        .ok_or_else(|| anyhow!("Invalid database path"))?;
    
    println!("Database path: {}", db_path_str);
    

    let test_file = data_dir.join("test_write.tmp");
    match fs::File::create(&test_file) {
        Ok(_) => {
            println!("✅ Data directory is writable");
            let _ = fs::remove_file(test_file);
        },
        Err(e) => {
            println!("❌ Data directory is NOT writable: {}", e);
            return Err(anyhow!("Data directory not writable: {}", e));
        }
    }
    

    let database_url = format!("sqlite:{}", db_path_str);
    println!("Connection URL: {}", database_url);
    

    match SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await 
    {
        Ok(pool) => {
            println!("✅ Database connected successfully!");
            
        
            match Self::init_schema(&pool).await {
                Ok(_) => println!("✅ Database schema initialized"),
                Err(e) => println!("⚠️ Schema initialization warning: {}", e),
            }
            
            Ok(Self { pool })
        },
        Err(e) => {
            println!("❌ Database connection failed!");
            println!("❌ Error type: {:?}", e);
            println!("❌ Error details: {}", e);
            
    
            println!("🔄 Trying in-memory database as fallback...");
            
            let memory_pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .context("Failed to connect to in-memory database")?;
            
            println!("✅ Connected to in-memory database!");
            Self::init_schema(&memory_pool).await?;
            println!("✅ In-memory schema initialized");
            
            Ok(Self { pool: memory_pool })
        }
    }
}
    
    async fn init_schema(pool: &SqlitePool) -> Result<()> {
        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                email TEXT,
                public_key BLOB,
                created_at DATETIME NOT NULL,
                last_login DATETIME
            )
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to create users table")?;
        
        // Create files table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash TEXT NOT NULL,
                filename TEXT NOT NULL,
                size INTEGER NOT NULL,
                owner_id INTEGER NOT NULL,
                description TEXT,
                chunks INTEGER NOT NULL,
                merkle_root TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                FOREIGN KEY (owner_id) REFERENCES users(id),
                UNIQUE(hash, owner_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to create files table")?;
        
        // Create shares table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS shares (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id INTEGER NOT NULL,
                shared_by_id INTEGER NOT NULL,
                shared_with_id INTEGER NOT NULL,
                commitment BLOB,
                shared_at DATETIME NOT NULL,
                expires_at DATETIME,
                FOREIGN KEY (file_id) REFERENCES files(id),
                FOREIGN KEY (shared_by_id) REFERENCES users(id),
                FOREIGN KEY (shared_with_id) REFERENCES users(id),
                UNIQUE(file_id, shared_with_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .context("Failed to create shares table")?;
        
        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash)")
            .execute(pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_owner ON files(owner_id)")
            .execute(pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_shares_with ON shares(shared_with_id)")
            .execute(pool)
            .await?;
        
        Ok(())
    }
    
    // بقیه متدها مثل قبل...
    pub async fn create_user(&self, username: &str, password_hash: &str, email: Option<&str>) -> Result<User> {
        let now = Utc::now();
        
        let id = sqlx::query(
            r#"
            INSERT INTO users (username, password_hash, email, created_at)
            VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(username)
        .bind(password_hash)
        .bind(email)
        .bind(now)
        .fetch_one(&self.pool)
        .await?
        .get(0);
        
        Ok(User {
            id,
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            email: email.map(|s| s.to_string()),
            public_key: None,
            created_at: now,
            last_login: None,
        })
    }
    
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, password_hash, email, public_key, created_at, last_login
            FROM users
            WHERE username = ?
            "#
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(user)
    }
    
    pub async fn update_last_login(&self, user_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET last_login = ?
            WHERE id = ?
            "#
        )
        .bind(Utc::now())
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn save_file(
        &self, 
        hash: &HashValue, 
        filename: &str, 
        size: u64,
        owner_id: i64,
        description: Option<&str>,
        chunks: usize,
        merkle_root: &HashValue,
    ) -> Result<FileRecord> {
        let now = Utc::now();
        
        let id = sqlx::query(
            r#"
            INSERT INTO files (hash, filename, size, owner_id, description, chunks, merkle_root, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(hash.to_hex())
        .bind(filename)
        .bind(size as i64)
        .bind(owner_id)
        .bind(description)
        .bind(chunks as i32)
        .bind(merkle_root.to_hex())
        .bind(now)
        .fetch_one(&self.pool)
        .await?
        .get(0);
        
        Ok(FileRecord {
            id,
            hash: hash.to_hex(),
            filename: filename.to_string(),
            size: size as i64,
            owner_id,
            description: description.map(|s| s.to_string()),
            chunks: chunks as i32,
            merkle_root: merkle_root.to_hex(),
            created_at: now,
        })
    }
    
    pub async fn get_user_files(&self, username: &str) -> Result<Vec<FileRecord>> {
        let files = sqlx::query_as::<_, FileRecord>(
            r#"
            SELECT f.id, f.hash, f.filename, f.size, f.owner_id, 
                f.description, f.chunks, f.merkle_root, f.created_at
            FROM files f
            JOIN users u ON f.owner_id = u.id
            WHERE u.username = ?
            ORDER BY f.created_at DESC
            "#
        )
        .bind(username)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(files)
    }
    
    pub async fn get_file_by_hash(&self, hash: &HashValue) -> Result<Option<FileRecord>> {
        let file = sqlx::query_as::<_, FileRecord>(
            r#"
            SELECT id, hash, filename, size, owner_id, description, chunks, merkle_root, created_at
            FROM files
            WHERE hash = ?
            "#
        )
        .bind(hash.to_hex())
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(file)
    }
    
    pub async fn create_share(
        &self,
        file_id: i64,
        shared_by_id: i64,
        shared_with_id: i64,
        commitment: Option<&[u8]>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO shares (file_id, shared_by_id, shared_with_id, commitment, shared_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(file_id)
        .bind(shared_by_id)
        .bind(shared_with_id)
        .bind(commitment)
        .bind(Utc::now())
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn get_shared_files(&self, username: &str) -> Result<Vec<SharedFile>> {
        let shares = sqlx::query_as::<_, SharedFile>(
            r#"
            SELECT 
                s.id,
                f.id as file_id,
                f.filename,
                u_sender.username as shared_by,
                s.shared_with_id,
                u_receiver.username as shared_with_username,
                s.commitment,
                s.shared_at,
                s.expires_at
            FROM shares s
            JOIN files f ON s.file_id = f.id
            JOIN users u_sender ON s.shared_by_id = u_sender.id
            JOIN users u_receiver ON s.shared_with_id = u_receiver.id
            WHERE u_receiver.username = ?
            ORDER BY s.shared_at DESC
            "#
        )
        .bind(username)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(shares)
    }
    
    pub async fn get_system_stats(&self) -> Result<SystemStats> {
        // Get user count
        let total_users: i64 = sqlx::query("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        
        // Get total files
        let total_files: i64 = sqlx::query("SELECT COUNT(*) FROM files")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        
        // Get unique files (by hash)
        let unique_files: i64 = sqlx::query("SELECT COUNT(DISTINCT hash) FROM files")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        
        // Get total shares
        let total_shares: i64 = sqlx::query("SELECT COUNT(*) FROM shares")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        
        // Get total bytes
        let total_bytes: i64 = sqlx::query("SELECT COALESCE(SUM(size), 0) FROM files")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        
        // Calculate saved bytes (deduplication)
        let saved_bytes = if total_files > unique_files {
            let avg_size: f64 = sqlx::query("SELECT COALESCE(AVG(size), 0) FROM files")
                .fetch_one(&self.pool)
                .await?
                .get(0);
            ((total_files - unique_files) as f64 * avg_size) as i64
        } else {
            0
        };
        
        let dedup_rate = if total_bytes > 0 {
            (saved_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };
        
        Ok(SystemStats {
            total_users,
            total_files,
            unique_files,
            total_shares,
            total_bytes,
            saved_bytes,
            dedup_rate,
            bloom_fp_rate: 0.01,
        })
    }
}