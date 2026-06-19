use tonic::{Request, Response, Status};
use crate::{auth, config::AppConfig, db, middleware, store::Store};
use std::sync::Arc;

pub struct AuthServiceImpl<S: Store> {
    pub config: Arc<AppConfig>,
    pub store: S,
}

#[tonic::async_trait]
impl<S: Store> crate::generated::auth_service_server::AuthService for AuthServiceImpl<S> {
    async fn login(
        &self,
        request: Request<crate::generated::LoginRequest>,
    ) -> Result<Response<crate::generated::LoginResponse>, Status> {
        let req = request.into_inner();
        let ip = middleware::extract_ip(&Request::new(()));

        if req.username.is_empty() || req.password.is_empty() {
            return Err(Status::invalid_argument("Username and password required"));
        }
        if req.username.len() > 100 || req.password.len() > 128 {
            return Err(Status::invalid_argument("Input too long"));
        }

        let user = self
            .store
            .get_user_by_username(&req.username)
            .await
            .map_err(|_| Status::internal("Database error"))?
            .ok_or_else(|| {
                let store = self.store.clone();
                let ip = ip.clone();
                let username = req.username.clone();
                tokio::spawn(async move {
                    let _ = store.audit("", "LOGIN_FAILED", "auth", "", &format!("username={}", username), &ip).await;
                });
                Status::unauthenticated("Invalid credentials")
            })?;

        if !user.is_active {
            return Err(Status::unauthenticated("Account disabled"));
        }

        let valid = auth::verify_password(&req.password, &user.password_hash)
            .await
            .map_err(|_| Status::internal("Auth error"))?;

        if !valid {
            let store = self.store.clone();
            let ip = ip.clone();
            let uid = user.id.clone();
            tokio::spawn(async move {
                let _ = store.audit(&uid, "LOGIN_FAILED", "auth", "", "Wrong password", &ip).await;
            });
            return Err(Status::unauthenticated("Invalid credentials"));
        }

        let access_claims = auth::Claims::new_access(
            &user.id, &user.username, &user.role,
            self.config.jwt_access_ttl_secs,
        );
        let refresh_claims = auth::Claims::new_refresh(
            &user.id, &user.username, &user.role,
            self.config.jwt_refresh_ttl_secs,
        );

        let access_token = auth::generate_token(&access_claims, &self.config.jwt_secret)?;
        let refresh_token = auth::generate_token(&refresh_claims, &self.config.jwt_refresh_secret)?;

        let expires_at = chrono::NaiveDateTime::from_timestamp_opt(access_claims.exp, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        self.store
            .create_session(&user.id, &access_claims.jti, &expires_at, &ip)
            .await
            .map_err(|_| Status::internal("Session error"))?;

        let _ = self.store.audit(&user.id, "LOGIN_SUCCESS", "auth", "", "", &ip).await;

        Ok(Response::new(crate::generated::LoginResponse {
            access_token,
            refresh_token,
            expires_in: self.config.jwt_access_ttl_secs,
            token_type: "Bearer".to_string(),
        }))
    }

    async fn refresh_token(
        &self,
        request: Request<crate::generated::RefreshRequest>,
    ) -> Result<Response<crate::generated::LoginResponse>, Status> {
        let req = request.into_inner();
        let claims = auth::validate_token(&req.refresh_token, &self.config.jwt_refresh_secret)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;

        if claims.token_type != "refresh" {
            return Err(Status::unauthenticated("Invalid token type"));
        }

        let access_claims = auth::Claims::new_access(
            &claims.sub, &claims.username, &claims.role,
            self.config.jwt_access_ttl_secs,
        );
        let new_refresh = auth::Claims::new_refresh(
            &claims.sub, &claims.username, &claims.role,
            self.config.jwt_refresh_ttl_secs,
        );

        let access_token = auth::generate_token(&access_claims, &self.config.jwt_secret)?;
        let refresh_token = auth::generate_token(&new_refresh, &self.config.jwt_refresh_secret)?;

        Ok(Response::new(crate::generated::LoginResponse {
            access_token,
            refresh_token,
            expires_in: self.config.jwt_access_ttl_secs,
            token_type: "Bearer".to_string(),
        }))
    }
}
