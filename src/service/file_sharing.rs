// ============================================================================
// File Sharing Service - Main Orchestrator with Database
// ============================================================================

use crate::crypto::hash::HashValue;
use crate::crypto::commitment::Commitment;
use crate::core::file_metadata::FileMetadata;
use crate::storage::engine::StorageEngine;
use crate::auth::authenticator::FileAuthenticator;
use crate::db::{Database, User, FileRecord, SharedFile, SystemStats};
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::Path;
use sha2::{Sha256, Digest};

pub struct FileSharingService {
    pub storage: StorageEngine,
    pub authenticator: FileAuthenticator,
    pub database: Database,
    pub current_user: Option<User>,
    users: HashMap<String, User>, // Cache
    _shares: HashMap<String, Vec<crate::db::models::SharedFile>>, // Cache with underscore
}

impl FileSharingService {
    pub async fn new(storage_path: &Path, watch_path: &Path, database: Database) -> Result<Self> {
        Ok(Self {
            storage: StorageEngine::new(storage_path)?,
            authenticator: FileAuthenticator::new(watch_path),
            database,
            current_user: None,
            users: HashMap::new(),
            _shares: HashMap::new(),
        })
    }
    
    pub async fn register_user(&mut self, username: &str, password: &str, email: Option<&str>) -> Result<User> {
        // Hash password (in production, use proper password hashing like bcrypt)
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let password_hash = hex::encode(hasher.finalize());
        
        let user = self.database.create_user(username, &password_hash, email).await?;
        self.users.insert(username.to_string(), user.clone());
        println!("👤 User registered: {}", username);
        Ok(user)
    }
    
    pub async fn login(&mut self, username: &str, password: &str) -> Result<Option<User>> {
        let user_opt = self.database.get_user_by_username(username).await?;
        
        if let Some(user) = user_opt {
            // Verify password
            let mut hasher = Sha256::new();
            hasher.update(password.as_bytes());
            let password_hash = hex::encode(hasher.finalize());
            
            if user.password_hash == password_hash {
                self.current_user = Some(user.clone());
                self.database.update_last_login(user.id).await?;
                println!(" User logged in: {}", username);
                return Ok(Some(user));
            }
        }
        
        Ok(None)
    }
    
    pub fn logout(&mut self) {
        self.current_user = None;
        println!(" User logged out");
    }
    
    pub async fn upload_file(
        &mut self, 
        data: &[u8], 
        filename: &str, 
        owner: &str,
        description: Option<&str>,
    ) -> Result<FileMetadata> {
        // Get user from database
        let user = self.database.get_user_by_username(owner).await?
            .context("User not found")?;
        
        // Store file in storage engine
        let metadata = self.storage.store_file(data, filename, owner)?;
        
        // Save to database
        self.database.save_file(
            &metadata.hash,
            filename,
            metadata.size,
            user.id,
            description,
            metadata.chunks.len(),
            &metadata.merkle_root,
        ).await?;
        
        // Register with authenticator
        let temp_path = self.authenticator.watch_dir.join(filename);
        std::fs::write(&temp_path, data)?;
        self.authenticator.register(&temp_path)?;
        
        Ok(metadata)
    }
    
    pub async fn share_file(&mut self, file_hash: &HashValue, owner: &str, target: &str) -> Result<()> {
        // Get users
        let owner_user = self.database.get_user_by_username(owner).await?
            .context("Owner not found")?;
        let target_user = self.database.get_user_by_username(target).await?
            .context("Target user not found")?;
        
        // Get file
        let file = self.database.get_file_by_hash(file_hash).await?
            .context("File not found")?;
        
        // Create commitment
        let commitment = Commitment::commit(file_hash.bytes.as_slice());
        let commitment_bytes = bincode::serialize(&commitment)?;
        
        // Save to database
        self.database.create_share(
            file.id,
            owner_user.id,
            target_user.id,
            Some(&commitment_bytes),
            None, // No expiration
        ).await?;
        
        println!("🔗 File shared: {} -> {}", owner, target);
        Ok(())
    }
    
    pub async fn download_and_verify(&self, file_hash: &HashValue) -> Result<Vec<u8>> {
        let data = self.storage.retrieve_file(file_hash)?;
        println!(" File verified: {} integrity check passed", file_hash.prefix(8));
        Ok(data)
    }
    
    pub async fn get_user_files(&self, username: &str) -> Result<Vec<FileRecord>> {
        self.database.get_user_files(username).await
    }
    
    pub async fn get_shared_files(&self, username: &str) -> Result<Vec<SharedFile>> {
        self.database.get_shared_files(username).await
    }
    
    pub async fn verify_file_integrity(&self, file_hash: &HashValue) -> Result<bool> {
        match self.storage.retrieve_file(file_hash) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    pub async fn get_system_stats(&self) -> Result<SystemStats> {
        let mut stats = self.database.get_system_stats().await?;
        stats.bloom_fp_rate = self.authenticator.bloom.false_positive_rate();
        Ok(stats)
    }
}