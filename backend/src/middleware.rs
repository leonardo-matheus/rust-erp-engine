use crate::store::Store;
use crate::error::AppError;
use tonic::{Request, Status};

#[derive(Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

#[derive(Clone)]
pub struct AuthInterceptor<S: Store> {
    pub store: S,
}

impl<S: Store> AuthInterceptor<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub async fn intercept<T>(&self, req: &Request<T>) -> Result<AuthContext, Status> {
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?;

        let claims = crate::auth::validate_token(
            token,
            std::env::var("JWT_SECRET")
                .unwrap_or_default()
                .as_bytes(),
        )
        .map_err(|e| Status::unauthenticated(e.to_string()))?;

        if claims.token_type != "access" {
            return Err(Status::unauthenticated("Invalid token type"));
        }

        let valid = self
            .store
            .is_session_valid(&claims.jti)
            .await
            .map_err(|_| Status::internal("Auth check failed"))?;

        if !valid {
            return Err(Status::unauthenticated("Session revoked or expired"));
        }

        Ok(AuthContext {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
        })
    }

    pub fn require_role(ctx: &AuthContext, role: &str) -> Result<(), Status> {
        if ctx.role != role && ctx.role != "admin" {
            Err(Status::permission_denied("Insufficient permissions"))
        } else {
            Ok(())
        }
    }
}

pub fn extract_ip<T>(req: &Request<T>) -> String {
    req.metadata()
        .get("x-forwarded-for")
        .or_else(|| req.metadata().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1")
        .split(',')
        .next()
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string()
}
