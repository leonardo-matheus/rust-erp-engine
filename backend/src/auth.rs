use crate::{config::AppConfig, error::AppError};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,      // user_id
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,      // unique token id for revocation
    pub token_type: String, // "access" or "refresh"
}

impl Claims {
    pub fn new_access(user_id: &str, username: &str, role: &str, ttl_secs: i64) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
            exp: (now + Duration::seconds(ttl_secs)).timestamp(),
            iat: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: "access".to_string(),
        }
    }

    pub fn new_refresh(user_id: &str, username: &str, role: &str, ttl_secs: i64) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
            exp: (now + Duration::seconds(ttl_secs)).timestamp(),
            iat: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            token_type: "refresh".to_string(),
        }
    }
}

pub fn generate_token(claims: &Claims, secret: &[u8]) -> Result<String, AppError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| AppError::Auth(format!("Token generation failed: {}", e)))
}

pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims, AppError> {
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["exp", "sub", "iat", "jti"]);

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| AppError::Auth(format!("Invalid token: {}", e)))
}

pub async fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::Internal(format!("Password hashing failed: {}", e)))
}

pub async fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub async fn create_session(
    db: &SqlitePool,
    user_id: &str,
    jti: &str,
    expires_at: chrono::NaiveDateTime,
    ip_address: &str,
    user_agent: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO sessions (id, user_id, jti, expires_at, ip_address, user_agent, created_at)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'))"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(jti)
    .bind(expires_at)
    .bind(ip_address)
    .bind(user_agent)
    .execute(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn revoke_session(db: &SqlitePool, jti: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET revoked = 1 WHERE jti = ?")
        .bind(jti)
        .execute(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

pub async fn is_session_valid(db: &SqlitePool, jti: &str) -> Result<bool, AppError> {
    let result: Option<(bool,)> = sqlx::query_as(
        "SELECT NOT revoked FROM sessions WHERE jti = ? AND expires_at > datetime('now')"
    )
    .bind(jti)
    .fetch_optional(db)
    .await
    .map_err(AppError::Database)?;

    Ok(result.map(|(valid,)| valid).unwrap_or(false))
}
