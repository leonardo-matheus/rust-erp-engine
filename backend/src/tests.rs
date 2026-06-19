#[cfg(test)]
mod tests {
    use crate::{auth, crypto, validation, store::Store};
    use validation::ValidateExt;

    // ── AUTH TESTS ──

    #[tokio::test]
    async fn test_password_hash_and_verify() {
        let hash = auth::hash_password("SecurePass123!").await.unwrap();
        assert!(!hash.is_empty());
        assert!(hash.starts_with("$argon2"));

        assert!(auth::verify_password("SecurePass123!", &hash).await.unwrap());
        assert!(!auth::verify_password("wrong_password", &hash).await.unwrap());
        assert!(!auth::verify_password("", &hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_password_hash_unique_per_call() {
        let hash1 = auth::hash_password("same_password").await.unwrap();
        let hash2 = auth::hash_password("same_password").await.unwrap();
        assert_ne!(hash1, hash2, "Each hash should use unique salt");
        assert!(auth::verify_password("same_password", &hash1).await.unwrap());
        assert!(auth::verify_password("same_password", &hash2).await.unwrap());
    }

    #[test]
    fn test_jwt_generate_and_validate() {
        let secret = b"test_secret_key_at_least_32_bytes_long!!";
        let claims = auth::Claims::new_access("user-123", "admin", "admin", 3600);
        let token = auth::generate_token(&claims, secret).unwrap();

        assert!(!token.is_empty());
        assert!(token.contains('.'));

        let validated = auth::validate_token(&token, secret).unwrap();
        assert_eq!(validated.sub, "user-123");
        assert_eq!(validated.username, "admin");
        assert_eq!(validated.role, "admin");
        assert_eq!(validated.token_type, "access");
    }

    #[test]
    fn test_jwt_rejects_wrong_secret() {
        let secret1 = b"secret_one_32_bytes_long_padding!!!";
        let secret2 = b"secret_two_32_bytes_long_padding!!!";
        let claims = auth::Claims::new_access("u1", "user", "user", 3600);
        let token = auth::generate_token(&claims, secret1).unwrap();

        assert!(auth::validate_token(&token, secret2).is_err());
    }

    #[test]
    fn test_jwt_rejects_expired_token() {
        let secret = b"test_secret_key_at_least_32_bytes_long!!";
        let mut claims = auth::Claims::new_access("u1", "user", "user", 3600);
        claims.exp = chrono::Utc::now().timestamp() - 100; // expired 100s ago
        let token = auth::generate_token(&claims, secret).unwrap();

        assert!(auth::validate_token(&token, secret).is_err());
    }

    #[test]
    fn test_jwt_refresh_token_type() {
        let secret = b"test_secret_key_at_least_32_bytes_long!!";
        let claims = auth::Claims::new_refresh("u1", "user", "user", 86400);
        let token = auth::generate_token(&claims, secret).unwrap();
        let validated = auth::validate_token(&token, secret).unwrap();
        assert_eq!(validated.token_type, "refresh");
    }

    #[test]
    fn test_jwt_tampered_token_rejected() {
        let secret = b"test_secret_key_at_least_32_bytes_long!!";
        let claims = auth::Claims::new_access("u1", "admin", "admin", 3600);
        let token = auth::generate_token(&claims, secret).unwrap();

        // Tamper with the payload
        let parts: Vec<&str> = token.split('.').collect();
        let tampered = format!("{}.{}_tampered.{}", parts[0], parts[1], parts[2]);
        assert!(auth::validate_token(&tampered, secret).is_err());
    }

    // ── CRYPTO TESTS ──

    #[test]
    fn test_aes_encrypt_decrypt_roundtrip() {
        let key: [u8; 32] = rand::random();
        let enc = crypto::Encryptor::new(&key).unwrap();

        let plaintext = "Sensitive payment data: 4111-1111-1111-1111";
        let ciphertext = enc.encrypt(plaintext).unwrap();

        assert_ne!(ciphertext, plaintext);
        assert!(ciphertext.len() > plaintext.len()); // nonce + tag + ciphertext

        let decrypted = enc.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes_encrypt_produces_unique_ciphertext() {
        let key: [u8; 32] = rand::random();
        let enc = crypto::Encryptor::new(&key).unwrap();

        let c1 = enc.encrypt("same data").unwrap();
        let c2 = enc.encrypt("same data").unwrap();
        assert_ne!(c1, c2, "Same plaintext should produce different ciphertext (random nonce)");
    }

    #[test]
    fn test_aes_decrypt_rejects_tampered_data() {
        let key: [u8; 32] = rand::random();
        let enc = crypto::Encryptor::new(&key).unwrap();

        let ciphertext = enc.encrypt("secret").unwrap();
        let mut tampered = ciphertext.clone();
        tampered.push('X');

        assert!(enc.decrypt(&tampered).is_err());
    }

    #[test]
    fn test_aes_decrypt_rejects_wrong_key() {
        let key1: [u8; 32] = rand::random();
        let key2: [u8; 32] = rand::random();
        let enc1 = crypto::Encryptor::new(&key1).unwrap();
        let enc2 = crypto::Encryptor::new(&key2).unwrap();

        let ciphertext = enc1.encrypt("secret").unwrap();
        assert!(enc2.decrypt(&ciphertext).is_err());
    }

    #[test]
    fn test_aes_rejects_invalid_key_length() {
        assert!(crypto::Encryptor::new(&[0u8; 16]).is_err());
        assert!(crypto::Encryptor::new(&[0u8; 64]).is_err());
        assert!(crypto::Encryptor::new(&[0u8; 32]).is_ok());
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(crypto::constant_time_eq(b"hello", b"hello"));
        assert!(!crypto::constant_time_eq(b"hello", b"world"));
        assert!(!crypto::constant_time_eq(b"hello", b"hell"));
        assert!(!crypto::constant_time_eq(b"", b"a"));
        assert!(crypto::constant_time_eq(b"", b""));
    }

    // ── VALIDATION TESTS ──

    #[test]
    fn test_create_product_valid() {
        let input = validation::CreateProductInput {
            name: "Widget".into(),
            sku: "WDG-001".into(),
            category: "Electronics".into(),
            price: 99.99,
            cost: 50.00,
            stock: 100,
            min_stock: 10,
        };
        assert!(input.validate_input().is_ok());
    }

    #[test]
    fn test_create_product_rejects_empty_name() {
        let input = validation::CreateProductInput {
            name: "".into(),
            sku: "WDG-001".into(),
            category: "Electronics".into(),
            price: 99.99,
            cost: 50.00,
            stock: 100,
            min_stock: 10,
        };
        assert!(input.validate_input().is_err());
    }

    #[test]
    fn test_create_product_rejects_invalid_sku() {
        let input = validation::CreateProductInput {
            name: "Widget".into(),
            sku: "WDG 001!".into(), // spaces and ! are invalid
            category: "Electronics".into(),
            price: 99.99,
            cost: 50.00,
            stock: 100,
            min_stock: 10,
        };
        assert!(input.validate_input().is_err());
    }

    #[test]
    fn test_create_product_accepts_valid_skus() {
        for sku in &["NB-001", "WDG_002", "ABC123", "X"] {
            let input = validation::CreateProductInput {
                name: "Test".into(),
                sku: sku.to_string(),
                category: "Cat".into(),
                price: 10.0,
                cost: 5.0,
                stock: 10,
                min_stock: 1,
            };
            assert!(input.validate_input().is_ok(), "SKU '{}' should be valid", sku);
        }
    }

    #[test]
    fn test_create_product_rejects_negative_price() {
        let input = validation::CreateProductInput {
            name: "Widget".into(),
            sku: "WDG-001".into(),
            category: "Electronics".into(),
            price: -10.0,
            cost: 50.00,
            stock: 100,
            min_stock: 10,
        };
        assert!(input.validate_input().is_err());
    }

    #[test]
    fn test_restock_valid() {
        let input = validation::RestockInput {
            product_id: uuid::Uuid::new_v4().to_string(),
            quantity: 50,
            notes: "Supplier delivery".into(),
        };
        assert!(input.validate_input().is_ok());
    }

    #[test]
    fn test_restock_rejects_zero_quantity() {
        let input = validation::RestockInput {
            product_id: uuid::Uuid::new_v4().to_string(),
            quantity: 0,
            notes: "".into(),
        };
        assert!(input.validate_input().is_err());
    }

    #[test]
    fn test_login_input_validation() {
        let valid = validation::LoginInput {
            username: "admin".into(),
            password: "securepass123".into(),
        };
        assert!(valid.validate_input().is_ok());

        let short_pass = validation::LoginInput {
            username: "admin".into(),
            password: "short".into(),
        };
        assert!(short_pass.validate_input().is_err());

        let empty_user = validation::LoginInput {
            username: "".into(),
            password: "securepass123".into(),
        };
        assert!(empty_user.validate_input().is_err());

        let invalid_user = validation::LoginInput {
            username: "admin; DROP TABLE--".into(),
            password: "securepass123".into(),
        };
        assert!(invalid_user.validate_input().is_err());
    }

    #[test]
    fn test_sanitize_strips_control_chars() {
        assert_eq!(validation::sanitize("hello\x00world"), "helloworld");
        assert_eq!(validation::sanitize("line1\nline2"), "line1\nline2");
        assert_eq!(validation::sanitize("tab\there"), "tab\there");
        assert_eq!(validation::sanitize("normal text"), "normal text");
    }

    #[test]
    fn test_sanitize_limits_length() {
        let long = "A".repeat(20000);
        let result = validation::sanitize(&long);
        assert!(result.len() <= 10000);
    }

    #[test]
    fn test_validate_uuid() {
        assert!(validation::validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validation::validate_uuid("not-a-uuid").is_err());
        assert!(validation::validate_uuid("").is_err());
    }

    // ── CONFIG TESTS ──

    #[test]
    fn test_config_validation() {
        let config = crate::config::AppConfig {
            database_url: "sqlite:test.db".into(),
            jwt_secret: vec![0u8; 64],
            jwt_refresh_secret: vec![0u8; 64],
            jwt_access_ttl_secs: 900,
            jwt_refresh_ttl_secs: 604800,
            aes_key: vec![0u8; 32],
            tls_cert_path: None,
            tls_key_path: None,
            grpc_port: 50051,
            grpc_web_port: 8443,
            rate_limit_rps: 100,
            log_level: "info".into(),
            audit_log_path: "./audit.log".into(),
            cors_origins: vec![],
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_rejects_short_jwt_secret() {
        let config = crate::config::AppConfig {
            database_url: "sqlite:test.db".into(),
            jwt_secret: vec![0u8; 16], // too short
            jwt_refresh_secret: vec![0u8; 64],
            jwt_access_ttl_secs: 900,
            jwt_refresh_ttl_secs: 604800,
            aes_key: vec![0u8; 32],
            tls_cert_path: None,
            tls_key_path: None,
            grpc_port: 50051,
            grpc_web_port: 8443,
            rate_limit_rps: 100,
            log_level: "info".into(),
            audit_log_path: "./audit.log".into(),
            cors_origins: vec![],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_rejects_wrong_aes_key_length() {
        let config = crate::config::AppConfig {
            database_url: "sqlite:test.db".into(),
            jwt_secret: vec![0u8; 64],
            jwt_refresh_secret: vec![0u8; 64],
            jwt_access_ttl_secs: 900,
            jwt_refresh_ttl_secs: 604800,
            aes_key: vec![0u8; 16], // wrong length
            tls_cert_path: None,
            tls_key_path: None,
            grpc_port: 50051,
            grpc_web_port: 8443,
            rate_limit_rps: 100,
            log_level: "info".into(),
            audit_log_path: "./audit.log".into(),
            cors_origins: vec![],
        };
        assert!(config.validate().is_err());
    }

    // ── SLED PERFORMANCE BENCHMARKS ──

    #[tokio::test]
    async fn bench_sled_write_products() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::sled_store::SledStore::open(dir.path().to_str().unwrap()).unwrap();

        let start = std::time::Instant::now();
        let now = chrono::Utc::now().naive_utc();
        for i in 0..1000 {
            let p = crate::db::Product {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("Product {}", i),
                sku: format!("SKU-{:04}", i),
                category: "Benchmark".into(),
                price: 99.99,
                cost: 50.00,
                stock: 100,
                min_stock: 10,
                created_at: now,
                updated_at: now,
            };
            store.create_product(&p).await.unwrap();
        }
        let elapsed = start.elapsed();
        let per_op = elapsed.as_micros() / 1000;
        println!("[Sled] Write 1000 products: {:?} ({} µs/op)", elapsed, per_op);
        assert!(elapsed.as_secs() < 5, "Sled writes should complete in <5s");
    }

    #[tokio::test]
    async fn bench_sled_read_products() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::sled_store::SledStore::open(dir.path().to_str().unwrap()).unwrap();
        let now = chrono::Utc::now().naive_utc();

        // Seed
        for i in 0..500 {
            let p = crate::db::Product {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("Product {}", i),
                sku: format!("SKU-{:04}", i),
                category: "Benchmark".into(),
                price: 99.99,
                cost: 50.00,
                stock: 100,
                min_stock: 10,
                created_at: now,
                updated_at: now,
            };
            store.create_product(&p).await.unwrap();
        }

        // Benchmark reads
        let (products, _) = store.list_products("", "", 0, 500).await.unwrap();
        let ids: Vec<String> = products.iter().map(|p| p.id.clone()).collect();

        let start = std::time::Instant::now();
        for id in &ids {
            let _ = store.get_product(id).await.unwrap();
        }
        let elapsed = start.elapsed();
        let per_op = elapsed.as_micros() / ids.len() as u128;
        println!("[Sled] Read 500 products by ID: {:?} ({} µs/op)", elapsed, per_op);
        assert!(elapsed.as_secs() < 2);
    }

    #[tokio::test]
    async fn bench_sled_list_products() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::sled_store::SledStore::open(dir.path().to_str().unwrap()).unwrap();
        let now = chrono::Utc::now().naive_utc();

        for i in 0..1000 {
            let p = crate::db::Product {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("Product {}", i),
                sku: format!("SKU-{:04}", i),
                category: if i % 2 == 0 { "Electronics" } else { "Accessories" }.into(),
                price: 99.99,
                cost: 50.00,
                stock: 100,
                min_stock: 10,
                created_at: now,
                updated_at: now,
            };
            store.create_product(&p).await.unwrap();
        }

        // Full scan
        let start = std::time::Instant::now();
        let (page, total) = store.list_products("", "", 0, 50).await.unwrap();
        let elapsed = start.elapsed();
        println!("[Sled] List products (full scan, page 1 of {}): {:?} ({} µs/op)", total, elapsed, elapsed.as_micros());
        assert_eq!(page.len(), 50);

        // Filtered scan
        let start = std::time::Instant::now();
        let (filtered, _) = store.list_products("", "Electronics", 0, 50).await.unwrap();
        let elapsed = start.elapsed();
        println!("[Sled] List products (filtered by category): {:?} ({} µs/op, {} results)", elapsed, elapsed.as_micros(), filtered.len());

        // Search
        let start = std::time::Instant::now();
        let (searched, _) = store.list_products("Product 42", "", 0, 50).await.unwrap();
        let elapsed = start.elapsed();
        println!("[Sled] List products (search 'Product 42'): {:?} ({} µs/op, {} results)", elapsed, elapsed.as_micros(), searched.len());
    }

    #[tokio::test]
    async fn bench_sled_dashboard_stats() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::sled_store::SledStore::open(dir.path().to_str().unwrap()).unwrap();
        let now = chrono::Utc::now().naive_utc();

        for i in 0..1000 {
            let p = crate::db::Product {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("Product {}", i),
                sku: format!("SKU-{:04}", i),
                category: "Cat".into(),
                price: 100.0,
                cost: 50.0,
                stock: 50,
                min_stock: 10,
                created_at: now,
                updated_at: now,
            };
            store.create_product(&p).await.unwrap();
        }

        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = store.dashboard_stats().await.unwrap();
        }
        let elapsed = start.elapsed();
        println!("[Sled] Dashboard stats x100: {:?} ({} µs/op)", elapsed, elapsed.as_micros() / 100);
    }

    #[tokio::test]
    async fn bench_sled_stock_update() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::sled_store::SledStore::open(dir.path().to_str().unwrap()).unwrap();
        let now = chrono::Utc::now().naive_utc();

        let p = crate::db::Product {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Benchmark Product".into(),
            sku: "BENCH-001".into(),
            category: "Test".into(),
            price: 100.0,
            cost: 50.0,
            stock: 10000,
            min_stock: 10,
            created_at: now,
            updated_at: now,
        };
        store.create_product(&p).await.unwrap();

        let start = std::time::Instant::now();
        for _ in 0..1000 {
            store.update_stock(&p.id, 1).await.unwrap();
        }
        let elapsed = start.elapsed();
        let per_op = elapsed.as_micros() / 1000;
        println!("[Sled] Stock update x1000: {:?} ({} µs/op)", elapsed, per_op);

        let updated = store.get_product(&p.id).await.unwrap().unwrap();
        assert_eq!(updated.stock, 11000);
    }

    #[tokio::test]
    async fn bench_sled_concurrent_reads() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::sled_store::SledStore::open(dir.path().to_str().unwrap()).unwrap();
        let now = chrono::Utc::now().naive_utc();

        let mut ids = Vec::new();
        for i in 0..100 {
            let p = crate::db::Product {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("Product {}", i),
                sku: format!("SKU-{:04}", i),
                category: "Cat".into(),
                price: 100.0,
                cost: 50.0,
                stock: 50,
                min_stock: 10,
                created_at: now,
                updated_at: now,
            };
            store.create_product(&p).await.unwrap();
            ids.push(p.id.clone());
        }

        let start = std::time::Instant::now();
        let mut handles = Vec::new();
        for id in &ids {
            let s = store.clone();
            let id = id.clone();
            handles.push(tokio::spawn(async move {
                let _ = s.get_product(&id).await.unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        let elapsed = start.elapsed();
        println!("[Sled] Concurrent reads x100 (tokio::spawn): {:?} ({} µs/op)", elapsed, elapsed.as_micros() / 100);
    }
}
