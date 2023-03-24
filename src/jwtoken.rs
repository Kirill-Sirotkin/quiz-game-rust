use chrono::{Days, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};

pub fn generate_token(
    id: &String,
    room_id: &String,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_days(Days::new(1))
        .expect("Timestamp invalid")
        .timestamp();

    let new_claims = Claims {
        id: id.clone(),
        roomId: room_id.clone(),
        exp: expiration as usize,
    };
    let token = encode(
        &Header::default(),
        &new_claims,
        &EncodingKey::from_secret("secret".as_ref()),
    );
    return token;
}

pub fn decode_token(token: &String) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    return decode::<Claims>(
        &token,
        &DecodingKey::from_secret("secret".as_ref()),
        &Validation::new(Algorithm::HS256),
    );
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub id: String,
    pub roomId: String,
    pub exp: usize,
}