use anyhow::Result;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub db_url: String,
    pub neo4j_url: String,
    pub neo4j_user: String,
    pub neo4j_pass: String,
}

impl Config {
    /// Load configuration from .env file and environment variables.
    pub fn load() -> Result<Self> {
        // Try to load .env file, but don't fail if it doesn't exist
        let _ = dotenvy::dotenv();

        let db_url = std::env::var("ROOTMAP_DB_URL")
            .unwrap_or_else(|_| "sqlite://rootmap.db".to_string());

        let neo4j_url = std::env::var("ROOTMAP_NEO4J_URL")
            .unwrap_or_else(|_| "127.0.0.1:7687".to_string());

        let neo4j_user = std::env::var("ROOTMAP_NEO4J_USER")
            .unwrap_or_else(|_| "neo4j".to_string());

        let neo4j_pass = std::env::var("ROOTMAP_NEO4J_PASS")
            .unwrap_or_else(|_| "rootmap123".to_string());

        Ok(Config {
            db_url,
            neo4j_url,
            neo4j_user,
            neo4j_pass,
        })
    }
}
