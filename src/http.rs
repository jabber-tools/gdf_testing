use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    iat: u64,
    exp: u64,
    aud: String
}


fn new_token(user_id: &str) -> Option<String> {

    let header: Header = Default::default();
    // see https://github.com/Keats/jsonwebtoken
    let _now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let claims = Claims {
        sub: user_id.into(),
        iat: _now,
        exp: _now + 3600,
        aud: "https://www.googleapis.com/oauth2/v4/token".to_owned(),
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret("secret".as_ref())).ok()

}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_token() {
        let token = new_token("foo@bar.com");
        match token {
            // TBD: implement token validation here
            Some(_token) => assert!(true, format!("token is {}", _token)),
            _ => assert!(false, "no token generated!")
        }
    }

}    