use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;
use chrono::{Duration, Utc};

use crate::domain::model::user::JwtClaims;

pub struct JwtAuth {
    secret: String,
}

impl JwtAuth {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    pub fn generate_token(&self, user_id: Uuid, email: String) -> Result<String, jsonwebtoken::errors::Error> {
        let expiration = Utc::now() + Duration::hours(24);
        let claims = JwtClaims::new(email, user_id, expiration.timestamp());

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, jsonwebtoken::errors::Error> {
        let validation = Validation::default();
        
        decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
    }
}

impl Clone for JwtAuth {
    fn clone(&self) -> Self {
        Self {
            secret: self.secret.clone(),
        }
    }
}