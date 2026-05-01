use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::model::user::{User, UserRole};
use crate::domain::repository::UserRepository;

pub struct Service {
    pool: PgPool,
}

impl Service {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn register_user(&self, email: &str, password: &str) -> Result<User, String> {
        let repo = UserRepository::new(self.pool.clone());
        
        if repo.find_by_email(email).await.map_err(|e| e.to_string())?.is_some() {
            return Err("User already exists".to_string());
        }
        
        let password_hash = bcrypt::hash(password, 10).map_err(|e| e.to_string())?;
        repo.create(email, &password_hash, &[UserRole::PageViewer]).await.map_err(|e| e.to_string())
    }

    pub async fn authenticate(&self, email: &str, password: &str, jwt_secret: &str) -> Result<String, String> {
        let repo = UserRepository::new(self.pool.clone());
        
        let user = repo.find_by_email(email)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Invalid credentials")?;
        
        bcrypt::verify(password, &user.password)
            .map_err(|e| e.to_string())?
            .then(|| ())
            .ok_or("Invalid credentials")?;
        
        use crate::infrastructure::auth::JwtAuth;
        let auth = JwtAuth::new(jwt_secret.to_string());
        auth.generate_token(user.id, user.email)
            .map_err(|e| e.to_string())
    }
}