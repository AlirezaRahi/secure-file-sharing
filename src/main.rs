// ============================================================================
// Main CLI Application with Interactive Menu
// ============================================================================

// ============================================================================
// Main CLI Application with Interactive Menu
// ============================================================================

use anyhow::Result;
use colored::*;
use dialoguer::{Input, Password, Select};
use secure_file_sharing::{
    FileSharingService, 
    Database, 
    HashValue,
    HashAlgo,
};
use std::path::Path;
use std::fs;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n{}", "🔐 ===== SECURE FILE SHARING SYSTEM ===== 🔐".bright_green().bold());
    println!("{}", "version 2.0 - Enterprise Edition with Database\n".bright_cyan());

    // Initialize database - فقط یک بار
    let database = match Database::new().await {
        Ok(db) => db,
        Err(e) => {
            println!("{} Failed to initialize database: {}", "❌".bright_red(), e);
            println!("{} Make sure the 'data' directory is writable", "💡".bright_yellow());
            return Ok(());
        }
    };
    
    // Initialize storage paths
    let storage_path = Path::new("./data/storage");  // تغییر مسیر به زیرپوشه data
    let watch_path = Path::new("./data/watch");      // تغییر مسیر به زیرپوشه data
    
    // Create directories if they don't exist
    fs::create_dir_all(storage_path)?;
    fs::create_dir_all(watch_path)?;
    
    let mut service = FileSharingService::new(storage_path, watch_path, database).await?;
    
    loop {
        println!("\n{}", "═══════════════════════════════════════".bright_blue());
        println!("{}", "MAIN MENU".bright_yellow().bold());
        println!("{}", "═══════════════════════════════════════".bright_blue());
        
        let options = vec![
            "1. Register New User",
            "2. Login",
            "3. Upload File",
            "4. List My Files",
            "5. Download File",
            "6. Share File",
            "7. List Shared Files",
            "8. Verify File Integrity",
            "9. System Statistics",
            "10. Exit",
        ];
        
        let selection = Select::new()
            .with_prompt("Select an option")
            .items(&options)
            .default(0)
            .interact()?;
        
        match selection {
            0 => register_user(&mut service).await?,
            1 => login_user(&mut service).await?,
            2 => upload_file(&mut service).await?,
            3 => list_my_files(&service).await?,
            4 => download_file(&service).await?,
            5 => share_file(&mut service).await?,
            6 => list_shared_files(&service).await?,
            7 => verify_file(&service).await?,
            8 => print_stats(&service).await?,
            9 => {
                println!("{}", "👋 Goodbye!".bright_green());
                break;
            }
            _ => continue,
        }
    }
    
    Ok(())
}

async fn register_user(service: &mut FileSharingService) -> Result<()> {
    println!("\n{}", "📝 REGISTER NEW USER".bright_magenta());
    
    let username: String = Input::new()
        .with_prompt("Enter username")
        .interact_text()?;
    
    // Check if user exists
    if service.database.get_user_by_username(&username).await?.is_some() {
        println!("{}", "❌ Username already exists!".bright_red());
        return Ok(());
    }
    
    let password: String = Password::new()
        .with_prompt("Enter password")
        .with_confirmation("Confirm password", "Passwords don't match")
        .interact()?;
    
    let email: String = Input::new()
        .with_prompt("Enter email (optional)")
        .allow_empty(true)
        .interact_text()?;
    
    let _user = service.register_user(&username, &password, 
        if email.is_empty() { None } else { Some(&email) }).await?;
    
    println!("{} User '{}' registered successfully!", "✅".bright_green(), username.bright_cyan());
    Ok(())
}

async fn login_user(service: &mut FileSharingService) -> Result<()> {
    println!("\n{}", "🔑 USER LOGIN".bright_magenta());
    
    let username: String = Input::new()
        .with_prompt("Enter username")
        .interact_text()?;
    
    let password: String = Password::new()
        .with_prompt("Enter password")
        .interact()?;
    
    match service.login(&username, &password).await? {
        Some(_user) => {
            println!("{} Welcome back, {}!", "✅".bright_green(), username.bright_cyan());
        }
        None => {
            println!("{} Invalid username or password!", "❌".bright_red());
        }
    }
    
    Ok(())
}

async fn upload_file(service: &mut FileSharingService) -> Result<()> {
    println!("\n{}", "📤 UPLOAD FILE".bright_magenta());
    
    if service.current_user.is_none() {
        println!("{} Please login first!", "❌".bright_red());
        return Ok(());
    }
    
    let file_path: String = Input::new()
        .with_prompt("Enter file path to upload")
        .interact_text()?;
    
    let path = Path::new(&file_path);
    if !path.exists() {
        println!("{} File not found!", "❌".bright_red());
        return Ok(());
    }
    
    let description: String = Input::new()
        .with_prompt("Enter file description (optional)")
        .allow_empty(true)
        .interact_text()?;
    
    let data = fs::read(path)?;
    let filename = path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    let username = service.current_user.as_ref().unwrap().username.clone();
    let metadata = service.upload_file(
        &data, 
        &filename, 
        &username,
        if description.is_empty() { None } else { Some(&description) }
    ).await?;
    
    println!("{} File uploaded successfully!", "✅".bright_green());
    println!("   Hash: {}", metadata.hash.to_hex().bright_cyan());
    println!("   Size: {} bytes", metadata.size.to_string().bright_yellow());
    println!("   Chunks: {}", metadata.chunks.len().to_string().bright_blue());
    
    Ok(())
}

async fn list_my_files(service: &FileSharingService) -> Result<()> {
    println!("\n{}", "📋 MY FILES".bright_magenta());
    
    if service.current_user.is_none() {
        println!("{} Please login first!", "❌".bright_red());
        return Ok(());
    }
    
    let files = service.get_user_files(
        service.current_user.as_ref().unwrap().username.as_str()
    ).await?;
    
    if files.is_empty() {
        println!("{} No files uploaded yet.", "📭".bright_yellow());
        return Ok(());
    }
    
    println!("\n{:<5} {:<30} {:<10} {:<20}", 
        "ID".bright_white(), 
        "Filename".bright_white(), 
        "Size".bright_white(), 
        "Uploaded".bright_white()
    );
    println!("{}", "─".repeat(70).bright_black());
    
    for (i, file) in files.iter().enumerate() {
        println!("{:<5} {:<30} {:<10} {:<20}", 
            (i+1).to_string().bright_blue(),
            file.filename.chars().take(28).collect::<String>(),
            format!("{}B", file.size).bright_yellow(),
            file.created_at.format("%Y-%m-%d").to_string().bright_green()
        );
    }
    
    Ok(())
}

async fn download_file(service: &FileSharingService) -> Result<()> {
    println!("\n{}", "📥 DOWNLOAD FILE".bright_magenta());
    
    if service.current_user.is_none() {
        println!("{} Please login first!", "❌".bright_red());
        return Ok(());
    }
    
    let files = service.get_user_files(
        service.current_user.as_ref().unwrap().username.as_str()
    ).await?;
    
    if files.is_empty() {
        println!("{} No files to download.", "📭".bright_yellow());
        return Ok(());
    }
    
    let filenames: Vec<String> = files.iter()
        .map(|f| format!("{} ({} bytes)", f.filename, f.size))
        .collect();
    
    let selection = Select::new()
        .with_prompt("Select file to download")
        .items(&filenames)
        .interact()?;
    
    let selected = &files[selection];
    
    let output_path: String = Input::new()
        .with_prompt("Enter output path")
        .default("./downloaded".to_string())
        .interact_text()?;
    
    // Convert hash string to HashValue
    let bytes = hex::decode(&selected.hash)?;
    let hash = HashValue {
        algo: HashAlgo::Sha256,
        bytes,
    };
    
    let data = service.download_and_verify(&hash).await?;
    let output_file = Path::new(&output_path).join(&selected.filename);
    fs::write(&output_file, data)?;
    
    println!("{} File downloaded to: {}", "✅".bright_green(), output_file.display().to_string().bright_cyan());
    
    Ok(())
}

async fn share_file(service: &mut FileSharingService) -> Result<()> {
    println!("\n{}", "🔗 SHARE FILE".bright_magenta());
    
    if service.current_user.is_none() {
        println!("{} Please login first!", "❌".bright_red());
        return Ok(());
    }
    
    // Clone username before using it to avoid borrow issues
    let current_username = service.current_user.as_ref().unwrap().username.clone();
    
    let files = service.get_user_files(&current_username).await?;
    
    if files.is_empty() {
        println!("{} No files to share.", "📭".bright_yellow());
        return Ok(());
    }
    
    let filenames: Vec<String> = files.iter()
        .map(|f| f.filename.clone())
        .collect();
    
    let selection = Select::new()
        .with_prompt("Select file to share")
        .items(&filenames)
        .interact()?;
    
    let selected = &files[selection];
    
    let target_username: String = Input::new()
        .with_prompt("Enter username to share with")
        .interact_text()?;
    
    // Convert hash string to HashValue
    let bytes = hex::decode(&selected.hash)?;
    let hash = HashValue {
        algo: HashAlgo::Sha256,
        bytes,
    };
    
    // Use the cloned username here
    service.share_file(
        &hash, 
        &current_username,
        &target_username
    ).await?;
    
    println!("{} File shared with {} successfully!", "✅".bright_green(), target_username.bright_cyan());
    
    Ok(())
}

async fn list_shared_files(service: &FileSharingService) -> Result<()> {
    println!("\n{}", "📋 SHARED WITH ME".bright_magenta());
    
    if service.current_user.is_none() {
        println!("{} Please login first!", "❌".bright_red());
        return Ok(());
    }
    
    let shares = service.get_shared_files(
        service.current_user.as_ref().unwrap().username.as_str()
    ).await?;
    
    if shares.is_empty() {
        println!("{} No files shared with you.", "📭".bright_yellow());
        return Ok(());
    }
    
    println!("\n{:<5} {:<25} {:<15} {:<20}", 
        "ID".bright_white(), 
        "Filename".bright_white(), 
        "Shared By".bright_white(), 
        "Shared At".bright_white()
    );
    println!("{}", "─".repeat(70).bright_black());
    
    for (i, share) in shares.iter().enumerate() {
        println!("{:<5} {:<25} {:<15} {:<20}", 
            (i+1).to_string().bright_blue(),
            share.filename.chars().take(23).collect::<String>(),
            share.shared_by.bright_green(),
            share.shared_at.format("%Y-%m-%d %H:%M").to_string().bright_cyan()
        );
    }
    
    Ok(())
}

async fn verify_file(service: &FileSharingService) -> Result<()> {
    println!("\n{}", "🔍 VERIFY FILE INTEGRITY".bright_magenta());
    
    if service.current_user.is_none() {
        println!("{} Please login first!", "❌".bright_red());
        return Ok(());
    }
    
    let file_hash: String = Input::new()
        .with_prompt("Enter file hash to verify")
        .interact_text()?;
    
    // Convert hex string to HashValue
    let bytes = hex::decode(&file_hash)?;
    let hash = HashValue {
        algo: HashAlgo::Sha256,
        bytes,
    };
    
    match service.verify_file_integrity(&hash).await? {
        true => println!("{} File integrity verified: OK", "✅".bright_green()),
        false => println!("{} File integrity check FAILED!", "❌".bright_red()),
    }
    
    Ok(())
}

async fn print_stats(service: &FileSharingService) -> Result<()> {
    println!("\n{}", "📊 SYSTEM STATISTICS".bright_magenta());
    
    let stats = service.get_system_stats().await?;
    
    println!("\n{}", "═══════════════════════════════════════".bright_blue());
    println!("{:<20}: {}", "Total Users".bright_white(), stats.total_users.to_string().bright_yellow());
    println!("{:<20}: {}", "Total Files".bright_white(), stats.total_files.to_string().bright_yellow());
    println!("{:<20}: {}", "Unique Files".bright_white(), stats.unique_files.to_string().bright_yellow());
    println!("{:<20}: {}", "Total Shares".bright_white(), stats.total_shares.to_string().bright_yellow());
    println!("{:<20}: {} GB", "Total Storage".bright_white(), format!("{:.2}", stats.total_bytes as f64 / 1_000_000_000.0).bright_green());
    println!("{:<20}: {} MB", "Saved Space".bright_white(), format!("{:.2}", stats.saved_bytes as f64 / 1_000_000.0).bright_green());
    println!("{:<20}: {:.1}%", "Deduplication Rate".bright_white(), format!("{:.1}", stats.dedup_rate).bright_blue());
    println!("{:<20}: {:.4}", "Bloom FP Rate".bright_white(), format!("{:.4}", stats.bloom_fp_rate).bright_magenta());
    println!("{}", "═══════════════════════════════════════".bright_blue());
    
    Ok(())
}