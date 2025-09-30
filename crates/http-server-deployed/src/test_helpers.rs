use crate::authentication::{Auth, Claims};
use chrono::Duration;
use jsonwebtoken::{encode, EncodingKey, Header};

impl Auth {
    pub fn generate_jwt(&self, ttl: Duration) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims::new(self.issuer.clone(), ttl);
        let header = Header::default();
        let encoding_key = EncodingKey::from_secret(&self.secret);

        encode(&header, &claims, &encoding_key)
    }
}
