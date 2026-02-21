// test_sqlite.rs
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::path::Path;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing SQLite connection...");
    
    let db_path = Path::new("./data/test.db");
    println!("DB Path: {:?}", db_path);
    
    let database_url = format!("sqlite:{}", db_path.display().to_string().replace('\\', "/"));
    println!("Database URL: {}", database_url);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await;
    
    match pool {
        Ok(p) => {
            println!("✅ SQLite connected successfully!");
            
            // یه query تستی
            let result = sqlx::query("SELECT 1").fetch_one(&p).await;
            match result {
                Ok(_) => println!("✅ Query executed successfully!"),
                Err(e) => println!("❌ Query failed: {}", e),
            }
            Ok(())
        },
        Err(e) => {
            println!("❌ SQLite connection failed: {}", e);
            Err(e.into())
        }
    }
}