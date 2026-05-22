use config::ConfigError;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct AppConfig {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub auth_base_url: String,
    pub auto_migrate_on_startup: bool,
    pub amqp_url: Option<String>,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        // Try to load .env from project root (where Cargo.toml is)
        let project_root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".env");

        // Try current directory first
        dotenv::dotenv().ok();

        // If .env doesn't exist in current dir, try project root
        if !project_root.exists() {
            let cargo_root = std::env::var("CARGO_MANIFEST_DIR")
                .map(|p| PathBuf::from(p).parent().unwrap().join(".env"))
                .ok();

            if let Some(path) = cargo_root {
                if path.exists() {
                    dotenv::from_path(&path).ok();
                }
            }
        }

        // Read values from environment directly (case-insensitive)
        let server_host = std::env::var("APP_SERVER_HOST")
            .or_else(|_| std::env::var("APP_server_host"))
            .or_else(|_| std::env::var("app_server_host"))
            .map_err(|_| ConfigError::Message("Missing APP_SERVER_HOST".to_string()))?;

        let server_port = std::env::var("APP_SERVER_PORT")
            .or_else(|_| std::env::var("APP_server_port"))
            .or_else(|_| std::env::var("app_server_port"))
            .map_err(|_| ConfigError::Message("Missing APP_SERVER_PORT".to_string()))?
            .parse::<u16>()
            .map_err(|_| ConfigError::Message("Invalid SERVER_PORT".to_string()))?;

        let database_url = std::env::var("APP_DATABASE_URL")
            .or_else(|_| std::env::var("APP_database_url"))
            .or_else(|_| std::env::var("app_database_url"))
            .map_err(|_| ConfigError::Message("Missing APP_DATABASE_URL".to_string()))?;

        let auth_base_url = std::env::var("APP_AUTH_BASE_URL")
            .or_else(|_| std::env::var("APP_auth_base_url"))
            .or_else(|_| std::env::var("app_auth_base_url"))
            .map_err(|_| ConfigError::Message("Missing APP_AUTH_BASE_URL".to_string()))?
            .trim()
            .to_string();
        if auth_base_url.is_empty() {
            // Fail-closed: an empty `auth_base_url` would cause downstream
            // modules (orders, wallet, catalog) to skip auth validation
            // silently. Reject at startup so the misconfiguration is loud.
            return Err(ConfigError::Message(
                "APP_AUTH_BASE_URL cannot be empty — set it to the auth-be base URL"
                    .to_string(),
            ));
        }

        let auto_migrate_on_startup = match std::env::var("APP_AUTO_MIGRATE_ON_STARTUP")
            .or_else(|_| std::env::var("APP_auto_migrate_on_startup"))
            .or_else(|_| std::env::var("app_auto_migrate_on_startup"))
        {
            Ok(raw) => raw.parse::<bool>().map_err(|_| {
                ConfigError::Message(
                    "Invalid APP_AUTO_MIGRATE_ON_STARTUP (expected true/false)".to_string(),
                )
            })?,
            // Default ON so `cargo run` against a fresh database brings up
            // the schema in one step. Production deployments should set
            // `APP_AUTO_MIGRATE_ON_STARTUP=false` and use the dedicated
            // `migrate` binary in a separate, controlled job.
            Err(_) => true,
        };

        let amqp_url = std::env::var("APP_AMQP_URL").ok();

        Ok(AppConfig {
            server_host,
            server_port,
            database_url,
            auth_base_url,
            auto_migrate_on_startup,
            amqp_url,
        })
    }
}
