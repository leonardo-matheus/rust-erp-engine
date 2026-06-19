use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: Vec<u8>,
    pub jwt_refresh_secret: Vec<u8>,
    pub jwt_access_ttl_secs: i64,
    pub jwt_refresh_ttl_secs: i64,
    pub aes_key: Vec<u8>,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub grpc_port: u16,
    pub grpc_web_port: u16,
    pub rate_limit_rps: u32,
    pub log_level: String,
    pub audit_log_path: String,
    pub cors_origins: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:techfix.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| Self::generate_random_key(64))
                .into_bytes(),
            jwt_refresh_secret: env::var("JWT_REFRESH_SECRET")
                .unwrap_or_else(|_| Self::generate_random_key(64))
                .into_bytes(),
            jwt_access_ttl_secs: env::var("JWT_ACCESS_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(900), // 15 min
            jwt_refresh_ttl_secs: env::var("JWT_REFRESH_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(604800), // 7 days
            aes_key: env::var("AES_KEY")
                .unwrap_or_else(|_| Self::generate_random_key(32))
                .into_bytes(),
            tls_cert_path: env::var("TLS_CERT_PATH").ok(),
            tls_key_path: env::var("TLS_KEY_PATH").ok(),
            grpc_port: env::var("PORT")
                .or_else(|_| env::var("GRPC_PORT"))
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50051),
            grpc_web_port: env::var("GRPC_WEB_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8443),
            rate_limit_rps: env::var("RATE_LIMIT_RPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            audit_log_path: env::var("AUDIT_LOG_PATH")
                .unwrap_or_else(|_| "./audit.log".to_string()),
            cors_origins: env::var("CORS_ORIGINS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|_| vec!["https://localhost:8443".to_string()]),
        }
    }

    fn generate_random_key(len: usize) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..len).map(|_| rng.gen::<u8>()).map(|b| format!("{:02x}", b)).collect()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 bytes".into());
        }
        if self.aes_key.len() != 32 {
            return Err("AES_KEY must be exactly 32 bytes (256 bits)".into());
        }
        Ok(())
    }
}
