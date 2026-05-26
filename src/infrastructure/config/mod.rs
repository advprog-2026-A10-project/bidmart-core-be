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
    pub storage: StorageConfig,
    /// AMQP URL untuk RabbitMQ publisher (opsional).
    /// Jika tidak di-set, bidding module berjalan tanpa push events.
    /// Contoh: amqp://bidmart:bidmart123@localhost:5672/
    pub amqp_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct StorageConfig {
    pub provider: String,
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub force_path_style: bool,
    pub public_base_url: String,
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
        let server_host = required_env("APP_SERVER_HOST", None)?;

        let server_port = required_env("APP_SERVER_PORT", None)?
            .parse::<u16>()
            .map_err(|_| ConfigError::Message("Invalid SERVER_PORT".to_string()))?;

        let database_url = required_env("APP_DATABASE_URL", None)?;

        let auth_base_url = required_env("APP_AUTH_BASE_URL", None)?.trim().to_string();
        if auth_base_url.is_empty() {
            // Fail-closed: an empty `auth_base_url` would cause downstream
            // modules (orders, wallet, catalog) to skip auth validation
            // silently. Reject at startup so the misconfiguration is loud.
            return Err(ConfigError::Message(
                "APP_AUTH_BASE_URL cannot be empty — set it to the auth-be base URL".to_string(),
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

        let storage = StorageConfig {
            provider: read_env("APP_STORAGE_PROVIDER")
                .unwrap_or_else(|_| "minio".to_string())
                .trim()
                .to_string(),
            endpoint: required_env(
                "APP_STORAGE_ENDPOINT",
                Some("http://localhost:9000".to_string()),
            )?
            .trim()
            .trim_end_matches('/')
            .to_string(),
            bucket: required_env(
                "APP_STORAGE_BUCKET",
                Some("bidmart-listing-images".to_string()),
            )?
            .trim()
            .to_string(),
            region: required_env("APP_STORAGE_REGION", Some("us-east-1".to_string()))?
                .trim()
                .to_string(),
            access_key: required_env("APP_STORAGE_ACCESS_KEY", Some("minioadmin".to_string()))?
                .trim()
                .to_string(),
            secret_key: required_env("APP_STORAGE_SECRET_KEY", Some("minioadmin123".to_string()))?
                .trim()
                .to_string(),
            force_path_style: read_env("APP_STORAGE_FORCE_PATH_STYLE")
                .unwrap_or_else(|_| "true".to_string())
                .parse::<bool>()
                .map_err(|_| {
                    ConfigError::Message(
                        "Invalid APP_STORAGE_FORCE_PATH_STYLE (expected true/false)".to_string(),
                    )
                })?,
            public_base_url: required_env(
                "APP_STORAGE_PUBLIC_BASE_URL",
                Some("http://localhost:9000/bidmart-listing-images".to_string()),
            )?
            .trim()
            .trim_end_matches('/')
            .to_string(),
        };

        let amqp_url = read_env("APP_AMQP_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Ok(AppConfig {
            server_host,
            server_port,
            database_url,
            auth_base_url,
            auto_migrate_on_startup,
            storage,
            amqp_url,
        })
    }
}

fn read_env(key: &str) -> Result<String, std::env::VarError> {
    std::env::var(key)
        .or_else(|_| std::env::var(key.to_ascii_lowercase()))
        .or_else(|_| std::env::var(format!("APP_{}", &key[4..].to_ascii_lowercase())))
}

fn required_env(key: &str, default: Option<String>) -> Result<String, ConfigError> {
    match read_env(key) {
        Ok(value) => Ok(value),
        Err(_) => default.ok_or_else(|| ConfigError::Message(format!("Missing {key}"))),
    }
}
